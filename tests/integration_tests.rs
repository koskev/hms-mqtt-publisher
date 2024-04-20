use std::sync::mpsc::{channel, Sender};

use hms2mqtt::targets::mqtt::{
    mqtt_config::MqttConfig,
    mqtt_wrapper::{MqttWrapper, PublishEvent, QoS},
};

struct MqttTester {
    published_values: Vec<(String, Vec<u8>)>,
}

impl MqttTester {
    pub fn len(&self) -> usize {
        self.published_values.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl MqttWrapper for MqttTester {
    fn subscribe(&mut self, _topic: &str, _qos: QoS) -> anyhow::Result<()> {
        Ok(())
    }

    fn publish<S, V>(
        &mut self,
        topic: S,
        _qos: QoS,
        _retain: bool,
        payload: V,
    ) -> anyhow::Result<()>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        self.published_values.push((topic.into(), payload.into()));
        Ok(())
    }

    fn new(_config: &MqttConfig, _tx: Sender<PublishEvent>) -> Self {
        Self {
            published_values: Vec::new(),
        }
    }
}

#[test]
fn publish_one_message() {
    let (tx, _rx) = channel();
    let mut mqtt = MqttTester::new(
        &MqttConfig {
            host: "frob".to_owned(),
            port: Some(1234),
            username: None,
            password: None,
            tls: None,
            client_id: "hms_test".into(),
            base_topic: "hms".into(),
            ..Default::default()
        },
        tx,
    );
    let result = mqtt.publish("hms/foo", QoS::AtMostOnce, true, "Hooray".to_string());
    assert!(result.is_ok());
    assert!(!mqtt.is_empty());
    assert_eq!(mqtt.len(), 1);
}
