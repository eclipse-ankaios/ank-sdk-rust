// Copyright (c) 2024 Elektrobit Automotive GmbH
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

//! This module contains the [`ControlInterface`] struct and the [`ControlInterfaceState`] enum.

use prost::{encoding::decode_varint, Message};
use std::{
    collections::HashMap,
    fmt,
    fs::metadata,
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, Error, ErrorKind},
    net::unix::pipe,
    spawn,
    sync::mpsc,
    task::JoinHandle,
    time::{sleep, Duration},
};

use crate::ankaios_api;
use crate::components::request::Request;
use crate::components::{
    response::LogResponse,
    response::{Response, ResponseType},
};
use crate::AnkaiosError;
use ankaios_api::control_api::{to_ankaios::ToAnkaiosEnum, FromAnkaios, Hello, ToAnkaios};

#[cfg(test)]
use mockall::automock;

/// Base path for the control interface FIFO pipes.
const ANKAIOS_CONTROL_INTERFACE_BASE_PATH: &str = "/run/ankaios/control_interface";
/// Input fifo path from the base path
const ANKAIOS_INPUT_FIFO_PATH: &str = "input";
/// Output fifo path from the base path
const ANKAIOS_OUTPUT_FIFO_PATH: &str = "output";
/// Version of [Ankaios](https://eclipse-ankaios.github.io/ankaios) that is compatible
/// with the [`ControlInterface`] implementation.
const ANKAIOS_VERSION: &str = "0.6.0";
/// Maximum size of a varint in bytes.
const MAX_VARINT_SIZE: usize = 19;

/// Enum representing the state of the control interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum ControlInterfaceState {
    /// The control interface is initialized.
    Initialized = 1,
    /// The control interface is terminated.
    Terminated = 2,
    /// The agent is disconnected.
    AgentDisconnected = 3,
    /// The connection is closed. This state is unrecoverable.
    ConnectionClosed = 4,
}

/// This struct handles the interaction with the control interface.
/// It provides means to send and receive messages through the FIFO pipes.
///
/// It uses two [tokio] tasks, one for reading from the input FIFO and one for
/// writing to the output FIFO.
pub struct ControlInterface {
    /// Path to the FIFO pipes directory.
    pub(crate) path: String,
    /// Output file for writing to the control interface.
    output_file: Option<pipe::Sender>,
    /// Handler for the read thread.
    read_thread_handler: Option<JoinHandle<Result<(), AnkaiosError>>>,
    /// Handler for the write thread.
    writer_thread_handler: Option<JoinHandle<Result<(), AnkaiosError>>>,
    /// State of the control interface.
    state: Arc<Mutex<ControlInterfaceState>>,
    /// Sender for the response channel.
    response_sender: mpsc::Sender<Response>,
    /// Sender for the writer channel.
    writer_ch_sender: Option<mpsc::Sender<ToAnkaios>>,
    /// Request ID to logs sender mapping
    request_id_to_logs_sender: Arc<Mutex<HashMap<String, mpsc::Sender<LogResponse>>>>,
}

/// Helper function that reads varint data from the input pipe.
///
/// ## Arguments
///
/// * `file` - A mutable reference to the input file.
///
/// ## Returns
///
/// A result containing the varint data as a byte array or an [Error].
async fn read_varint_data(
    file: &mut BufReader<pipe::Receiver>,
) -> Result<[u8; MAX_VARINT_SIZE], Error> {
    let mut res = [0u8; MAX_VARINT_SIZE];
    for item in &mut res {
        *item = file.read_u8().await?;
        if *item & 0b1000_0000 == 0 {
            break;
        }
    }
    Ok(res)
}

/// Helper function that reads protobuf data from the input pipe.
///
/// ## Arguments
///
/// * `file` - A mutable reference to the input file.
///
/// ## Returns
///
/// A result containing the protobuf data as a byte array or an [Error].
async fn read_protobuf_data(file: &mut BufReader<pipe::Receiver>) -> Result<Vec<u8>, Error> {
    let varint_data = read_varint_data(file).await?;
    let mut boxed_varint_data = Box::new(&varint_data[..]);

    let size = usize::try_from(decode_varint(&mut boxed_varint_data)?)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid varint size"))?;

    let mut buf = vec![0; size];
    file.read_exact(&mut buf).await?;
    Ok(buf)
}

impl fmt::Display for ControlInterfaceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let res_str = match *self {
            ControlInterfaceState::Initialized => "Initialized",
            ControlInterfaceState::Terminated => "Terminated",
            ControlInterfaceState::AgentDisconnected => "AgentDisconnected",
            ControlInterfaceState::ConnectionClosed => "ConnectionClosed",
        };
        write!(f, "{res_str}")
    }
}

#[cfg_attr(test, automock)]
impl ControlInterface {
    /// Creates a new instance of the control interface.
    ///
    /// ## Arguments
    ///
    /// * `response_sender` - A sender for the response channel.
    ///
    /// ## Returns
    ///
    /// A new [`ControlInterface`] instance.
    pub fn new(response_sender: mpsc::Sender<Response>) -> Self {
        Self {
            path: ANKAIOS_CONTROL_INTERFACE_BASE_PATH.to_owned(),
            output_file: None,
            read_thread_handler: None,
            writer_thread_handler: None,
            state: Arc::new(Mutex::new(ControlInterfaceState::Terminated)),
            response_sender,
            writer_ch_sender: None,
            request_id_to_logs_sender: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Connects to the control interface.
    ///
    /// ## Returns
    ///
    /// An [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if the connection fails.
    pub async fn connect(&mut self) -> Result<(), AnkaiosError> {
        if *self.state.lock().unwrap_or_else(|_| unreachable!())
            == ControlInterfaceState::Initialized
        {
            return Err(AnkaiosError::ControlInterfaceError(
                "Already connected.".to_owned(),
            ));
        }
        if metadata(&(self.path.clone() + "/" + ANKAIOS_INPUT_FIFO_PATH)).is_err() {
            return Err(AnkaiosError::ControlInterfaceError(
                "Control interface input fifo does not exist.".to_owned(),
            ));
        }
        if metadata(&(self.path.clone() + "/" + ANKAIOS_OUTPUT_FIFO_PATH)).is_err() {
            return Err(AnkaiosError::ControlInterfaceError(
                "Control interface output fifo does not exist.".to_owned(),
            ));
        }

        self.prepare_writer();
        self.read_from_control_interface();
        ControlInterface::change_state(&self.state, ControlInterfaceState::Initialized);
        ControlInterface::send_initial_hello(
            self.writer_ch_sender
                .as_ref()
                .unwrap_or_else(|| unreachable!()),
        )
        .await;

        log::trace!("Connected to the control interface.");
        Ok(())
    }

    /// Disconnects from the control interface.
    ///
    /// ## Returns
    ///
    /// An [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if the disconnection fails.
    pub fn disconnect(&mut self) -> Result<(), AnkaiosError> {
        if *self.state.lock().unwrap_or_else(|_| unreachable!())
            != ControlInterfaceState::Initialized
        {
            return Err(AnkaiosError::ControlInterfaceError(
                "Already disconnected.".to_owned(),
            ));
        }
        if let Some(handler) = self.read_thread_handler.take() {
            handler.abort();
        }
        self.state
            .lock()
            .unwrap_or_else(|_| unreachable!())
            .clone_from(&ControlInterfaceState::Terminated);
        self.output_file = None;
        Ok(())
    }

    /// Changes the state of the control interface.
    /// This method should be used for all state changes inside the control interface.
    ///
    /// ## Arguments
    ///
    /// * `state` - A reference to the current state;
    /// * `new_state` - The new state to be set.
    fn change_state(state: &Arc<Mutex<ControlInterfaceState>>, new_state: ControlInterfaceState) {
        if *state.lock().unwrap_or_else(|_| unreachable!()) == new_state {
            return;
        }
        state
            .lock()
            .unwrap_or_else(|_| unreachable!())
            .clone_from(&new_state);
        log::info!("State changed: {new_state:?}");
    }

    /// Prepares the writer thread for the control interface.
    /// It uses a [tokio] task that waits for messages and sends them to the output FIFO.
    fn prepare_writer(&mut self) {
        let (writer_ch_sender, mut writer_ch_receiver) = mpsc::channel::<ToAnkaios>(5);
        self.writer_ch_sender = Some(writer_ch_sender.clone());
        let output_path = Path::new(&self.path)
            .to_path_buf()
            .join(ANKAIOS_OUTPUT_FIFO_PATH);
        let state_clone = Arc::<Mutex<ControlInterfaceState>>::clone(&self.state);
        self.writer_thread_handler = Some(spawn(async move {
            const AGENT_RECONNECT_INTERVAL: u64 = 1;
            let sender = pipe::OpenOptions::new()
                .open_sender(output_path)
                .map_err(|_| {
                    AnkaiosError::ControlInterfaceError("Could not open output fifo.".to_owned())
                })?;
            let mut output_file = BufWriter::new(sender);

            while let Some(message) = writer_ch_receiver.recv().await {
                output_file
                    .write_all(&message.encode_length_delimited_to_vec())
                    .await
                    .unwrap_or_else(|err| {
                        log::error!("Error while writing to output fifo: '{err}'");
                        // let _ = self.disconnect();
                    });
                #[allow(clippy::else_if_without_else)]
                if let Err(err) = output_file.flush().await {
                    if err.kind() == ErrorKind::BrokenPipe {
                        if *state_clone.lock().unwrap_or_else(|_| unreachable!())
                            == ControlInterfaceState::Initialized
                        {
                            ControlInterface::change_state(
                                &state_clone,
                                ControlInterfaceState::AgentDisconnected,
                            );
                        }
                        log::warn!("Waiting for the agent..");
                        sleep(Duration::from_secs(AGENT_RECONNECT_INTERVAL)).await;
                        ControlInterface::send_initial_hello(&writer_ch_sender).await;
                    } else {
                        log::error!("Error while flushing to output fifo: '{err}'");
                        // let _ = self.disconnect();
                    }
                } else if *state_clone.lock().unwrap_or_else(|_| unreachable!())
                    == ControlInterfaceState::AgentDisconnected
                {
                    ControlInterface::change_state(
                        &state_clone,
                        ControlInterfaceState::Initialized,
                    );
                }
            }
            Ok(())
        }));
    }

    /// Prepares the reader thread for the control interface.
    /// It uses a [tokio] task that reads continuously from the FIFO input pipe.
    fn read_from_control_interface(&mut self) {
        #[cfg(not(test))]
        const SLEEP_DURATION: u64 = 500; // ms
        #[cfg(test)]
        const SLEEP_DURATION: u64 = 50; // ms
        let input_path = Path::new(&self.path)
            .to_path_buf()
            .join(ANKAIOS_INPUT_FIFO_PATH);
        let response_sender_clone = self.response_sender.clone();
        let writer_ch_sender_clone = self
            .writer_ch_sender
            .as_ref()
            .unwrap_or_else(|| unreachable!())
            .clone();
        let state_clone = Arc::<Mutex<ControlInterfaceState>>::clone(&self.state);
        let request_id_logs_sender_map =
            Arc::<Mutex<HashMap<String, mpsc::Sender<LogResponse>>>>::clone(
                &self.request_id_to_logs_sender,
            );
        self.read_thread_handler = Some(spawn(async move {
            let receiver = pipe::OpenOptions::new()
                .open_receiver(input_path)
                .map_err(|_| {
                    AnkaiosError::ControlInterfaceError("Could not open input fifo.".to_owned())
                })?;
            let mut input_file = BufReader::new(receiver);

            loop {
                match read_protobuf_data(&mut input_file).await {
                    Ok(binary) => {
                        if *state_clone.lock().unwrap_or_else(|_| unreachable!())
                            == ControlInterfaceState::AgentDisconnected
                        {
                            log::info!("Agent reconnected successfully.");
                            ControlInterface::change_state(
                                &state_clone,
                                ControlInterfaceState::Initialized,
                            );
                        }

                        match FromAnkaios::decode(&mut Box::new(binary.as_ref())) {
                            Ok(from_ankaios) => {
                                let received_response = Response::new(from_ankaios);
                                let is_con_closed = matches!(
                                    received_response.content,
                                    ResponseType::ConnectionClosedReason(_)
                                );

                                if let ResponseType::LogEntriesResponse(log_entries) =
                                    received_response.content
                                {
                                    let request_id = received_response.id;
                                    let log_entries_sender = request_id_logs_sender_map
                                        .lock()
                                        .unwrap_or_else(|_| unreachable!())
                                        .get(&request_id)
                                        .cloned();

                                    if let Some(sender) = log_entries_sender {
                                        sender
                                            .send(LogResponse::LogEntries(log_entries))
                                            .await
                                            .unwrap_or_else(|err| {
                                                log::error!(
                                                    "Error while sending log entries: '{err}'"
                                                );
                                            });
                                    }
                                } else if let ResponseType::LogsStopResponse(instance_name) =
                                    received_response.content
                                {
                                    let request_id = received_response.id;
                                    let log_entries_sender = request_id_logs_sender_map
                                        .lock()
                                        .unwrap_or_else(|_| unreachable!())
                                        .remove(&request_id);
                                    if let Some(sender) = log_entries_sender {
                                        sender
                                            .send(LogResponse::LogsStopResponse(instance_name))
                                            .await
                                            .unwrap_or_else(|err| {
                                                log::error!(
                                                    "Error while sending log stop message: '{err}'"
                                                );
                                            });
                                    }
                                } else {
                                    response_sender_clone
                                        .send(received_response)
                                        .await
                                        .unwrap_or_else(|err| {
                                            log::error!("Error while sending response: '{err}'");
                                        });
                                }
                                if is_con_closed {
                                    log::error!("Connection closed by the agent.");
                                    break;
                                }
                            }
                            Err(err) => log::error!("Invalid response, parsing error: '{err}'"),
                        }
                    }
                    Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                        if *state_clone.lock().unwrap_or_else(|_| unreachable!())
                            == ControlInterfaceState::Initialized
                        {
                            ControlInterface::change_state(
                                &state_clone,
                                ControlInterfaceState::AgentDisconnected,
                            );
                            ControlInterface::send_initial_hello(&writer_ch_sender_clone).await;
                        }
                        sleep(Duration::from_millis(SLEEP_DURATION)).await;
                    }
                    Err(err) => {
                        log::error!("Error while reading from input fifo: '{err}'");
                        ControlInterface::change_state(
                            &state_clone,
                            ControlInterfaceState::Terminated,
                        );
                        break;
                    }
                }
            }

            Ok(())
        }));
    }

    /// Writes a request to the control interface.
    ///
    /// ## Arguments
    ///
    /// * `request` - A [`Request`] object to be sent.
    ///
    /// ## Returns
    ///
    /// An [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected.
    pub async fn write_request<T: Request + 'static>(
        &mut self,
        request: T,
    ) -> Result<(), AnkaiosError> {
        if *self.state.lock().unwrap_or_else(|_| unreachable!())
            != ControlInterfaceState::Initialized
        {
            log::error!("Could not write to pipe, not connected.");
            return Err(AnkaiosError::ControlInterfaceError(
                "Could not write to pipe, not connected.".to_owned(),
            ));
        }
        let message = ToAnkaios {
            to_ankaios_enum: Some(ToAnkaiosEnum::Request(request.to_proto())),
        };
        if let Some(sender) = self.writer_ch_sender.as_ref() {
            sender.send(message).await.unwrap_or_else(|err| {
                log::error!("Error while sending request: '{err}'");
            });
        }
        Ok(())
    }

    pub fn add_log_campaign_sender(
        &mut self,
        request_id: String,
        log_entries_sender: mpsc::Sender<LogResponse>,
    ) {
        self.request_id_to_logs_sender
            .lock()
            .unwrap_or_else(|_| unreachable!())
            .insert(request_id, log_entries_sender);
    }

    /// Prepares and sends a hello to the [Ankaios](https://eclipse-ankaios.github.io/ankaios) cluster.
    ///
    /// ## Arguments
    ///
    /// * `writer_ch_sender` - A sender for the writer channel.
    async fn send_initial_hello(writer_ch_sender: &mpsc::Sender<ToAnkaios>) {
        log::trace!("Sending initial hello message to the control interface.");
        let hello_msg = ToAnkaios {
            to_ankaios_enum: Some(ToAnkaiosEnum::Hello(Hello {
                protocol_version: ANKAIOS_VERSION.to_owned(),
            })),
        };
        writer_ch_sender
            .send(hello_msg)
            .await
            .unwrap_or_else(|err| {
                log::error!("Error while sending initial hello message: '{err}'");
            });
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
    use nix::{sys::stat::Mode, unistd::mkfifo};
    use prost::Message;
    use std::{sync::Arc, time::Duration};
    use tokio::{
        fs::OpenOptions,
        io::{AsyncWriteExt, BufReader, BufWriter},
        net::unix::pipe,
        spawn,
        sync::{mpsc, Barrier},
        time::{sleep, timeout as tokio_timeout},
    };

    use super::{
        read_protobuf_data, ControlInterface, ControlInterfaceState, ANKAIOS_INPUT_FIFO_PATH,
        ANKAIOS_OUTPUT_FIFO_PATH, ANKAIOS_VERSION,
    };
    use crate::ankaios::CHANNEL_SIZE;
    use crate::ankaios_api;
    use crate::components::{
        request::{generate_test_request, Request},
        response::{
            generate_test_proto_update_state_success, generate_test_response_update_state_success,
            Response,
        },
    };
    use ankaios_api::control_api::{to_ankaios::ToAnkaiosEnum, Hello, ToAnkaios};

    /// Helper function for getting the state of the control interface.
    fn get_state(ci: &ControlInterface) -> ControlInterfaceState {
        let state = ci.state.lock().unwrap();
        *state
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn utest_read_protobuf_data() {
        let tmpdir = tempfile::tempdir().unwrap();
        let fifo = tmpdir.path().join("fifo");
        mkfifo(&fifo, Mode::S_IRWXU).unwrap();

        let barrier1 = Arc::new(Barrier::new(2));
        let barrier2 = Arc::<Barrier>::clone(&barrier1);
        let fifo_clone = fifo.clone();
        let jh = spawn(async move {
            let mut file = tokio::io::BufReader::new(
                pipe::OpenOptions::new().open_receiver(&fifo_clone).unwrap(),
            );
            barrier1.wait().await;
            let data = read_protobuf_data(&mut file).await.unwrap();
            assert_eq!(data, vec![17]);
        });

        barrier2.wait().await; // Wait for the reader to start

        let mut f = pipe::OpenOptions::new().open_sender(&fifo).unwrap();
        let v = vec![1, 17];
        f.write_all(&v).await.unwrap();
        f.flush().await.unwrap();

        jh.await.unwrap();
    }

    #[test]
    fn utest_control_interface_state() {
        let mut cis = ControlInterfaceState::Initialized;
        assert_eq!(cis.to_string(), "Initialized");
        cis = ControlInterfaceState::Terminated;
        assert_eq!(cis.to_string(), "Terminated");
        cis = ControlInterfaceState::AgentDisconnected;
        assert_eq!(cis.to_string(), "AgentDisconnected");
        cis = ControlInterfaceState::ConnectionClosed;
        assert_eq!(cis.to_string(), "ConnectionClosed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn itest_control_interface() {
        // Crate mpsc channel
        let (response_sender, _response_receiver) = mpsc::channel::<Response>(CHANNEL_SIZE);

        // Prepare fifo pipes
        let tmpdir = tempfile::tempdir().unwrap();
        let fifo_input = tmpdir.path().join(ANKAIOS_INPUT_FIFO_PATH);
        let fifo_output = tmpdir.path().join(ANKAIOS_OUTPUT_FIFO_PATH);

        // Create control interface
        let mut ci = ControlInterface::new(response_sender);
        tmpdir.path().to_str().unwrap().clone_into(&mut ci.path);
        assert_eq!(get_state(&ci), ControlInterfaceState::Terminated);

        // Try to connect - should fail because the input fifo is not yet created
        assert!(ci.connect().await.is_err());
        mkfifo(&fifo_input, Mode::S_IRWXU).unwrap();

        // Try to connect - should fail because the output fifo is not yet created
        assert!(ci.connect().await.is_err());
        mkfifo(&fifo_output, Mode::S_IRWXU).unwrap();

        // Open the output file for reading
        let mut file_output = tokio::io::BufReader::new(
            pipe::OpenOptions::new()
                .open_receiver(&fifo_output)
                .unwrap(),
        );

        // Connect to the control interface - success
        ci.connect().await.unwrap();
        assert_eq!(get_state(&ci), ControlInterfaceState::Initialized);

        // Check that the initial hello was received
        #[allow(clippy::match_wild_err_arm)]
        match tokio_timeout(Duration::from_secs(1), read_protobuf_data(&mut file_output)).await {
            Ok(Ok(binary)) => {
                let to_ankaios = ToAnkaios::decode(&mut Box::new(binary.as_ref())).unwrap();
                assert_eq!(
                    to_ankaios.to_ankaios_enum,
                    Some(ToAnkaiosEnum::Hello(Hello {
                        protocol_version: ANKAIOS_VERSION.to_owned(),
                    }))
                );
            }
            Err(_) => panic!("Hello message was not sent"),
            _ => panic!("Error while reading pipe"),
        }

        // Try to connect again - should fail because it's already connected
        assert!(ci.connect().await.is_err());

        sleep(Duration::from_millis(50)).await;

        // Disconnect from the control interface
        ci.disconnect().unwrap();
        assert_eq!(get_state(&ci), ControlInterfaceState::Terminated);

        // Try to disconnect again - should fail
        assert!(ci.disconnect().is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn utest_control_interface_send_request() {
        // Crate mpsc channel
        let (response_sender, mut response_receiver) = mpsc::channel::<Response>(CHANNEL_SIZE);

        // Prepare fifo pipes
        let tmpdir = tempfile::tempdir().unwrap();
        let fifo_input = tmpdir.path().join(ANKAIOS_INPUT_FIFO_PATH);
        let fifo_output = tmpdir.path().join(ANKAIOS_OUTPUT_FIFO_PATH);

        // Open fifo pipes
        mkfifo(&fifo_input, Mode::S_IRWXU).unwrap();
        mkfifo(&fifo_output, Mode::S_IRWXU).unwrap();

        // Open the output file for reading
        let mut file_output = tokio::io::BufReader::new(
            pipe::OpenOptions::new()
                .open_receiver(&fifo_output)
                .unwrap(),
        );

        // Create control interface
        let mut ci = ControlInterface::new(response_sender);
        tmpdir.path().to_str().unwrap().clone_into(&mut ci.path);
        assert_eq!(get_state(&ci), ControlInterfaceState::Terminated);

        // Send dummy request - should fail
        assert!(ci.write_request(generate_test_request()).await.is_err());

        // Connect to the control interface
        ci.connect().await.unwrap();
        assert_eq!(get_state(&ci), ControlInterfaceState::Initialized);

        // Read the initial hello message
        let _ = tokio_timeout(Duration::from_secs(1), read_protobuf_data(&mut file_output))
            .await
            .unwrap();

        // Create sender to the input pipe
        sleep(Duration::from_millis(20)).await; // the receiver should be available first
        let mut file_input =
            BufWriter::new(pipe::OpenOptions::new().open_sender(&fifo_input).unwrap());

        // Generate and send request
        let req = generate_test_request();
        let req_proto = req.to_proto();
        let req_id = req.get_id();
        ci.write_request(req).await.unwrap();

        // Check that the request was sent
        #[allow(clippy::match_wild_err_arm)]
        match tokio_timeout(Duration::from_secs(1), read_protobuf_data(&mut file_output)).await {
            Ok(Ok(binary)) => {
                let to_ankaios = ToAnkaios::decode(&mut Box::new(binary.as_ref())).unwrap();
                assert_eq!(
                    to_ankaios.to_ankaios_enum,
                    Some(ToAnkaiosEnum::Request(req_proto))
                );
            }
            Err(_) => panic!("Request was not sent"),
            _ => panic!("Error while reading pipe"),
        }

        // Send response
        let response = generate_test_proto_update_state_success(req_id.clone());
        file_input
            .write_all(&response.encode_length_delimited_to_vec())
            .await
            .unwrap();
        file_input.flush().await.unwrap();

        // Check that the response was received
        let received_response = response_receiver.recv().await.unwrap();
        assert_eq!(received_response.id, req_id.clone());
        assert_eq!(
            received_response.content.to_string(),
            generate_test_response_update_state_success(req_id.clone())
                .content
                .to_string()
        );

        // Disconnect from the control interface
        ci.disconnect().unwrap();
        assert_eq!(get_state(&ci), ControlInterfaceState::Terminated);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn itest_control_interface_agent_disconnected() {
        // Crate mpsc channel
        let (response_sender, _) = mpsc::channel::<Response>(CHANNEL_SIZE);

        // Prepare fifo pipes
        let tmpdir = tempfile::tempdir().unwrap();
        let fifo_input = tmpdir.path().join(ANKAIOS_INPUT_FIFO_PATH);
        let fifo_input_clone = fifo_input.clone();
        let fifo_output = tmpdir.path().join(ANKAIOS_OUTPUT_FIFO_PATH);

        // Open fifo pipes
        mkfifo(&fifo_input, Mode::S_IRWXU).unwrap();
        mkfifo(&fifo_output, Mode::S_IRWXU).unwrap();
        let barrier = Arc::new(Barrier::new(2));

        // Open the output file for reading
        let file_output = BufReader::new(
            pipe::OpenOptions::new()
                .open_receiver(&fifo_output)
                .unwrap(),
        );

        // Spawn a writer task for the input file
        let writer_barrier = Arc::<Barrier>::clone(&barrier);
        tokio::spawn(async move {
            let writer = OpenOptions::new()
                .write(true)
                .open(fifo_input)
                .await
                .unwrap();

            writer_barrier.wait().await;
            drop(writer); // Closing the writer, EOF will be triggered in the reader
        });

        // Create control interface
        let mut ci = ControlInterface::new(response_sender);
        tmpdir.path().to_str().unwrap().clone_into(&mut ci.path);
        assert_eq!(get_state(&ci), ControlInterfaceState::Terminated);

        // Connect to the control interface
        sleep(Duration::from_millis(10)).await;
        ci.connect().await.unwrap();
        assert_eq!(get_state(&ci), ControlInterfaceState::Initialized);

        // Wait to ensure the reader gets to open the input pipe
        sleep(Duration::from_millis(20)).await;
        drop(file_output);
        barrier.wait().await;

        // Wait for the state to change
        sleep(Duration::from_millis(20)).await;
        assert_eq!(get_state(&ci), ControlInterfaceState::AgentDisconnected);

        // Recreate the output file for reading
        let _file_output = tokio::io::BufReader::new(
            pipe::OpenOptions::new()
                .open_receiver(&fifo_output)
                .unwrap(),
        );

        // Reconnect the input pipe
        let barrier_open = Arc::new(Barrier::new(2));
        let barrier_open_clone = Arc::<Barrier>::clone(&barrier_open);
        let barrier_close = Arc::new(Barrier::new(2));
        let barrier_close_clone = Arc::<Barrier>::clone(&barrier_close);
        tokio::spawn(async move {
            let writer = tokio::fs::OpenOptions::new()
                .write(true)
                .open(fifo_input_clone)
                .await
                .unwrap();

            barrier_open_clone.wait().await;
            // Wait for the state to change to initialized
            barrier_close_clone.wait().await;
            drop(writer);
        });
        // Wait for the writer to open the input pipe
        barrier_open.wait().await;

        // wait for the state to change
        tokio_timeout(Duration::from_secs(2), async {
            loop {
                if get_state(&ci) == ControlInterfaceState::Initialized {
                    break;
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("State is not initialized as it's supposed to be.");
        barrier_close.wait().await;

        // Disconnect from the control interface
        ci.disconnect().unwrap();
        assert_eq!(get_state(&ci), ControlInterfaceState::Terminated);
    }
}
