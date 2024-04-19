use crate::protos::hoymiles::RealData::{HMSStateResponse, RealDataResDTO};
use crc16::{State, MODBUS};
use log::{debug, error, info, warn};
use protobuf::Message;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

static INVERTER_PORT: &str = "10081";

const CMD_HEADER: &[u8; 2] = b"HM";
const CMD_GET_DATA: &[u8; 2] = b"\xa3\x03";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetworkState {
    Unknown,
    Online,
    Offline,
}

trait InverterRequest {
    fn get_cmd(&self) -> &'static [u8; 2];
}

impl InverterRequest for RealDataResDTO {
    fn get_cmd(&self) -> &'static [u8; 2] {
        CMD_GET_DATA
    }
}

pub trait Inverter {
    fn set_state(&mut self, new_state: NetworkState);
    // TODO: replace HMSStateResponse with generic response for any inverter
    fn update_state(&mut self) -> Option<HMSStateResponse>;
}

pub struct HMSInverter<'a> {
    host: &'a str,
    state: NetworkState,
    sequence: u16,
}

impl<'a> Inverter for HMSInverter<'a> {
    fn set_state(&mut self, new_state: NetworkState) {
        if self.state != new_state {
            self.state = new_state;
            info!("Inverter is {new_state:?}");
        }
    }

    fn update_state(&mut self) -> Option<HMSStateResponse> {
        self.sequence = self.sequence.wrapping_add(1);

        let request = RealDataResDTO::default();

        self.send_request(request)
    }
}

impl<'a> HMSInverter<'a> {
    pub fn new(host: &'a str) -> Self {
        Self {
            host,
            state: NetworkState::Unknown,
            sequence: 0_u16,
        }
    }

    fn send_request<REQ, RES>(&mut self, request: REQ) -> Option<RES>
    where
        REQ: Message + InverterRequest,
        RES: Message,
    {
        let request_as_bytes = request.write_to_bytes().expect("serialize to bytes");
        let crc16 = State::<MODBUS>::calculate(&request_as_bytes);
        let len = request_as_bytes.len() as u16 + 10u16;
        // compose request message
        let mut message = Vec::new();
        message.extend_from_slice(CMD_HEADER);
        message.extend_from_slice(request.get_cmd());
        message.extend_from_slice(&self.sequence.to_be_bytes());
        message.extend_from_slice(&crc16.to_be_bytes());
        message.extend_from_slice(&len.to_be_bytes());
        message.extend_from_slice(&request_as_bytes);

        let inverter_host = self.host.to_string() + ":" + INVERTER_PORT;
        let address = match inverter_host.to_socket_addrs() {
            Ok(mut a) => a.next(),
            Err(e) => {
                debug!("Unable to resolve domain: {e}");
                return None;
            }
        };
        if address.is_none() {
            error!("Unable to parse name");
            return None;
        }

        let stream = TcpStream::connect_timeout(&address.unwrap(), Duration::from_millis(500));
        if let Err(e) = stream {
            debug!("could not connect: {e}");
            self.set_state(NetworkState::Offline);
            return None;
        }

        let mut stream = stream.unwrap();
        if let Err(e) = stream.set_write_timeout(Some(Duration::new(5, 0))) {
            warn!("could not set write timeout: {e}");
        }
        if let Err(e) = stream.set_read_timeout(Some(Duration::new(5, 0))) {
            warn!("could not set read timeout: {e}");
        }
        if let Err(e) = stream.write(&message) {
            debug!(r#"{e}"#);
            self.set_state(NetworkState::Offline);
            return None;
        }

        let mut buf = [0u8; 1024];
        let read = stream.read(&mut buf);

        if let Err(e) = read {
            debug!("{e}");
            self.set_state(NetworkState::Offline);
            return None;
        }
        let read_length = read.unwrap();
        let parsed = RES::parse_from_bytes(&buf[10..read_length]);

        match parsed {
            Ok(parsed) => {
                self.set_state(NetworkState::Online);
                Some(parsed)
            }
            Err(e) => {
                debug!("{e}");
                self.set_state(NetworkState::Offline);
                None
            }
        }
    }

    pub fn update_state(&mut self) -> Option<HMSStateResponse> {
        self.sequence = self.sequence.wrapping_add(1);

        let request = RealDataResDTO::default();

        self.send_request(request)
    }
}

pub struct FakeInverter {
    pub sn: String,
}

impl Inverter for FakeInverter {
    fn set_state(&mut self, _new_state: NetworkState) {}

    fn update_state(&mut self) -> Option<HMSStateResponse> {
        let resp = HMSStateResponse {
            dtu_sn: self.sn.clone(),
            ..Default::default()
        };

        Some(resp)
    }
}
