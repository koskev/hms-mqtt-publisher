use crate::protos::hoymiles::RealData::HMSStateResponse;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetworkState {
    Unknown,
    Online,
    Offline,
}

pub trait InverterRequest {
    fn get_cmd(&self) -> &'static [u8; 2];
}

pub trait Inverter {
    fn set_state(&mut self, new_state: NetworkState);
    // TODO: replace HMSStateResponse with generic response for any inverter
    fn update_state(&mut self) -> Option<HMSStateResponse>;
}
