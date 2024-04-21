use crate::{
    protos::hoymiles::RealData::HMSStateResponse,
    targets::{
        metric_publisher::MetricPublisher,
        mqtt::{
            mqtt_config::MqttConfig,
            mqtt_wrapper::{MqttWrapper, QoS},
        },
    },
};

use log::{debug, warn};
use std::sync::mpsc::channel;

pub struct Mqtt<MQTT: MqttWrapper> {
    client: MQTT,
    config: MqttConfig,
}

impl<MQTT: MqttWrapper> Mqtt<MQTT> {
    pub fn new(config: &MqttConfig) -> Self {
        let (tx, _rx) = channel();
        let client = MQTT::new(config, tx);
        Self {
            client,
            config: config.clone(),
        }
    }
}

impl<MQTT: MqttWrapper> MetricPublisher for Mqtt<MQTT> {
    fn publish(&mut self, hms_state: &HMSStateResponse) {
        let topic_payload_pairs =
            hms_state.get_topics(Some(&self.config.base_topic), &self.config.serial_aliases);

        topic_payload_pairs
            .into_iter()
            .for_each(|(topic, payload)| {
                debug!("Publishing to {} value: {}", topic, payload);
                if let Err(e) =
                    self.client
                        .publish(topic, QoS::AtMostOnce, true, payload.to_string())
                {
                    warn!("mqtt error: {e:?}")
                }
            });
    }
}
