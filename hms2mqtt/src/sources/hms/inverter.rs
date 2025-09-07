use crate::protos::hoymiles::RealData::{HMSStateResponse, RealDataResDTO};
use crate::sources::inverter::{Inverter, InverterRequest, NetworkState};
use crc16::{State, MODBUS};
use log::{debug, error, info, warn};
use protobuf::Message;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

static INVERTER_PORT: &str = "10081";

const CMD_HEADER: &[u8; 2] = b"HM";
const CMD_GET_DATA: &[u8; 2] = b"\xa3\x03";

impl InverterRequest for RealDataResDTO {
    fn get_cmd(&self) -> &'static [u8; 2] {
        CMD_GET_DATA
    }
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
        let request = RealDataResDTO::default();

        self.send_request(request)
    }
}

impl HMSStateResponse {
    pub fn get_topics(
        &self,
        prefix: Option<&str>,
        aliases: &HashMap<String, String>,
    ) -> HashMap<String, f32> {
        let mut topic_payload_pairs: HashMap<String, f32> = HashMap::new();

        let pv_current_power = self.pv_current_power as f32 / 10.;
        let pv_daily_yield = self.pv_daily_yield;

        let serial = aliases.get(&self.dtu_sn).unwrap_or(&self.dtu_sn);

        let base_topic = if let Some(prefix) = prefix {
            format!("{}/dtu/{}", prefix, serial)
        } else {
            format!("dtu/{}", serial)
        };

        topic_payload_pairs.insert(
            format!("{}/inverter_local_time", base_topic),
            self.time as f32,
        );

        topic_payload_pairs.insert(format!("{}/current_power", base_topic), pv_current_power);
        topic_payload_pairs.insert(format!("{}/daily_yield", base_topic), pv_daily_yield as f32);

        for inverter_state in &self.inverter_state {
            let pv_grid_voltage = inverter_state.grid_voltage as f32 / 10.;
            let pv_grid_freq = inverter_state.grid_freq as f32 / 100.;
            let pv_inv_temperature = inverter_state.temperature as f32 / 10.;
            let base_topic = format!("{}/inverter/{}", base_topic, inverter_state.inv_id);

            topic_payload_pairs.insert(format!("{base_topic}/grid_voltage"), pv_grid_voltage);
            topic_payload_pairs.insert(format!("{base_topic}/grid_freq"), pv_grid_freq);
            topic_payload_pairs.insert(format!("{base_topic}/temperature"), pv_inv_temperature);
        }

        for port_state in self.port_state.iter() {
            let pv_port_voltage = port_state.pv_vol as f32 / 10.;
            let pv_port_curr = port_state.pv_cur as f32 / 100.;
            let pv_port_power = port_state.pv_power as f32 / 10.;
            let pv_port_energy = port_state.pv_energy_total as f32;
            let pv_port_daily_yield = port_state.pv_daily_yield as f32;
            let base_topic = format!("{}/port/{}", base_topic, port_state.pv_port);
            topic_payload_pairs.insert(format!("{base_topic}/voltage"), pv_port_voltage);
            topic_payload_pairs.insert(format!("{base_topic}/curr"), pv_port_curr);
            topic_payload_pairs.insert(format!("{base_topic}/power"), pv_port_power);
            topic_payload_pairs.insert(format!("{base_topic}/energy"), pv_port_energy);
            topic_payload_pairs.insert(format!("{base_topic}/daily_yield"), pv_port_daily_yield);
        }
        topic_payload_pairs
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
        self.sequence = self.sequence.wrapping_add(1);
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
            error!("could not connect: {e}");
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
}
