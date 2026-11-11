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
#[cfg(unix)]
use hyper_util::rt::TokioIo;
#[cfg(test)]
use mockall::automock;
use std::error::Error as StdError;
use std::fmt;
#[cfg(unix)]
use std::io::{Error as IoError, ErrorKind};
#[cfg(unix)]
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
#[cfg(unix)]
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep, timeout as tokio_timeout};
use tokio_stream::wrappers::ReceiverStream;
#[cfg(unix)]
use tonic::transport::Endpoint;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
#[cfg(unix)]
use tower::service_fn;

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
/// The transport is selected first, with [`GrpcConfig::tcp`] or [`GrpcConfig::unix_socket`],
/// and then refined with the methods of the returned builder:
///
/// * [`GrpcTcpConfig`]: [`url`](GrpcTcpConfig::url) sets a custom server URL and
///   [`mtls`](GrpcTcpConfig::mtls) secures the connection. Without `mtls` the connection is
///   insecure (plaintext);
/// * [`GrpcUnixSocketConfig`]: [`url`](GrpcUnixSocketConfig::url) sets a custom socket. No TLS
///   is used, access is controlled by the permissions of the socket file.
///
/// The builders can be passed directly to
/// [`AnkaiosBuilder::command_interface`](crate::AnkaiosBuilder::command_interface).
/// The configuration is validated when connecting.
///
/// ## Examples
///
/// ```rust
/// # use ankaios_sdk::GrpcConfig;
/// // Insecure TCP connection to the default URL.
/// let config = GrpcConfig::tcp();
///
/// // Insecure TCP connection to a custom URL.
/// let config = GrpcConfig::tcp().url("http://192.168.0.10:25551");
///
/// // mTLS connection.
/// # let (ca_pem, crt_pem, key_pem) = (String::new(), String::new(), String::new());
/// let config = GrpcConfig::tcp()
///     .url("https://192.168.0.10:25551")
///     .mtls(ca_pem, crt_pem, key_pem);
///
/// // Unix domain socket connection to the default socket.
/// # #[cfg(unix)]
/// let config = GrpcConfig::unix_socket();
///
/// // Unix domain socket connection to a custom socket.
/// # #[cfg(unix)]
/// let config = GrpcConfig::unix_socket().url("unix:///run/ankaios.sock");
/// ```
#[derive(Clone)]
pub struct GrpcConfig {
    target: ConnectionTarget,
}

/// How the [`GrpcConfig`] reaches the Ankaios server. Kept private so that invalid
/// combinations, like TLS over a Unix domain socket, cannot be expressed.
#[derive(Clone)]
enum ConnectionTarget {
    Tcp(GrpcTcpConfig),
    #[cfg(unix)]
    UnixSocket(GrpcUnixSocketConfig),
}

/// The default URL of the Ankaios server for TCP connections, without the scheme.
const DEFAULT_TCP_ADDRESS: &str = "127.0.0.1:25551";
/// The default socket the Ankaios server listens on.
#[cfg(unix)]
const DEFAULT_UNIX_SOCKET_PATH: &str = "/run/ankaios.sock";
/// The scheme prefix used by Ankaios for Unix domain socket URLs.
#[cfg(unix)]
const UNIX_SCHEME: &str = "unix://";

impl GrpcConfig {
    /// Selects the TCP transport. Use [`GrpcTcpConfig::url`] to set a custom server URL
    /// (default: `127.0.0.1:25551`) and [`GrpcTcpConfig::mtls`] to secure the connection.
    #[must_use]
    pub fn tcp() -> GrpcTcpConfig {
        GrpcTcpConfig {
            server_url: DEFAULT_TCP_ADDRESS.to_owned(),
            mtls: None,
        }
    }

    /// Selects the Unix domain socket transport. Use [`GrpcUnixSocketConfig::url`] to set a
    /// custom socket (default: `/run/ankaios.sock`).
    ///
    /// No TLS is used: access is controlled by the permissions of the socket file, so the
    /// process needs to be allowed to access it (e.g. by being a member of the group
    /// configured as `socket_group` in the Ankaios server).
    #[cfg(unix)]
    #[must_use]
    pub fn unix_socket() -> GrpcUnixSocketConfig {
        GrpcUnixSocketConfig {
            socket_url: DEFAULT_UNIX_SOCKET_PATH.to_owned(),
        }
    }

    /// Creates a new insecure (plaintext) [`GrpcConfig`] for the given server URL.
    ///
    /// ## Arguments
    ///
    /// * `server_url` - The URL of the Ankaios server, e.g. `http://127.0.0.1:25551`.
    #[deprecated(
        since = "1.2.0",
        note = "use `GrpcConfig::tcp().url(server_url)` instead"
    )]
    #[must_use]
    pub fn insecure(server_url: impl Into<String>) -> Self {
        Self::tcp().url(server_url).into()
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
    #[deprecated(
        since = "1.2.0",
        note = "use `GrpcConfig::tcp().url(server_url).mtls(ca_pem, crt_pem, key_pem)` instead"
    )]
    #[must_use]
    pub fn mtls(
        server_url: impl Into<String>,
        ca_pem: impl Into<String>,
        crt_pem: impl Into<String>,
        key_pem: impl Into<String>,
    ) -> Self {
        Self::tcp()
            .url(server_url)
            .mtls(ca_pem, crt_pem, key_pem)
            .into()
    }
}

impl fmt::Debug for GrpcConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("GrpcConfig");
        match &self.target {
            ConnectionTarget::Tcp(tcp) => tcp.debug_fields(&mut debug),
            #[cfg(unix)]
            ConnectionTarget::UnixSocket(socket) => socket.debug_fields(&mut debug),
        }
        debug.finish()
    }
}

/// The certificate material of an mTLS connection.
#[derive(Clone)]
struct MtlsMaterial {
    ca_pem: String,
    crt_pem: String,
    key_pem: String,
}

/// Builder for a [`GrpcConfig`] using the TCP transport, created with [`GrpcConfig::tcp`].
///
/// Without [`mtls`](Self::mtls) the connection is insecure (plaintext).
///
/// ## Examples
///
/// ```rust
/// # use ankaios_sdk::GrpcConfig;
/// # let (ca_pem, crt_pem, key_pem) = (String::new(), String::new(), String::new());
/// let config = GrpcConfig::tcp()
///     .url("https://192.168.0.10:25551")
///     .mtls(ca_pem, crt_pem, key_pem);
/// ```
#[derive(Clone)]
pub struct GrpcTcpConfig {
    server_url: String,
    mtls: Option<MtlsMaterial>,
}

impl GrpcTcpConfig {
    /// Sets the URL of the Ankaios server, e.g. `http://192.168.0.10:25551`. If not set,
    /// `127.0.0.1:25551` is used.
    ///
    /// The scheme must match the security of the connection: `http` without
    /// [`mtls`](Self::mtls) and `https` with it. If the scheme is omitted, it is derived from
    /// that. A mismatch is reported when connecting.
    ///
    /// ## Arguments
    ///
    /// * `server_url` - The URL of the Ankaios server.
    #[must_use]
    pub fn url(mut self, server_url: impl Into<String>) -> Self {
        self.server_url = server_url.into();
        self
    }

    /// Secures the connection with mTLS, using the given PEM-encoded certificate content.
    ///
    /// ## Arguments
    ///
    /// * `ca_pem` - The PEM-encoded CA certificate content;
    /// * `crt_pem` - The PEM-encoded client certificate content;
    /// * `key_pem` - The PEM-encoded client private key content.
    #[must_use]
    pub fn mtls(
        mut self,
        ca_pem: impl Into<String>,
        crt_pem: impl Into<String>,
        key_pem: impl Into<String>,
    ) -> Self {
        self.mtls = Some(MtlsMaterial {
            ca_pem: ca_pem.into(),
            crt_pem: crt_pem.into(),
            key_pem: key_pem.into(),
        });
        self
    }

    /// The URL to connect to, with the scheme derived if missing.
    fn resolved_url(&self) -> String {
        let url = &self.server_url;
        if url.contains("://") {
            url.clone()
        } else if self.mtls.is_some() {
            format!("https://{url}")
        } else {
            format!("http://{url}")
        }
    }

    /// Checks that the URL fits the transport and the security of the connection.
    fn validated_url(&self) -> Result<String, AnkaiosError> {
        let url = self.resolved_url();
        let mismatch = if url.starts_with("unix://") {
            Some("a unix socket URL requires `GrpcConfig::unix_socket()`")
        } else if self.mtls.is_some() && url.starts_with("http://") {
            Some("mTLS requires an 'https' URL")
        } else if self.mtls.is_none() && url.starts_with("https://") {
            Some("an 'https' URL requires `mtls(...)`, otherwise use an 'http' URL")
        } else {
            None
        };
        match mismatch {
            Some(reason) => Err(AnkaiosError::ConnectionError(format!(
                "Invalid gRPC server URL '{url}': {reason}."
            ))),
            None => Ok(url),
        }
    }

    fn debug_fields(&self, debug: &mut fmt::DebugStruct<'_, '_>) {
        let redacted = self.mtls.as_ref().map(|_| "<redacted>");
        debug
            .field("server_url", &self.resolved_url())
            .field("insecure", &self.mtls.is_none())
            .field("ca_pem", &redacted)
            .field("crt_pem", &redacted)
            .field("key_pem", &redacted);
    }
}

impl From<GrpcTcpConfig> for GrpcConfig {
    fn from(tcp: GrpcTcpConfig) -> Self {
        Self {
            target: ConnectionTarget::Tcp(tcp),
        }
    }
}

impl fmt::Debug for GrpcTcpConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("GrpcTcpConfig");
        self.debug_fields(&mut debug);
        debug.finish()
    }
}

/// Builder for a [`GrpcConfig`] using a Unix domain socket, created with
/// [`GrpcConfig::unix_socket`]. Only available on Unix.
///
/// No TLS is used: access is controlled by the permissions of the socket file.
///
/// ## Examples
///
/// ```rust
/// # use ankaios_sdk::GrpcConfig;
/// # #[cfg(unix)]
/// let config = GrpcConfig::unix_socket().url("/run/ankaios.sock");
/// ```
#[cfg(unix)]
#[derive(Clone, Debug)]
pub struct GrpcUnixSocketConfig {
    socket_url: String,
}

#[cfg(unix)]
impl GrpcUnixSocketConfig {
    /// Sets the socket the Ankaios server listens on. If not set, `/run/ankaios.sock` is
    /// used.
    ///
    /// Both a plain absolute path (`/run/ankaios.sock`) and the Ankaios URL form
    /// (`unix:///run/ankaios.sock`) are accepted. The path must be absolute, which is
    /// validated when connecting.
    ///
    /// ## Arguments
    ///
    /// * `socket_url` - The path or `unix://` URL of the socket.
    #[must_use]
    pub fn url(mut self, socket_url: impl Into<String>) -> Self {
        self.socket_url = socket_url.into();
        self
    }

    /// The path of the socket, with the `unix://` scheme removed.
    fn resolved_path(&self) -> PathBuf {
        let url = &self.socket_url;
        PathBuf::from(url.strip_prefix(UNIX_SCHEME).unwrap_or(url))
    }

    /// Checks that the socket path is usable.
    fn validated_path(&self) -> Result<PathBuf, AnkaiosError> {
        let path = self.resolved_path();
        if path.as_os_str().is_empty() {
            return Err(AnkaiosError::ConnectionError(
                "Invalid unix socket path: the path is empty.".to_owned(),
            ));
        }
        if !path.is_absolute() {
            return Err(AnkaiosError::ConnectionError(format!(
                "Invalid unix socket path: the path must be absolute: '{}'.",
                path.display()
            )));
        }
        Ok(path)
    }

    fn debug_fields(&self, debug: &mut fmt::DebugStruct<'_, '_>) {
        debug.field("socket_path", &self.resolved_path());
    }
}

#[cfg(unix)]
impl From<GrpcUnixSocketConfig> for GrpcConfig {
    fn from(socket: GrpcUnixSocketConfig) -> Self {
        Self {
            target: ConnectionTarget::UnixSocket(socket),
        }
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
#[allow(clippy::double_must_use)] // async_trait's generated futures already carry #[must_use].
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

/// Joins the message of the error with the ones of its sources. Tonic's transport error only
/// displays `transport error`, the actual cause is only available in its source chain.
fn error_chain(err: &(dyn StdError + 'static)) -> String {
    let mut message = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        let cause_message = cause.to_string();
        if !message.contains(&cause_message) {
            message.push_str(" <- ");
            message.push_str(&cause_message);
        }
        source = cause.source();
    }
    message
}

/// Checks if the error was caused by missing permissions on the file system.
#[cfg(unix)]
fn has_permission_denied(err: &(dyn StdError + 'static)) -> bool {
    let mut current = Some(err);
    while let Some(error) = current {
        if error
            .downcast_ref::<IoError>()
            .is_some_and(|io_error| io_error.kind() == ErrorKind::PermissionDenied)
        {
            return true;
        }
        current = error.source();
    }
    false
}

/// Reads from `transport` until the connection is closed or errors, forwarding every decoded
/// message to [`CommandInterfaceConnection::handle_decoded_response`].
async fn read_until_disconnected<T: ReaderTransport>(
    transport: &mut T,
    response_sender: &mpsc::Sender<Response>,
    logs_sender_map: &mut SynchronizedSenderMap<LogResponse>,
    events_sender_map: &mut SynchronizedSenderMap<EventEntry>,
) {
    loop {
        match transport.next_message().await {
            Ok(Some(from_server)) => {
                CommandInterfaceConnection::handle_decoded_response(
                    from_server,
                    response_sender,
                    logs_sender_map,
                    events_sender_map,
                )
                .await;
            }
            Ok(None) => {
                log::warn!("The connection to the Ankaios server was closed.");
                return;
            }
            Err(status) => {
                log::error!("Error while reading from the connection: '{status}'");
                return;
            }
        }
    }
}

/// Retries `transport.attempt_connect()` every [`RECONNECT_INTERVAL_MILLIS`] until it succeeds
/// or `state` becomes [`CommandInterfaceState::Terminated`]. Returns `true` once reconnected
/// (with `state` set back to [`CommandInterfaceState::Connected`]), or `false` if `disconnect()`
/// won the race.
async fn reconnect_until_connected_or_terminated<T: ReaderTransport>(
    transport: &mut T,
    state: &Arc<Mutex<CommandInterfaceState>>,
    writer_ch_sender: &Arc<Mutex<Option<mpsc::Sender<ToServer>>>>,
) -> bool {
    loop {
        sleep(Duration::from_millis(RECONNECT_INTERVAL_MILLIS)).await;
        let sender = match transport.attempt_connect().await {
            Ok(sender) => sender,
            Err(err) => {
                log::debug!("Reconnect attempt failed: '{err}'");
                continue;
            }
        };

        let mut state_guard = state.lock().unwrap_or_else(|_| unreachable!());
        if *state_guard == CommandInterfaceState::Terminated {
            return false; // disconnect() won; drop the new sender/stream.
        }
        *writer_ch_sender.lock().unwrap_or_else(|_| unreachable!()) = Some(sender);
        *state_guard = CommandInterfaceState::Connected;
        log::info!("Reconnected to the Ankaios server.");
        return true;
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
    loop {
        read_until_disconnected(
            &mut transport,
            &response_sender,
            &mut logs_sender_map,
            &mut events_sender_map,
        )
        .await;

        {
            let mut state_guard = state.lock().unwrap_or_else(|_| unreachable!());
            if *state_guard == CommandInterfaceState::Terminated {
                // disconnect() was called; don't try to reconnect.
                return;
            }
            *state_guard = CommandInterfaceState::Reconnecting;
        }
        log::warn!(
            "Lost connection to the Ankaios server, attempting to reconnect every {RECONNECT_INTERVAL_MILLIS}ms..."
        );

        if !reconnect_until_connected_or_terminated(&mut transport, &state, &writer_ch_sender).await
        {
            return; // disconnect() was called during reconnect attempts.
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

    /// Builds the [`Channel`] used to reach the Ankaios server: plaintext, mTLS-secured or
    /// through a Unix domain socket, depending on the [`GrpcConfig`].
    async fn build_channel(config: &GrpcConfig) -> Result<Channel, AnkaiosError> {
        match &config.target {
            ConnectionTarget::Tcp(tcp) => Self::connect_tcp_channel(tcp).await,
            #[cfg(unix)]
            ConnectionTarget::UnixSocket(socket) => {
                Self::connect_unix_channel(&socket.validated_path()?).await
            }
        }
    }

    /// Builds the plaintext or mTLS-secured [`Channel`] over TCP.
    async fn connect_tcp_channel(tcp: &GrpcTcpConfig) -> Result<Channel, AnkaiosError> {
        let server_url = tcp.validated_url()?;
        let mut endpoint = Channel::from_shared(server_url.clone()).map_err(|err| {
            AnkaiosError::ConnectionError(format!(
                "Invalid gRPC server URL '{server_url}': '{err}'"
            ))
        })?;
        if let Some(mtls) = &tcp.mtls {
            // Ankaios server certificates are always issued for this domain name,
            // regardless of the actual connection address.
            let client_tls = ClientTlsConfig::new()
                .domain_name("ank-server")
                .ca_certificate(Certificate::from_pem(&mtls.ca_pem))
                .identity(Identity::from_pem(&mtls.crt_pem, &mtls.key_pem));
            endpoint = endpoint.tls_config(client_tls).map_err(|err| {
                AnkaiosError::ConnectionError(format!("Invalid TLS configuration: '{err}'"))
            })?;
        }

        endpoint.connect().await.map_err(|err| {
            AnkaiosError::ConnectionError(format!(
                "Could not connect to the Ankaios server: '{}'",
                error_chain(&err)
            ))
        })
    }

    /// Builds the [`Channel`] over a Unix domain socket. The path is re-dialed on every call,
    /// so a reconnect also works after the server recreated the socket file.
    #[cfg(unix)]
    async fn connect_unix_channel(socket_path: &Path) -> Result<Channel, AnkaiosError> {
        let connector_path = socket_path.to_path_buf();
        // Tonic requires an endpoint URI even with a custom connector. The URI is not used to
        // establish the connection, `UnixStream::connect` is.
        Endpoint::from_static("http://unix-socket")
            .connect_with_connector(service_fn(move |_| {
                let path = connector_path.clone();
                async move { UnixStream::connect(path).await.map(TokioIo::new) }
            }))
            .await
            .map_err(|err| {
                let mut message = format!(
                    "Could not connect to the Ankaios server on '{}': '{}'.",
                    socket_path.display(),
                    error_chain(&err)
                );
                if has_permission_denied(&err) {
                    message.push_str(
                        " The process needs access to the socket: run it as root, as the \
                         owner of the socket or as a member of its group (by default \
                         'ankaios').",
                    );
                }
                AnkaiosError::ConnectionError(message)
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
        *self.state.lock().unwrap_or_else(|_| unreachable!()) = CommandInterfaceState::Connected;
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
        let maybe_sender = {
            let state_guard = self.state.lock().unwrap_or_else(|_| unreachable!());
            if *state_guard == CommandInterfaceState::Connected {
                self.writer_ch_sender
                    .lock()
                    .unwrap_or_else(|_| unreachable!())
                    .clone()
            } else {
                None
            }
        };
        let Some(sender) = maybe_sender else {
            log::error!("Could not write to the command interface, not connected.");
            return Err(AnkaiosError::ConnectionError(
                "Could not write to the command interface, not connected.".to_owned(),
            ));
        };
        sender
            .send(ToServer {
                to_server_enum: Some(ToServerEnum::Request(request)),
            })
            .await
            .map_err(|err| {
                log::error!("Error while sending request: '{err}'");
                AnkaiosError::ConnectionError(format!(
                    "Could not write to the command interface, connection lost: '{err}'"
                ))
            })
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

    #[cfg(unix)]
    use super::has_permission_denied;
    use super::{
        CommandInterfaceConnection, CommandInterfaceState, Connection, FromServer, FromServerEnum,
        GrpcConfig, MockReaderTransport, SynchronizedSenderMap, ToServer, ToServerEnum,
        error_chain, run_reader_loop,
    };
    use crate::ankaios_api::ank_base::{self, response::ResponseContent as AnkaiosResponseContent};
    use crate::components::request::{Request, generate_test_request};
    use crate::components::response::{
        generate_test_ank_base_update_state_success, generate_test_proto_log_entries_response,
    };
    use crate::{AnkaiosError, EventEntry, LogResponse, Response};
    #[cfg(unix)]
    use std::io::{Error as IoError, ErrorKind};
    #[cfg(unix)]
    use std::path::PathBuf;

    const SERVER_URL: &str = "http://127.0.0.1:25551";
    const REQUEST_ID: &str = "request_id_1";
    const CHANNEL_SIZE: usize = 10;

    fn generate_test_command_interface_connection()
    -> (CommandInterfaceConnection, mpsc::Receiver<Response>) {
        let (response_sender, response_receiver) = mpsc::channel::<Response>(CHANNEL_SIZE);
        (
            CommandInterfaceConnection::new(
                GrpcConfig::tcp().url(SERVER_URL).into(),
                response_sender,
            ),
            response_receiver,
        )
    }

    #[test]
    fn utest_grpc_config_without_tls() {
        let config = GrpcConfig::from(GrpcConfig::tcp().url(SERVER_URL));

        assert_eq!(
            format!("{config:?}"),
            "GrpcConfig { server_url: \"http://127.0.0.1:25551\", insecure: true, ca_pem: None, crt_pem: None, key_pem: None }"
        );
    }

    #[test]
    fn utest_grpc_config_with_tls() {
        let config = GrpcConfig::from(GrpcConfig::tcp().url("https://127.0.0.1:25551").mtls(
            "ca-secret",
            "crt-secret",
            "key-secret",
        ));

        assert_eq!(
            format!("{config:?}"),
            "GrpcConfig { server_url: \"https://127.0.0.1:25551\", insecure: false, ca_pem: Some(\"<redacted>\"), crt_pem: Some(\"<redacted>\"), key_pem: Some(\"<redacted>\") }"
        );
    }

    #[test]
    fn utest_grpc_tcp_config_defaults() {
        let config = GrpcConfig::tcp();
        assert_eq!(config.resolved_url(), "http://127.0.0.1:25551");
        assert_eq!(config.validated_url().unwrap(), "http://127.0.0.1:25551");

        let config = GrpcConfig::tcp().mtls("ca", "crt", "key");
        assert_eq!(config.validated_url().unwrap(), "https://127.0.0.1:25551");
    }

    #[test]
    fn utest_grpc_tcp_config_url_scheme() {
        // Without a scheme, it is derived from the security of the connection.
        let config = GrpcConfig::tcp().url("192.168.0.10:25551");
        assert_eq!(config.validated_url().unwrap(), "http://192.168.0.10:25551");

        // The order of the calls does not matter.
        let config = GrpcConfig::tcp()
            .mtls("ca", "crt", "key")
            .url("192.168.0.10:25551");
        assert_eq!(
            config.validated_url().unwrap(),
            "https://192.168.0.10:25551"
        );

        let config = GrpcConfig::tcp()
            .url("https://host:1")
            .mtls("ca", "crt", "key");
        assert_eq!(config.validated_url().unwrap(), "https://host:1");
    }

    #[test]
    fn utest_grpc_tcp_config_url_mismatch() {
        let configs = [
            GrpcConfig::tcp().url("https://127.0.0.1:25551"),
            GrpcConfig::tcp()
                .url("http://127.0.0.1:25551")
                .mtls("ca", "crt", "key"),
            GrpcConfig::tcp().url("unix:///run/ankaios.sock"),
        ];
        for config in configs {
            assert!(matches!(
                config.validated_url(),
                Err(AnkaiosError::ConnectionError(_))
            ));
        }
    }

    #[test]
    fn utest_grpc_tcp_config_debug_redacts_secrets() {
        let config = GrpcConfig::tcp().mtls("ca-secret", "crt-secret", "key-secret");
        let debug = format!("{config:?}");
        assert!(debug.starts_with("GrpcTcpConfig {"));
        assert!(!debug.contains("secret"));
    }

    #[test]
    #[allow(deprecated)]
    fn utest_grpc_config_deprecated_constructors() {
        let insecure = GrpcConfig::insecure(SERVER_URL);
        assert_eq!(
            format!("{insecure:?}"),
            format!("{:?}", GrpcConfig::from(GrpcConfig::tcp().url(SERVER_URL)))
        );

        let mtls = GrpcConfig::mtls("https://127.0.0.1:25551", "ca", "crt", "key");
        assert_eq!(
            format!("{mtls:?}"),
            format!(
                "{:?}",
                GrpcConfig::from(
                    GrpcConfig::tcp()
                        .url("https://127.0.0.1:25551")
                        .mtls("ca", "crt", "key")
                )
            )
        );
    }

    #[test]
    fn utest_error_chain() {
        #[derive(Debug)]
        struct Outer(Inner);
        #[derive(Debug)]
        struct Inner;
        impl std::fmt::Display for Outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "outer")
            }
        }
        impl std::fmt::Display for Inner {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "inner")
            }
        }
        impl std::error::Error for Inner {}
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }

        assert_eq!(error_chain(&Inner), "inner");
        assert_eq!(error_chain(&Outer(Inner)), "outer <- inner");
    }

    #[cfg(unix)]
    #[test]
    fn utest_grpc_unix_socket_config_path() {
        let config = GrpcConfig::unix_socket();
        assert_eq!(
            config.validated_path().unwrap(),
            PathBuf::from("/run/ankaios.sock")
        );

        for url in ["/tmp/ank.sock", "unix:///tmp/ank.sock"] {
            let config = GrpcConfig::unix_socket().url(url);
            assert_eq!(
                config.validated_path().unwrap(),
                PathBuf::from("/tmp/ank.sock")
            );
        }

        let config: GrpcConfig = GrpcConfig::unix_socket().url("unix:///tmp/ank.sock").into();
        assert_eq!(
            format!("{config:?}"),
            "GrpcConfig { socket_path: \"/tmp/ank.sock\" }"
        );
    }

    #[cfg(unix)]
    #[test]
    fn utest_grpc_unix_socket_config_invalid_path() {
        for url in ["", "unix://", "relative.sock", "unix://relative.sock"] {
            assert!(
                matches!(
                    GrpcConfig::unix_socket().url(url).validated_path(),
                    Err(AnkaiosError::ConnectionError(_))
                ),
                "'{url}' should be rejected"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn utest_has_permission_denied() {
        let denied = IoError::from(ErrorKind::PermissionDenied);
        let not_found = IoError::from(ErrorKind::NotFound);
        assert!(has_permission_denied(&denied));
        assert!(!has_permission_denied(&not_found));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn utest_build_channel_unix_socket_invalid_path() {
        let config: GrpcConfig = GrpcConfig::unix_socket().url("relative.sock").into();
        let result = CommandInterfaceConnection::build_channel(&config).await;
        assert!(
            matches!(result, Err(AnkaiosError::ConnectionError(msg)) if msg.contains("absolute"))
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn utest_build_channel_unix_socket_missing() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("missing.sock");
        let config: GrpcConfig = GrpcConfig::unix_socket()
            .url(socket.to_string_lossy())
            .into();

        let result = CommandInterfaceConnection::build_channel(&config).await;
        assert!(matches!(
            result,
            Err(AnkaiosError::ConnectionError(msg)) if msg.contains("No such file or directory")
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn utest_build_channel_unix_socket_reaches_listener() {
        use tokio::io::AsyncReadExt;
        use tokio::net::UnixListener;

        const HTTP2_PREFACE: &[u8] = b"PRI * HTTP/2.0";

        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("ankaios.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0_u8; HTTP2_PREFACE.len()];
            stream.read_exact(&mut buffer).await.unwrap();
            buffer
        });

        // Both the plain path and the `unix://` URL reach the same socket.
        let config: GrpcConfig = GrpcConfig::unix_socket()
            .url(format!("unix://{}", socket.display()))
            .into();
        let channel = CommandInterfaceConnection::build_channel(&config).await;
        assert!(channel.is_ok());

        let received = tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("the listener did not receive anything")
            .unwrap();
        assert_eq!(received, HTTP2_PREFACE);
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
            GrpcConfig::tcp().url("http://127.0.0.1:0").into(),
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
    async fn utest_command_interface_write_request_fails_when_channel_closed() {
        let (mut connection, _response_receiver) = generate_test_command_interface_connection();
        let (grpc_tx, grpc_rx) = mpsc::channel::<ToServer>(CHANNEL_SIZE);
        drop(grpc_rx);
        *connection.writer_ch_sender.lock().unwrap() = Some(grpc_tx);
        *connection.state.lock().unwrap() = CommandInterfaceState::Connected;

        let result = connection
            .write_request(generate_test_request().to_proto())
            .await;
        assert!(matches!(result, Err(AnkaiosError::ConnectionError(_))));
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
