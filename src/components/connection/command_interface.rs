// Copyright (c) 2026 Elektrobit Automotive GmbH
//
// This program and the accompanying materials are made available under the
// terms of the Apache License, Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0.
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! This module contains the [`CommandInterfaceConnection`] and [`GrpcConfig`] structs and the
//! [`CommandInterfaceState`] enum, implementing the [`Connection`] trait over the
//! [Ankaios](https://eclipse-ankaios.github.io/ankaios) command interface (a direct gRPC
//! connection to the Ankaios server), used to connect to Ankaios from outside a workload.

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep, timeout as tokio_timeout};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};

use crate::AnkaiosError;
use crate::ankaios_api::ank_base::Request as AnkaiosRequest;
use crate::ankaios_api::grpc_api::{
    CommanderHello, FromServer, ToServer, command_connection_client::CommandConnectionClient,
    from_server::FromServerEnum, to_server::ToServerEnum,
};
use crate::components::connection::{ANKAIOS_VERSION, Connection, SynchronizedSenderMap};
use crate::components::event_types::EventEntry;
use crate::components::log_types::LogResponse;
use crate::components::response::{Response, ResponseType};

/// Configuration for connecting to an [Ankaios](https://eclipse-ankaios.github.io/ankaios)
/// server over the command interface.
///
/// Constructed either insecure (plaintext, via [`GrpcConfig::insecure`]) or mTLS-secured (via
/// [`GrpcConfig::mtls`]) with the certificate material being mandatory context.
///
/// ## Examples
///
/// ```rust
/// # use ankaios_sdk::GrpcConfig;
/// // Insecure connection.
/// let config = GrpcConfig::insecure("http://127.0.0.1:25551");
///
/// // mTLS connection.
/// # let (ca_pem, crt_pem, key_pem) = (String::new(), String::new(), String::new());
/// let config = GrpcConfig::mtls("https://127.0.0.1:25551", ca_pem, crt_pem, key_pem);
/// ```
#[derive(Clone)]
pub struct GrpcConfig {
    pub(crate) server_url: String,
    pub(crate) insecure: bool,
    pub(crate) ca_pem: Option<String>,
    pub(crate) crt_pem: Option<String>,
    pub(crate) key_pem: Option<String>,
}

impl GrpcConfig {
    /// Creates a new insecure (plaintext) [`GrpcConfig`] for the given server URL.
    ///
    /// ## Arguments
    ///
    /// * `server_url` - The URL of the Ankaios server, e.g. `http://127.0.0.1:25551`.
    #[must_use]
    pub fn insecure(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            insecure: true,
            ca_pem: None,
            crt_pem: None,
            key_pem: None,
        }
    }

    /// Creates a new mTLS-secured [`GrpcConfig`] for the given server URL, using the given
    /// PEM-encoded certificate content.
    ///
    /// ## Arguments
    ///
    /// * `server_url` - The URL of the Ankaios server, e.g. `https://127.0.0.1:25551`;
    /// * `ca_pem` - The PEM-encoded CA certificate content;
    /// * `crt_pem` - The PEM-encoded client certificate content;
    /// * `key_pem` - The PEM-encoded client private key content.
    #[must_use]
    pub fn mtls(
        server_url: impl Into<String>,
        ca_pem: impl Into<String>,
        crt_pem: impl Into<String>,
        key_pem: impl Into<String>,
    ) -> Self {
        Self {
            server_url: server_url.into(),
            insecure: false,
            ca_pem: Some(ca_pem.into()),
            crt_pem: Some(crt_pem.into()),
            key_pem: Some(key_pem.into()),
        }
    }
}

impl std::fmt::Debug for GrpcConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcConfig")
            .field("server_url", &self.server_url)
            .field("insecure", &self.insecure)
            .field("ca_pem", &self.ca_pem.as_ref().map(|_| "<redacted>"))
            .field("crt_pem", &self.crt_pem.as_ref().map(|_| "<redacted>"))
            .field("key_pem", &self.key_pem.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

/// Enum representing the state of the [`CommandInterfaceConnection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum CommandInterfaceState {
    /// The connection was initialized but not yet accepted by the server.
    Initialized = 1,
    /// The connection is established.
    Connected = 2,
    /// The connection is terminated.
    Terminated = 3,
    /// A previously established connection was lost and is being retried.
    Reconnecting = 4,
}

/// How long to wait between reconnect attempts once a previously established connection is
/// lost.
const RECONNECT_INTERVAL_MILLIS: u64 = 500;

/// This struct handles the interaction with an [Ankaios](https://eclipse-ankaios.github.io/ankaios)
/// server over the command interface (a gRPC connection). Once a connection has been
/// successfully established, losing it is treated as transient: the connection is rebuilt and
/// retried every [`RECONNECT_INTERVAL_MILLIS`] until it succeeds.
pub struct CommandInterfaceConnection {
    /// Configuration used to connect to the Ankaios server.
    config: GrpcConfig,
    /// State of the command interface connection.
    state: Arc<Mutex<CommandInterfaceState>>,
    /// Sender for the outgoing message stream fed into the command interface bidi call. Shared with the
    /// reader/reconnect task, which replaces it whenever the connection is rebuilt.
    writer_ch_sender: Arc<Mutex<Option<mpsc::Sender<ToServer>>>>,
    /// Handler for the task reading incoming messages from the command interface bidi stream (and
    /// transparently reconnecting on a lost connection).
    reader_task_handle: Option<JoinHandle<()>>,
    /// Sender for the response channel.
    response_sender: mpsc::Sender<Response>,
    /// Request ID to logs sender mapping.
    log_senders_map: SynchronizedSenderMap<LogResponse>,
    /// Request ID to events sender mapping.
    events_senders_map: SynchronizedSenderMap<EventEntry>,
}

/// What the reader task needs from a connection: read the next message, or (re)connect. Lets
/// tests mock the connection instead of needing a real server.
#[cfg_attr(test, automock)]
#[async_trait]
trait ReaderTransport: Send {
    async fn next_message(&mut self) -> Result<Option<FromServer>, tonic::Status>;

    /// Used for both the initial connection and the reconnect logic.
    async fn attempt_connect(&mut self) -> Result<mpsc::Sender<ToServer>, AnkaiosError>;
}

/// The real [`ReaderTransport`]. `streaming` is `None` until the first `attempt_connect` call.
struct GrpcTransport {
    config: GrpcConfig,
    streaming: Option<tonic::Streaming<FromServer>>,
}

#[async_trait]
impl ReaderTransport for GrpcTransport {
    async fn next_message(&mut self) -> Result<Option<FromServer>, tonic::Status> {
        self.streaming
            .as_mut()
            .expect("attempt_connect must succeed before next_message is called")
            .message()
            .await
    }

    async fn attempt_connect(&mut self) -> Result<mpsc::Sender<ToServer>, AnkaiosError> {
        let (sender, streaming) = CommandInterfaceConnection::open_stream(&self.config).await?;
        self.streaming = Some(streaming);
        Ok(sender)
    }
}

/// Reads continuously from `transport`, reconnecting every [`RECONNECT_INTERVAL_MILLIS`] if the
/// connection is lost, until `state` becomes [`CommandInterfaceState::Terminated`]. A free function
/// so tests can call it directly instead of through [`tokio::spawn`].
async fn run_reader_loop<T: ReaderTransport>(
    mut transport: T,
    state: Arc<Mutex<CommandInterfaceState>>,
    writer_ch_sender: Arc<Mutex<Option<mpsc::Sender<ToServer>>>>,
    response_sender: mpsc::Sender<Response>,
    mut logs_sender_map: SynchronizedSenderMap<LogResponse>,
    mut events_sender_map: SynchronizedSenderMap<EventEntry>,
) {
    'connection: loop {
        loop {
            match transport.next_message().await {
                Ok(Some(from_server)) => {
                    CommandInterfaceConnection::handle_decoded_response(
                        from_server,
                        &response_sender,
                        &mut logs_sender_map,
                        &mut events_sender_map,
                    )
                    .await;
                }
                Ok(None) => {
                    log::warn!("The connection to the Ankaios server was closed.");
                    break;
                }
                Err(status) => {
                    log::error!("Error while reading from the connection: '{status}'");
                    break;
                }
            }
        }

        {
            let mut state_guard = state.lock().unwrap_or_else(|_| unreachable!());
            if *state_guard == CommandInterfaceState::Terminated {
                // disconnect() was called; don't try to reconnect.
                break 'connection;
            }
            *state_guard = CommandInterfaceState::Reconnecting;
        }
        log::warn!(
            "Lost connection to the Ankaios server, attempting to reconnect every {RECONNECT_INTERVAL_MILLIS}ms..."
        );

        loop {
            sleep(Duration::from_millis(RECONNECT_INTERVAL_MILLIS)).await;
            match transport.attempt_connect().await {
                Ok(sender) => {
                    let mut state_guard = state.lock().unwrap_or_else(|_| unreachable!());
                    if *state_guard == CommandInterfaceState::Terminated {
                        break 'connection; // disconnect() won; drop the new sender/stream.
                    }
                    *writer_ch_sender.lock().unwrap_or_else(|_| unreachable!()) = Some(sender);
                    *state_guard = CommandInterfaceState::Connected;
                    log::info!("Reconnected to the Ankaios server.");
                    break;
                }
                Err(err) => {
                    log::debug!("Reconnect attempt failed: '{err}'");
                }
            }
        }
    }
}

impl CommandInterfaceConnection {
    /// Creates a new instance of the command interface connection.
    ///
    /// ## Arguments
    ///
    /// * `config` - The [`GrpcConfig`] to use when connecting;
    /// * `response_sender` - A sender for the response channel.
    ///
    /// ## Returns
    ///
    /// A new [`CommandInterfaceConnection`] instance.
    pub fn new(config: GrpcConfig, response_sender: mpsc::Sender<Response>) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(CommandInterfaceState::Terminated)),
            writer_ch_sender: Arc::new(Mutex::new(None)),
            reader_task_handle: None,
            response_sender,
            log_senders_map: SynchronizedSenderMap::default(),
            events_senders_map: SynchronizedSenderMap::default(),
        }
    }

    /// Builds the (optionally mTLS-secured) [`Channel`] used to reach the Ankaios server.
    async fn build_channel(config: &GrpcConfig) -> Result<Channel, AnkaiosError> {
        let plain_endpoint = Channel::from_shared(config.server_url.clone()).map_err(|err| {
            AnkaiosError::ConnectionError(format!(
                "Invalid gRPC server URL '{}': '{err}'",
                config.server_url
            ))
        })?;

        let endpoint = if config.insecure {
            plain_endpoint
        } else {
            let ca = Certificate::from_pem(config.ca_pem.as_deref().unwrap_or_default());
            let identity = Identity::from_pem(
                config.crt_pem.as_deref().unwrap_or_default(),
                config.key_pem.as_deref().unwrap_or_default(),
            );
            // Ankaios server certificates are always issued for this domain name, regardless
            // of the actual connection address.
            let tls = ClientTlsConfig::new()
                .domain_name("ank-server")
                .ca_certificate(ca)
                .identity(identity);

            plain_endpoint.tls_config(tls).map_err(|err| {
                AnkaiosError::ConnectionError(format!("Invalid TLS configuration: '{err}'"))
            })?
        };

        endpoint.connect().await.map_err(|err| {
            AnkaiosError::ConnectionError(format!(
                "Could not connect to the Ankaios server: '{err}'"
            ))
        })
    }

    /// Builds the gRPC channel, sends the initial [`CommanderHello`] and opens the
    /// `ConnectCommand` bidi stream.
    async fn open_stream(
        config: &GrpcConfig,
    ) -> Result<(mpsc::Sender<ToServer>, tonic::Streaming<FromServer>), AnkaiosError> {
        let channel = Self::build_channel(config).await?;
        let mut client = CommandConnectionClient::new(channel);

        let (grpc_tx, grpc_rx) = mpsc::channel::<ToServer>(5);
        grpc_tx
            .send(ToServer {
                to_server_enum: Some(ToServerEnum::CommanderHello(CommanderHello {
                    protocol_version: ANKAIOS_VERSION.to_owned(),
                })),
            })
            .await
            .map_err(|err| {
                AnkaiosError::ConnectionError(format!(
                    "Error while sending initial hello message: '{err}'"
                ))
            })?;

        let streaming = client
            .connect_command(ReceiverStream::new(grpc_rx))
            .await
            .map_err(|status| {
                AnkaiosError::ConnectionError(format!(
                    "Could not connect to the Ankaios server: '{status}'"
                ))
            })?
            .into_inner();

        Ok((grpc_tx, streaming))
    }

    /// Establishes the gRPC channel, sends the initial [`CommanderHello`], opens the
    /// `ConnectCommand` bidi stream and spawns the task reading from it.
    async fn connect_internal(&mut self) -> Result<(), AnkaiosError> {
        let mut transport = GrpcTransport {
            config: self.config.clone(),
            streaming: None,
        };
        let sender = transport.attempt_connect().await?;
        *self
            .writer_ch_sender
            .lock()
            .unwrap_or_else(|_| unreachable!()) = Some(sender);
        self.spawn_reader_task(transport);
        Ok(())
    }

    /// Spawns [`run_reader_loop`] as a background task.
    fn spawn_reader_task<T: ReaderTransport + 'static>(&mut self, transport: T) {
        self.reader_task_handle = Some(tokio::spawn(run_reader_loop(
            transport,
            Arc::clone(&self.state),
            Arc::clone(&self.writer_ch_sender),
            self.response_sender.clone(),
            self.log_senders_map.clone(),
            self.events_senders_map.clone(),
        )));
    }

    #[doc(hidden)]
    /// Handles a decoded [`FromServer`] message and dispatches to the appropriate action.
    ///
    /// ## Arguments
    ///
    /// * `from_server` - A decoded [`FromServer`] message from the Ankaios server;
    /// * `response_sender` - A [`mpsc::Sender<Response>`] to forward generic responses;
    /// * `logs_sender_map` - A [`SynchronizedSenderMap<LogResponse>`] to forward log entries and stop responses for a log campaign;
    /// * `events_sender_map` - A [`SynchronizedSenderMap<EventEntry>`] to forward events for an event campaign.
    async fn handle_decoded_response(
        from_server: FromServer,
        response_sender: &mpsc::Sender<Response>,
        logs_sender_map: &mut SynchronizedSenderMap<LogResponse>,
        events_sender_map: &mut SynchronizedSenderMap<EventEntry>,
    ) {
        match from_server.from_server_enum {
            Some(FromServerEnum::Response(response)) => {
                let received_response = Response::from(response);
                match received_response.content {
                    ResponseType::LogEntriesResponse(log_entries) => {
                        super::forward_log_entries(
                            received_response.id,
                            log_entries,
                            logs_sender_map,
                        )
                        .await;
                    }
                    ResponseType::LogsStopResponse(instance_name) => {
                        super::forward_logs_stop_response(
                            received_response.id,
                            instance_name,
                            logs_sender_map,
                        )
                        .await;
                    }
                    ResponseType::EventResponse(event_entry) => {
                        super::forward_event_response(
                            received_response.id,
                            event_entry,
                            events_sender_map,
                        )
                        .await;
                    }
                    _ => {
                        response_sender
                            .send(received_response)
                            .await
                            .unwrap_or_else(|err| {
                                log::error!("Error while sending response: '{err}'");
                            });
                    }
                }
            }
            Some(FromServerEnum::ServerHello(_)) => {
                log::trace!("Received server hello.");
            }
            // The remaining variants (UpdateWorkload, UpdateWorkloadState, LogsRequest,
            // LogsCancelRequest) are targeted at a specific agent by name and not expected on a
            // `CommandConnection`.
            Some(other) => {
                log::warn!("Received unexpected message from the Ankaios server: '{other:?}'");
            }
            None => {
                log::warn!("Received an empty message from the Ankaios server.");
            }
        }
    }
}

#[async_trait]
impl Connection for CommandInterfaceConnection {
    async fn connect(&mut self, timeout: Duration) -> Result<(), AnkaiosError> {
        {
            let mut state_guard = self.state.lock().unwrap_or_else(|_| unreachable!());
            if matches!(
                *state_guard,
                CommandInterfaceState::Initialized
                    | CommandInterfaceState::Connected
                    | CommandInterfaceState::Reconnecting
            ) {
                return Err(AnkaiosError::ConnectionError(
                    "Already connected.".to_owned(),
                ));
            }
            *state_guard = CommandInterfaceState::Initialized;
        }
        match tokio_timeout(timeout, self.connect_internal()).await {
            Ok(Ok(())) => {
                *self.state.lock().unwrap_or_else(|_| unreachable!()) =
                    CommandInterfaceState::Connected;
                log::trace!("Connected to the Ankaios server over the command interface.");
                Ok(())
            }
            Ok(Err(err)) => Err(err),
            Err(_) => Err(AnkaiosError::ConnectionError(
                "Connection to the Ankaios server timed out.".to_owned(),
            )),
        }
    }

    fn disconnect(&mut self) -> Result<(), AnkaiosError> {
        {
            let mut state_guard = self.state.lock().unwrap_or_else(|_| unreachable!());
            if *state_guard == CommandInterfaceState::Terminated {
                return Err(AnkaiosError::ConnectionError(
                    "Already disconnected.".to_owned(),
                ));
            }
            *state_guard = CommandInterfaceState::Terminated;
        }

        if let Some(handler) = self.reader_task_handle.take() {
            handler.abort();
        }
        *self
            .writer_ch_sender
            .lock()
            .unwrap_or_else(|_| unreachable!()) = None;
        Ok(())
    }

    async fn write_request(&mut self, request: AnkaiosRequest) -> Result<(), AnkaiosError> {
        if *self.state.lock().unwrap_or_else(|_| unreachable!()) != CommandInterfaceState::Connected
        {
            log::error!("Could not write to the command interface, not connected.");
            return Err(AnkaiosError::ConnectionError(
                "Could not write to the command interface, not connected.".to_owned(),
            ));
        }
        let maybe_sender = self
            .writer_ch_sender
            .lock()
            .unwrap_or_else(|_| unreachable!())
            .clone();
        if let Some(sender) = maybe_sender {
            sender
                .send(ToServer {
                    to_server_enum: Some(ToServerEnum::Request(request)),
                })
                .await
                .unwrap_or_else(|err| {
                    log::error!("Error while sending request: '{err}'");
                });
        }
        Ok(())
    }

    fn add_log_campaign(&mut self, request_id: String, logs_sender: mpsc::Sender<LogResponse>) {
        log::trace!("Add log campaign with request id: '{request_id}'");
        self.log_senders_map.insert(request_id, logs_sender);
    }

    fn remove_log_campaign(&mut self, request_id: &str) {
        if self.log_senders_map.remove(request_id).is_some() {
            log::trace!("Removed log campaign with request id: '{request_id}'");
        }
    }

    fn add_events_campaign(&mut self, request_id: String, events_sender: mpsc::Sender<EventEntry>) {
        log::trace!("Add event campaign with request id: '{request_id}'");
        self.events_senders_map.insert(request_id, events_sender);
    }

    fn remove_events_campaign(&mut self, request_id: &str) {
        if self.events_senders_map.remove(request_id).is_some() {
            log::trace!("Removed events campaign with request id: '{request_id}'");
        }
    }
}

//////////////////////////////////////////////////////////////////////////////
//                 ########  #######    #########  #########                //
//                    ##     ##        ##             ##                    //
//                    ##     #####     #########      ##                    //
//                    ##     ##                ##     ##                    //
//                    ##     #######   #########      ##                    //
//////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tokio::sync::mpsc;
    use tokio::time::{Duration, sleep};

    use super::{
        CommandInterfaceConnection, CommandInterfaceState, Connection, FromServer, FromServerEnum,
        GrpcConfig, MockReaderTransport, SynchronizedSenderMap, ToServer, ToServerEnum,
        run_reader_loop,
    };
    use crate::ankaios_api::ank_base::{self, response::ResponseContent as AnkaiosResponseContent};
    use crate::components::request::{Request, generate_test_request};
    use crate::components::response::{
        generate_test_ank_base_update_state_success, generate_test_proto_log_entries_response,
    };
    use crate::{AnkaiosError, EventEntry, LogResponse, Response};

    const SERVER_URL: &str = "http://127.0.0.1:25551";
    const REQUEST_ID: &str = "request_id_1";
    const CHANNEL_SIZE: usize = 10;

    fn generate_test_command_interface_connection() -> (CommandInterfaceConnection, mpsc::Receiver<Response>) {
        let (response_sender, response_receiver) = mpsc::channel::<Response>(CHANNEL_SIZE);
        (
            CommandInterfaceConnection::new(GrpcConfig::insecure(SERVER_URL), response_sender),
            response_receiver,
        )
    }

    #[test]
    fn utest_grpc_config_without_tls() {
        let config = GrpcConfig::insecure(SERVER_URL);

        assert_eq!(config.server_url, SERVER_URL);
        assert!(config.insecure);
        assert!(config.ca_pem.is_none());
        assert!(config.crt_pem.is_none());
        assert!(config.key_pem.is_none());

        assert_eq!(
            format!("{config:?}"),
            "GrpcConfig { server_url: \"http://127.0.0.1:25551\", insecure: true, ca_pem: None, crt_pem: None, key_pem: None }"
        );
    }

    #[test]
    fn utest_grpc_config_with_tls() {
        let config = GrpcConfig::mtls(SERVER_URL, "ca-secret", "crt-secret", "key-secret");

        assert_eq!(config.server_url, SERVER_URL);
        assert!(!config.insecure);
        assert_eq!(config.ca_pem.as_deref(), Some("ca-secret"));
        assert_eq!(config.crt_pem.as_deref(), Some("crt-secret"));
        assert_eq!(config.key_pem.as_deref(), Some("key-secret"));

        assert_eq!(
            format!("{config:?}"),
            "GrpcConfig { server_url: \"http://127.0.0.1:25551\", insecure: false, ca_pem: Some(\"<redacted>\"), crt_pem: Some(\"<redacted>\"), key_pem: Some(\"<redacted>\") }"
        );
    }

    #[tokio::test]
    async fn utest_command_interface_new_starts_terminated() {
        let (connection, _response_receiver) = generate_test_command_interface_connection();
        assert_eq!(
            *connection.state.lock().unwrap(),
            CommandInterfaceState::Terminated
        );
    }

    #[tokio::test]
    async fn utest_command_interface_connect_fails_on_unreachable_server() {
        let (response_sender, _response_receiver) = mpsc::channel::<Response>(CHANNEL_SIZE);
        // Port 0 is never a valid connection target, so this fails fast without needing a
        // real (or even reachable) Ankaios server.
        let mut connection = CommandInterfaceConnection::new(
            GrpcConfig::insecure("http://127.0.0.1:0"),
            response_sender,
        );

        let result = connection.connect(Duration::from_secs(5)).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::ConnectionError(_))));
        assert_eq!(
            *connection.state.lock().unwrap(),
            CommandInterfaceState::Initialized
        );
    }

    #[tokio::test]
    async fn utest_command_interface_disconnect_without_connect_fails() {
        let (mut connection, _response_receiver) = generate_test_command_interface_connection();
        let result = connection.disconnect();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::ConnectionError(_))));
    }

    #[tokio::test]
    async fn utest_command_interface_write_request_fails_when_not_connected() {
        let (mut connection, _response_receiver) = generate_test_command_interface_connection();
        let result = connection
            .write_request(crate::ankaios_api::ank_base::Request::default())
            .await;
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::ConnectionError(_))));
    }

    #[tokio::test]
    async fn utest_command_interface_write_request_succeeds_when_connected() {
        let (mut connection, _response_receiver) = generate_test_command_interface_connection();
        let (grpc_tx, mut grpc_rx) = mpsc::channel::<ToServer>(CHANNEL_SIZE);
        *connection.writer_ch_sender.lock().unwrap() = Some(grpc_tx);
        *connection.state.lock().unwrap() = CommandInterfaceState::Connected;

        let request = generate_test_request().to_proto();
        let result = connection.write_request(request.clone()).await;
        assert!(result.is_ok());

        let sent = grpc_rx
            .try_recv()
            .expect("request should have been written");
        assert_eq!(sent.to_server_enum, Some(ToServerEnum::Request(request)));
    }

    #[tokio::test]
    async fn utest_command_interface_log_campaign() {
        let (mut connection, _response_receiver) = generate_test_command_interface_connection();
        let (logs_sender, _logs_receiver) = mpsc::channel(CHANNEL_SIZE);

        connection.add_log_campaign(REQUEST_ID.to_owned(), logs_sender);
        assert!(
            connection
                .log_senders_map
                .senders_map
                .lock()
                .unwrap()
                .contains_key(REQUEST_ID)
        );

        connection.remove_log_campaign(REQUEST_ID);
        assert!(
            !connection
                .log_senders_map
                .senders_map
                .lock()
                .unwrap()
                .contains_key(REQUEST_ID)
        );
    }

    #[tokio::test]
    async fn utest_command_interface_events_campaign() {
        let (mut connection, _response_receiver) = generate_test_command_interface_connection();
        let (events_sender, _events_receiver) = mpsc::channel(CHANNEL_SIZE);

        connection.add_events_campaign(REQUEST_ID.to_owned(), events_sender);
        assert!(
            connection
                .events_senders_map
                .senders_map
                .lock()
                .unwrap()
                .contains_key(REQUEST_ID)
        );

        connection.remove_events_campaign(REQUEST_ID);
        assert!(
            !connection
                .events_senders_map
                .senders_map
                .lock()
                .unwrap()
                .contains_key(REQUEST_ID)
        );
    }

    #[tokio::test]
    async fn utest_command_interface_handle_decoded_response_forwards_generic_response() {
        let (connection, mut response_receiver) = generate_test_command_interface_connection();
        let mut log_senders_map = connection.log_senders_map.clone();
        let mut events_senders_map = connection.events_senders_map.clone();
        let ank_base_response = generate_test_ank_base_update_state_success(REQUEST_ID.to_owned());

        CommandInterfaceConnection::handle_decoded_response(
            FromServer {
                from_server_enum: Some(FromServerEnum::Response(ank_base_response)),
            },
            &connection.response_sender,
            &mut log_senders_map,
            &mut events_senders_map,
        )
        .await;

        let result = tokio::time::timeout(Duration::from_millis(100), response_receiver.recv())
            .await
            .expect("response should have been forwarded")
            .expect("channel should not be closed");
        assert_eq!(result.get_request_id(), REQUEST_ID);
    }

    #[tokio::test]
    async fn utest_command_interface_handle_decoded_response_forwards_log_entries() {
        let (connection, _response_receiver) = generate_test_command_interface_connection();
        let mut log_senders_map = connection.log_senders_map.clone();
        let mut events_senders_map = connection.events_senders_map.clone();
        let (logs_sender, mut logs_receiver) = mpsc::channel::<LogResponse>(CHANNEL_SIZE);
        log_senders_map.insert(REQUEST_ID.to_owned(), logs_sender);

        let ank_base_response = ank_base::Response {
            request_id: REQUEST_ID.to_owned(),
            response_content: Some(AnkaiosResponseContent::LogEntriesResponse(
                generate_test_proto_log_entries_response(),
            )),
        };

        CommandInterfaceConnection::handle_decoded_response(
            FromServer {
                from_server_enum: Some(FromServerEnum::Response(ank_base_response)),
            },
            &connection.response_sender,
            &mut log_senders_map,
            &mut events_senders_map,
        )
        .await;

        let result = tokio::time::timeout(Duration::from_millis(100), logs_receiver.recv())
            .await
            .expect("log entries should have been forwarded");
        assert!(matches!(result, Some(LogResponse::LogEntries(_))));
    }

    #[tokio::test]
    async fn utest_command_interface_handle_decoded_response_forwards_logs_stop_response() {
        let (connection, _response_receiver) = generate_test_command_interface_connection();
        let mut log_senders_map = connection.log_senders_map.clone();
        let mut events_senders_map = connection.events_senders_map.clone();
        let (logs_sender, mut logs_receiver) = mpsc::channel::<LogResponse>(CHANNEL_SIZE);
        log_senders_map.insert(REQUEST_ID.to_owned(), logs_sender);

        let ank_base_response = ank_base::Response {
            request_id: REQUEST_ID.to_owned(),
            response_content: Some(AnkaiosResponseContent::LogsStopResponse(
                ank_base::LogsStopResponse {
                    workload_name: Some(ank_base::WorkloadInstanceName {
                        agent_name: "agent_A".to_owned(),
                        workload_name: "workload_A".to_owned(),
                        id: "id_a".to_owned(),
                    }),
                },
            )),
        };

        CommandInterfaceConnection::handle_decoded_response(
            FromServer {
                from_server_enum: Some(FromServerEnum::Response(ank_base_response)),
            },
            &connection.response_sender,
            &mut log_senders_map,
            &mut events_senders_map,
        )
        .await;

        let result = tokio::time::timeout(Duration::from_millis(100), logs_receiver.recv())
            .await
            .expect("logs stop response should have been forwarded");
        assert!(matches!(result, Some(LogResponse::LogsStopResponse(_))));
    }

    #[tokio::test]
    async fn utest_command_interface_handle_decoded_response_forwards_event_response() {
        let (connection, _response_receiver) = generate_test_command_interface_connection();
        let mut log_senders_map = connection.log_senders_map.clone();
        let mut events_senders_map = connection.events_senders_map.clone();
        let (events_sender, mut events_receiver) = mpsc::channel::<EventEntry>(CHANNEL_SIZE);
        events_senders_map.insert(REQUEST_ID.to_owned(), events_sender);

        let ank_base_response = ank_base::Response {
            request_id: REQUEST_ID.to_owned(),
            response_content: Some(AnkaiosResponseContent::CompleteStateResponse(Box::new(
                ank_base::CompleteStateResponse {
                    complete_state: Some(ank_base::CompleteState::default()),
                    altered_fields: Some(ank_base::AlteredFields {
                        added_fields: vec!["desiredState.workloads.test".to_owned()],
                        updated_fields: Vec::default(),
                        removed_fields: Vec::default(),
                    }),
                },
            ))),
        };

        CommandInterfaceConnection::handle_decoded_response(
            FromServer {
                from_server_enum: Some(FromServerEnum::Response(ank_base_response)),
            },
            &connection.response_sender,
            &mut log_senders_map,
            &mut events_senders_map,
        )
        .await;

        let event = tokio::time::timeout(Duration::from_millis(100), events_receiver.recv())
            .await
            .expect("event should have been forwarded")
            .expect("channel should not be closed");
        assert_eq!(
            event.added_fields,
            vec!["desiredState.workloads.test".to_owned()]
        );
    }

    #[tokio::test]
    async fn utest_command_interface_handle_decoded_response_ignores_non_response_messages() {
        let (connection, mut response_receiver) = generate_test_command_interface_connection();
        let mut log_senders_map = connection.log_senders_map.clone();
        let mut events_senders_map = connection.events_senders_map.clone();

        for from_server_enum in [
            FromServerEnum::ServerHello(crate::ankaios_api::grpc_api::ServerHello::default()),
            FromServerEnum::UpdateWorkloadState(
                crate::ankaios_api::grpc_api::UpdateWorkloadState::default(),
            ),
            FromServerEnum::LogsCancelRequest(
                crate::ankaios_api::grpc_api::LogsCancelRequest::default(),
            ),
            FromServerEnum::UpdateWorkload(crate::ankaios_api::grpc_api::UpdateWorkload::default()),
        ] {
            CommandInterfaceConnection::handle_decoded_response(
                FromServer {
                    from_server_enum: Some(from_server_enum),
                },
                &connection.response_sender,
                &mut log_senders_map,
                &mut events_senders_map,
            )
            .await;
        }
        CommandInterfaceConnection::handle_decoded_response(
            FromServer {
                from_server_enum: None,
            },
            &connection.response_sender,
            &mut log_senders_map,
            &mut events_senders_map,
        )
        .await;

        assert!(response_receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn utest_run_reader_loop_reconnects_after_stream_drop() {
        let (response_sender, mut response_receiver) = mpsc::channel::<Response>(CHANNEL_SIZE);
        let state = Arc::new(Mutex::new(CommandInterfaceState::Connected));
        let writer_ch_sender = Arc::new(Mutex::new(None));

        let mut mock_transport = MockReaderTransport::new();
        let mut seq = mockall::Sequence::new();

        mock_transport
            .expect_next_message()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|| {
                Ok(Some(FromServer {
                    from_server_enum: Some(FromServerEnum::Response(
                        generate_test_ank_base_update_state_success("req_1".to_owned()),
                    )),
                }))
            });
        mock_transport
            .expect_next_message()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|| Err(tonic::Status::unavailable("connection dropped")));

        let (new_sender, _new_receiver) = mpsc::channel::<ToServer>(1);
        mock_transport
            .expect_attempt_connect()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(move || Ok(new_sender));
        // No further expectations on purpose: the next next_message() call panics (harmlessly,
        // inside the task), freezing state/writer_ch_sender right after reconnecting instead of
        // racing them.

        let task = tokio::spawn(run_reader_loop(
            mock_transport,
            Arc::clone(&state),
            Arc::clone(&writer_ch_sender),
            response_sender,
            SynchronizedSenderMap::default(),
            SynchronizedSenderMap::default(),
        ));

        let first = tokio::time::timeout(Duration::from_millis(200), response_receiver.recv())
            .await
            .expect("first response should have been forwarded")
            .expect("channel should not be closed");
        assert_eq!(first.get_request_id(), "req_1");

        // Give the loop enough headroom to notice the drop, wait RECONNECT_INTERVAL_MILLIS,
        // reconnect and (harmlessly) panic on the unmocked call after that.
        sleep(Duration::from_secs(2)).await;

        assert_eq!(*state.lock().unwrap(), CommandInterfaceState::Connected);
        assert!(writer_ch_sender.lock().unwrap().is_some());
        assert!(
            task.is_finished(),
            "task should have ended (panicked on the unmocked call)"
        );
    }

    #[tokio::test]
    async fn utest_run_reader_loop_stops_reconnecting_once_terminated() {
        let (response_sender, _response_receiver) = mpsc::channel::<Response>(CHANNEL_SIZE);
        let state = Arc::new(Mutex::new(CommandInterfaceState::Connected));
        let writer_ch_sender = Arc::new(Mutex::new(None));

        let mut mock_transport = MockReaderTransport::new();
        // The stream is already gone from the very first read.
        mock_transport.expect_next_message().returning(|| Ok(None));
        // attempt_connect() must never be called once disconnect() has set Terminated.
        mock_transport.expect_attempt_connect().times(0);

        *state.lock().unwrap() = CommandInterfaceState::Terminated;

        let task = tokio::spawn(run_reader_loop(
            mock_transport,
            Arc::clone(&state),
            Arc::clone(&writer_ch_sender),
            response_sender,
            SynchronizedSenderMap::default(),
            SynchronizedSenderMap::default(),
        ));

        tokio::time::timeout(Duration::from_millis(200), task)
            .await
            .expect("loop should exit promptly once state is Terminated")
            .expect("task should not panic");
    }
}
