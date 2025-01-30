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
    path::Path,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, Error},
    time::{sleep, Duration},
    net::unix::pipe,
    sync::mpsc,
    spawn,
};
use prost::Message;

use api::control_api::{
    to_ankaios::ToAnkaiosEnum,
    FromAnkaios, Hello, ToAnkaios,
};
use crate::AnkaiosError;
use crate::components::request::Request;
use crate::components::response::Response;

const ANKAIOS_CONTROL_INTERFACE_BASE_PATH: &str = "/run/ankaios/control_interface";
const ANKAIOS_VERSION: &str = "0.5.0";
const MAX_VARINT_SIZE: usize = 19;


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
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
    state: Arc<Mutex<ControlInterfaceState>>,
    response_sender: mpsc::Sender<Response>,
    state_changed_sender: mpsc::Sender<ControlInterfaceState>,
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
    pub fn new(response_sender: mpsc::Sender<Response>, state_changed_sender: mpsc::Sender<ControlInterfaceState>) -> Self {
        //Arc::new(Mutex::new())
        Self{
            path: ANKAIOS_CONTROL_INTERFACE_BASE_PATH.to_string(),
            output_file: None,
            read_thread_handler: None,
            state: Arc::new(Mutex::new(ControlInterfaceState::Terminated)),
            response_sender,
            state_changed_sender,
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

        let output_path = Path::new(&self.path).to_path_buf().join("output");
        self.output_file = match pipe::OpenOptions::new().open_sender(output_path) {
            Ok(file) => Some(file),
            Err(err) => {
                log::error!("Error while opening output fifo: {}", err);
                return Err(AnkaiosError::ControlInterfaceError(format!("Error while opening output fifo: {}", err)));
            }
        };

        self.read_from_control_interface();
        self.change_state(ControlInterfaceState::Initialized).await;
        self.send_initial_hello().await;

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

    pub async fn change_state(&mut self, new_state: ControlInterfaceState) {
        self.state.lock().unwrap().clone_from(&new_state);
        self.state_changed_sender.send(new_state).await.unwrap_or_else(|err| {
            log::error!("Error while sending state change: '{}'", err);
        });
    }

    fn read_from_control_interface(&mut self) {
        let input_path = Path::new(&self.path).to_path_buf().join("input");
        let response_sender_clone = self.response_sender.clone();
        self.read_thread_handler = Some(spawn(async move {
            let mut input_file = BufReader::new(
                pipe::OpenOptions::new().open_receiver(input_path).unwrap()
            );

            loop {
                match read_protobuf_data(&mut input_file).await{
                    Ok(binary) => {
                        match FromAnkaios::decode(&mut Box::new(binary.as_ref())) {
                            Ok(from_ankaios) => {
                                let received_response = Response::new(from_ankaios);
                                response_sender_clone.send(received_response).await.unwrap_or_else(|err| {
                                    log::error!("Error while sending response: '{}'", err);
                                });
                            },
                            Err(err) => log::error!("Invalid response, parsing error: '{}'", err),
                        }
                    },
                    Err(err) => {
                        // TODO Agent disconnected?
                        log::error!("Error while reading from input fifo: '{}'", err);
                        break;
                    }
                }
            }

            Ok(())
        }));
    }

    #[allow(dead_code)]
    async fn agent_gone_routine(&mut self) {
        const AGENT_RECONNECT_INTERVAL: u64 = 1;
        while *self.state.lock().unwrap() == ControlInterfaceState::AgentDisconnected {
            sleep(Duration::from_secs(AGENT_RECONNECT_INTERVAL)).await;
            // TODO
        }
    }

    async fn write_to_pipe(&mut self, message: ToAnkaios) {
        match self.output_file {
            Some(ref mut file) => {
                file.write_all(&message.encode_length_delimited_to_vec()).await.unwrap_or_else(|err| {
                    log::error!("Error while writing to output fifo: '{}'", err);
                    let _ = self.disconnect();
                });
            }
            None => {
                log::error!("Could not write to pipe, output file handler is not available.");
            }
        };
    }

    pub async fn write_request(&mut self, request: Request) {
        if *self.state.lock().unwrap() != ControlInterfaceState::Initialized {
            log::error!("Control interface is not initialized.");
            return;
        }
        let message = ToAnkaios {
            to_ankaios_enum: Some(ToAnkaiosEnum::Request(request.to_proto())),
        };
        self.write_to_pipe(message).await;
    }

    async fn send_initial_hello(&mut self) {
        log::trace!("Sending initial hello message to the control interface.");
        let hello_msg = ToAnkaios {
            to_ankaios_enum: Some(ToAnkaiosEnum::Hello(Hello {
                protocol_version: ANKAIOS_VERSION.to_string(),
            })),
        };
        self.write_to_pipe(hello_msg).await;
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
    use tokio::sync::mpsc;
    use super::{ControlInterface, ControlInterfaceState};
    use crate::components::response::Response;

    #[test]
    fn test_control_interface() {
        let _ = ControlInterface::new(
            mpsc::channel::<Response>(100).0,
            mpsc::channel::<ControlInterfaceState>(100).0,
        );
    }
}