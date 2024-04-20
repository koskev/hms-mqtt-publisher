use crate::protos::hoymiles::RealData::HMSStateResponse;

pub trait MetricPublisher {
    fn publish(&mut self, hms_state: &HMSStateResponse);
}
