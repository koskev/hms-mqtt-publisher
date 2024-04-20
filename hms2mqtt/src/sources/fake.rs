use crate::protos::hoymiles::RealData::HMSStateResponse;

use super::inverter::{Inverter, NetworkState};

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
