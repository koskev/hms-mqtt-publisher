use crate::mqtt_config::MqttConfig;
use bytes::Bytes;
use std::sync::mpsc::Sender;

#[derive(Clone, Copy)]
pub enum QoS {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

pub struct PublishEvent {
    pub topic: String,
    pub qos: QoS,
    pub retain: bool,
    pub payload: Bytes,
}

// TODO: add an implementation of the MqttWrapper for testing
// TODO: should this be renamed to MqttImplementation?
pub trait MqttWrapper {
    // This trait provides an interface that the decouples library code from an
    // implementation of the MQTT client. On library calling code, one needs to
    // wrap the MQTT implementation, i.e. the client, in a new type that in
    // turn implements this trait.

    fn subscribe(&mut self, topic: &str, qos: QoS) -> anyhow::Result<()>;

    fn publish<S, V>(&mut self, topic: S, qos: QoS, retain: bool, payload: V) -> anyhow::Result<()>
    where
        S: Clone + Into<String>,
        V: Clone + Into<Vec<u8>>;

    fn new(config: &MqttConfig, pub_tx: Sender<PublishEvent>) -> Self;
}
