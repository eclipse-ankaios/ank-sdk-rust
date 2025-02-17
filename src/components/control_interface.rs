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

use std::{
    sync::{Arc, Mutex},
    path::Path
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, Error, ErrorKind}, net::unix::pipe, spawn, sync::mpsc, time::{sleep, Duration}
};
use prost::Message;

use api::control_api::{
    to_ankaios::ToAnkaiosEnum,
    FromAnkaios, Hello, ToAnkaios,
};
use crate::AnkaiosError;
use crate::components::request::Request;
use crate::components::response::{Response, ResponseType};

const ANKAIOS_CONTROL_INTERFACE_BASE_PATH: &str = "/run/ankaios/control_interface";
const ANKAIOS_VERSION: &str = "0.5.0";
const MAX_VARINT_SIZE: usize = 19;


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum ControlInterfaceState {
    Initialized = 1,
    Terminated = 2,
    AgentDisconnected = 3,
    ConnectionClosed = 4,
}

pub struct ControlInterface{
    pub path: String,
    output_file: Option<pipe::Sender>,
    read_thread_handler: Option<tokio::task::JoinHandle<Result<(), AnkaiosError>>>,
    writer_thread_handler: Option<tokio::task::JoinHandle<Result<(), AnkaiosError>>>,
    state: Arc<Mutex<ControlInterfaceState>>,
    response_sender: mpsc::Sender<Response>,
    writer_ch_sender: Option<mpsc::Sender<ToAnkaios>>,
}

async fn read_varint_data(file: &mut BufReader<pipe::Receiver>) -> Result<[u8; MAX_VARINT_SIZE], Error> {
    let mut res = [0u8; MAX_VARINT_SIZE];
    for item in res.iter_mut() {
        *item = file.read_u8().await?;
        if *item & 0b10000000 == 0 {
            break;
        }
    }
    Ok(res)
}

async fn read_protobuf_data(file: &mut BufReader<pipe::Receiver>) -> Result<Vec<u8>, Error> {
    let varint_data = read_varint_data(file).await?;
    let mut varint_data = Box::new(&varint_data[..]);

    let size = prost::encoding::decode_varint(&mut varint_data)? as usize;

    let mut buf = vec![0; size];
    file.read_exact(&mut buf[..]).await?;
    Ok(buf)
}

impl std::fmt::Display for ControlInterfaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res_str = match self {
            ControlInterfaceState::Initialized => "Initialized",
            ControlInterfaceState::Terminated => "Terminated",
            ControlInterfaceState::AgentDisconnected => "AgentDisconnected",
            ControlInterfaceState::ConnectionClosed => "ConnectionClosed",
        };
        write!(f, "{}", res_str)
    }
}

impl ControlInterface {
    pub fn new(response_sender: mpsc::Sender<Response>) -> Self {
        Self{
            path: ANKAIOS_CONTROL_INTERFACE_BASE_PATH.to_string(),
            output_file: None,
            read_thread_handler: None,
            writer_thread_handler: None,
            state: Arc::new(Mutex::new(ControlInterfaceState::Terminated)),
            response_sender,
            writer_ch_sender: None,
        }
    }

    pub fn state(&self) -> ControlInterfaceState {
        let state = self.state.lock().unwrap();
        *state
    }

    pub async fn connect(&mut self) -> Result<(), AnkaiosError> {
        if *self.state.lock().unwrap() == ControlInterfaceState::Initialized {
            return Err(AnkaiosError::ControlInterfaceError("Already connected.".to_string()));
        }
        if std::fs::metadata(&(self.path.clone() + "/input")).is_err() {
            return Err(AnkaiosError::ControlInterfaceError("Control interface input fifo does not exist.".to_string()));
        }
        if std::fs::metadata(&(self.path.clone() + "/output")).is_err() {
            return Err(AnkaiosError::ControlInterfaceError("Control interface output fifo does not exist.".to_string()));
        }

        self.prepare_writer();
        self.read_from_control_interface();
        ControlInterface::change_state(&self.state, ControlInterfaceState::Initialized).await;
        ControlInterface::send_initial_hello(self.writer_ch_sender.as_ref().unwrap()).await;

        log::trace!("Connected to the control interface.");
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), AnkaiosError> {
        if *self.state.lock().unwrap() != ControlInterfaceState::Initialized {
            return Err(AnkaiosError::ControlInterfaceError("Already disconnected.".to_string()));
        }
        if let Some(handler) = self.read_thread_handler.take() {
            handler.abort();
        }
        self.state.lock().unwrap().clone_from(&ControlInterfaceState::Terminated);
        self.output_file = None;
        Ok(())
    }

    async fn change_state(state: &Arc<Mutex<ControlInterfaceState>>, new_state: ControlInterfaceState) {
        if *state.lock().unwrap() == new_state {
            return;
        }
        state.lock().unwrap().clone_from(&new_state);
        log::info!("State changed: {:?}", new_state);
    }

    fn prepare_writer(&mut self) {
        let (writer_ch_sender, mut writer_ch_receiver) = mpsc::channel::<ToAnkaios>(5);
        self.writer_ch_sender = Some(writer_ch_sender.clone());
        let output_path = Path::new(&self.path).to_path_buf().join("output");
        let state_clone = self.state.clone();
        self.writer_thread_handler = Some(spawn(async move {
            const AGENT_RECONNECT_INTERVAL: u64 = 1;
            let mut output_file = BufWriter::new(
                pipe::OpenOptions::new().open_sender(output_path).unwrap()
            );

            while let Some(message) = writer_ch_receiver.recv().await {
                output_file.write_all(&message.encode_length_delimited_to_vec()).await.unwrap_or_else(|err| {
                    log::error!("Error while writing to output fifo: '{}'", err);
                    // let _ = self.disconnect();
                });
                if let Err(err) = output_file.flush().await {
                    if err.kind() == ErrorKind::BrokenPipe {
                        if *state_clone.lock().unwrap() == ControlInterfaceState::Initialized {
                            ControlInterface::change_state(&state_clone, ControlInterfaceState::AgentDisconnected).await;
                        }
                        log::warn!("Waiting for the agent..");
                        sleep(Duration::from_secs(AGENT_RECONNECT_INTERVAL)).await;
                        ControlInterface::send_initial_hello(&writer_ch_sender).await;
                    } else {
                        log::error!("Error while flushing to output fifo: '{}'", err);
                        // let _ = self.disconnect();
                    }
                } else if *state_clone.lock().unwrap() == ControlInterfaceState::AgentDisconnected {
                    ControlInterface::change_state(&state_clone, ControlInterfaceState::Initialized).await;
                }
            }
            Ok(())
        }));
    }

    fn read_from_control_interface(&mut self) {
        let input_path = Path::new(&self.path).to_path_buf().join("input");
        let response_sender_clone = self.response_sender.clone();
        let writer_ch_sender_clone = self.writer_ch_sender.as_ref().unwrap().clone();
        let state_clone = self.state.clone();
        self.read_thread_handler = Some(spawn(async move {
            let mut input_file = BufReader::new(
                pipe::OpenOptions::new().open_receiver(input_path).unwrap()
            );

            loop {
                match read_protobuf_data(&mut input_file).await{
                    Ok(binary) => {
                        if *state_clone.lock().unwrap() == ControlInterfaceState::AgentDisconnected {
                            log::info!("Agent reconnected successfully.");
                            ControlInterface::change_state(&state_clone, ControlInterfaceState::Initialized).await;
                        }

                        match FromAnkaios::decode(&mut Box::new(binary.as_ref())) {
                            Ok(from_ankaios) => {
                                let received_response = Response::new(from_ankaios);
                                let is_con_closed = matches!(received_response.content, ResponseType::ConnectionClosedReason(_));
                                response_sender_clone.send(received_response).await.unwrap_or_else(|err| {
                                    log::error!("Error while sending response: '{}'", err);
                                });
                                if is_con_closed {
                                    log::error!("Connection closed by the agent.");
                                    break;
                                }
                            },
                            Err(err) => log::error!("Invalid response, parsing error: '{}'", err),
                        }
                    },
                    Err(err) if err.kind() == ErrorKind::UnexpectedEof => {
                        if *state_clone.lock().unwrap() == ControlInterfaceState::Initialized {
                            ControlInterface::change_state(&state_clone, ControlInterfaceState::AgentDisconnected).await;
                            ControlInterface::send_initial_hello(&writer_ch_sender_clone).await;
                        }
                        sleep(Duration::from_millis(500)).await;
                    }
                    Err(err) => {
                        log::error!("Error while reading from input fifo: '{}'", err);
                        ControlInterface::change_state(&state_clone, ControlInterfaceState::Terminated).await;
                        break;
                    }
                }
            }

            Ok(())
        }));
    }

    pub async fn write_request(&mut self, request: Request) -> Result<(), AnkaiosError> {
        if *self.state.lock().unwrap() != ControlInterfaceState::Initialized {
            log::error!("Could not write to pipe, not connected.");
            return Err(AnkaiosError::ControlInterfaceError("Could not write to pipe, not connected.".to_string()));
        }
        let message = ToAnkaios {
            to_ankaios_enum: Some(ToAnkaiosEnum::Request(request.to_proto())),
        };
        self.writer_ch_sender.as_ref().unwrap().send(message).await.unwrap();
        Ok(())
    }

    async fn send_initial_hello(writer_ch_sender: &mpsc::Sender<ToAnkaios>) {
        log::trace!("Sending initial hello message to the control interface.");
        let hello_msg = ToAnkaios {
            to_ankaios_enum: Some(ToAnkaiosEnum::Hello(Hello {
                protocol_version: ANKAIOS_VERSION.to_string(),
            })),
        };
        writer_ch_sender.send(hello_msg).await.unwrap();
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
    use std::{
        sync::Arc,
        time::Duration,
    };
    use tokio::{
        sync::{mpsc, Barrier},
        io::AsyncWriteExt,
        fs::File,
        net::unix::pipe,
        time::{sleep, timeout as tokio_timeout},
        spawn,
    };
    use prost::Message;
    use nix::{
        sys::stat::Mode,
        unistd::mkfifo,
    };

    use api::control_api::{
        to_ankaios::ToAnkaiosEnum,
        Hello, ToAnkaios,
    };
    use super::{ControlInterface, ControlInterfaceState, read_protobuf_data, ANKAIOS_VERSION};
    use crate::components::{
        response::{Response, generate_test_proto_update_state_success, generate_test_response_update_state_success},
        request::generate_test_request,
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn utest_read_protobuf_data() {
        let tmpdir = tempfile::tempdir().unwrap();
        let fifo = tmpdir.path().join("fifo");
        mkfifo(&fifo, Mode::S_IRWXU).unwrap();

        let barrier1 = Arc::new(Barrier::new(2));
        let barrier2 = barrier1.clone();
        let fifo_clone = fifo.clone();
        let jh = spawn(async move {
            let mut file = tokio::io::BufReader::new(
                pipe::OpenOptions::new().open_receiver(&fifo_clone).unwrap()
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
        let (response_sender, _response_receiver) = mpsc::channel::<Response>(100);

        // Prepare fifo pipes
        let tmpdir = tempfile::tempdir().unwrap();
        let fifo_input = tmpdir.path().join("input");
        let fifo_output = tmpdir.path().join("output");

        // Create control interface
        let mut ci = ControlInterface::new(response_sender);
        ci.path = tmpdir.path().to_str().unwrap().to_string();
        assert_eq!(ci.state(), ControlInterfaceState::Terminated);

        // Try to connect - should fail because the input fifo is not yet created
        assert!(ci.connect().await.is_err());
        mkfifo(&fifo_input, Mode::S_IRWXU).unwrap();

        // Try to connect - should fail because the output fifo is not yet created
        assert!(ci.connect().await.is_err());
        mkfifo(&fifo_output, Mode::S_IRWXU).unwrap();

        // Try to connect - should fail because the pipe receiver is not open
        assert!(ci.connect().await.is_err());

        // Open the output file for reading
        let mut file_output = tokio::io::BufReader::new(
            pipe::OpenOptions::new().open_receiver(&fifo_output).unwrap()
        );

        // Connect to the control interface - success
        ci.connect().await.unwrap();
        assert_eq!(ci.state(), ControlInterfaceState::Initialized);

        // Check that the initial hello was received
        match tokio_timeout(Duration::from_secs(1), read_protobuf_data(&mut file_output)).await {
            Ok(Ok(binary)) => {
                let to_ankaios = ToAnkaios::decode(&mut Box::new(binary.as_ref())).unwrap();
                assert_eq!(to_ankaios.to_ankaios_enum, Some(ToAnkaiosEnum::Hello(Hello {
                    protocol_version: ANKAIOS_VERSION.to_string(),
                })));
            },
            Err(_) => panic!("Hello message was not sent"),
            _ => panic!("Error while reading pipe"),
        }

        // Try to connect again - should fail because it's already connected
        assert!(ci.connect().await.is_err());

        sleep(Duration::from_millis(50)).await;

        // Disconnect from the control interface
        ci.disconnect().unwrap();
        assert_eq!(ci.state(), ControlInterfaceState::Terminated);

        // Try to disconnect again - should fail
        assert!(ci.disconnect().is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn utest_control_interface_send_request() {
        // Crate mpsc channel
        let (response_sender, mut response_receiver) = mpsc::channel::<Response>(100);

        // Prepare fifo pipes
        let tmpdir = tempfile::tempdir().unwrap();
        let fifo_input = tmpdir.path().join("input");
        let fifo_output = tmpdir.path().join("output");

        // Open fifo pipes
        mkfifo(&fifo_input, Mode::S_IRWXU).unwrap();
        mkfifo(&fifo_output, Mode::S_IRWXU).unwrap();

        // Open the output file for reading
        let mut file_output = tokio::io::BufReader::new(
            pipe::OpenOptions::new().open_receiver(&fifo_output).unwrap()
        );

        // Create control interface
        let mut ci = ControlInterface::new(response_sender);
        ci.path = tmpdir.path().to_str().unwrap().to_string();
        assert_eq!(ci.state(), ControlInterfaceState::Terminated);

        // Send dummy request - should just return
        ci.write_request(generate_test_request()).await.unwrap();

        // Connect to the control interface
        ci.connect().await.unwrap();
        assert_eq!(ci.state(), ControlInterfaceState::Initialized);

        // Read the initial hello message
        let _ = tokio_timeout(Duration::from_secs(1), read_protobuf_data(&mut file_output)).await.unwrap();

        // Create sender to the input pipe
        sleep(Duration::from_millis(20)).await; // the receiver should be available first
        let mut file_input = tokio::io::BufWriter::new(
            pipe::OpenOptions::new().open_sender(&fifo_input).unwrap()
        );

        // Generate and send request
        let req = generate_test_request();
        let req_proto = req.to_proto();
        let req_id = req.get_id();
        ci.write_request(req).await.unwrap();

        // Check that the request was sent
        match tokio_timeout(Duration::from_secs(1), read_protobuf_data(&mut file_output)).await {
            Ok(Ok(binary)) => {
                let to_ankaios = ToAnkaios::decode(&mut Box::new(binary.as_ref())).unwrap();
                assert_eq!(to_ankaios.to_ankaios_enum, Some(
                    ToAnkaiosEnum::Request(req_proto)
                ));
            },
            Err(_) => panic!("Request was not sent"),
            _ => panic!("Error while reading pipe"),
        }

        // Send response
        let response = generate_test_proto_update_state_success(req_id.clone());
        file_input.write_all(&response.encode_length_delimited_to_vec()).await.unwrap();
        file_input.flush().await.unwrap();

        // Check that the response was received
        let received_response = response_receiver.recv().await.unwrap();
        assert_eq!(received_response.id, req_id.clone());
        assert_eq!(received_response.content.to_string(), generate_test_response_update_state_success(req_id.clone()).content.to_string());

        // Disconnect from the control interface
        ci.disconnect().unwrap();
        assert_eq!(ci.state(), ControlInterfaceState::Terminated);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn utest_control_interface_agent_disconnected() {
        // Crate mpsc channel
        let (response_sender, mut _response_receiver) = mpsc::channel::<Response>(100);

        // Prepare fifo pipes
        let tmpdir = tempfile::tempdir().unwrap();
        let fifo_input = tmpdir.path().join("input");
        let fifo_output = tmpdir.path().join("output");

        // Open fifo pipes
        mkfifo(&fifo_input, Mode::S_IRWXU).unwrap();
        mkfifo(&fifo_output, Mode::S_IRWXU).unwrap();

        // Open the output file for reading
        let mut _file_output = tokio::io::BufReader::new(
            pipe::OpenOptions::new().open_receiver(&fifo_output).unwrap()
        );

        // Create control interface
        let mut ci = ControlInterface::new(response_sender);
        ci.path = tmpdir.path().to_str().unwrap().to_string();
        assert_eq!(ci.state(), ControlInterfaceState::Terminated);

        // Connect to the control interface
        ci.connect().await.unwrap();
        assert_eq!(ci.state(), ControlInterfaceState::Initialized);

        // Open the FIFO for writing, simulating an agent
        let mut file_input = File::create(&fifo_input).await.unwrap();

        // Write some data and flush it to ensure the pipe is in use
        file_input.write_all(b"Test Data").await.unwrap();
        file_input.flush().await.unwrap();

        // Explicitly close the file descriptor to simulate agent disconnection
        drop(file_input);
        sleep(Duration::from_millis(50)).await; // Allow time for detection

        // Disconnect from the control interface
        // ci.disconnect().unwrap();
        // assert_eq!(ci.state(), ControlInterfaceState::Terminated);
    }
}