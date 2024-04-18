use std::{sync::mpsc::Sender, thread, time::Duration};

use hms2mqtt::{
    mqtt_config::MqttConfig,
    mqtt_wrapper::{self, PublishEvent},
};
use log::{debug, warn};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use rumqttc::{
    tokio_rustls::{self, rustls::ClientConfig},
    Client, Event, Incoming, MqttOptions, Transport,
};

pub struct RumqttcWrapper {
    client: Client,
}

// TODO: Is the a better way to implement Into or From for external stuff?
struct RumqttcQosWrapper(rumqttc::QoS);

impl Into<mqtt_wrapper::QoS> for RumqttcQosWrapper {
    fn into(self) -> mqtt_wrapper::QoS {
        match self.0 {
            rumqttc::QoS::AtMostOnce => mqtt_wrapper::QoS::AtMostOnce,
            rumqttc::QoS::AtLeastOnce => mqtt_wrapper::QoS::AtLeastOnce,
            rumqttc::QoS::ExactlyOnce => mqtt_wrapper::QoS::ExactlyOnce,
        }
    }
}

fn match_qos(qos: mqtt_wrapper::QoS) -> rumqttc::QoS {
    match qos {
        mqtt_wrapper::QoS::AtMostOnce => rumqttc::QoS::AtMostOnce,
        mqtt_wrapper::QoS::AtLeastOnce => rumqttc::QoS::AtLeastOnce,
        mqtt_wrapper::QoS::ExactlyOnce => rumqttc::QoS::ExactlyOnce,
    }
}

impl mqtt_wrapper::MqttWrapper for RumqttcWrapper {
    fn subscribe(&mut self, topic: &str, qos: mqtt_wrapper::QoS) -> anyhow::Result<()> {
        Ok(self.client.subscribe(topic, match_qos(qos))?)
    }

    fn publish<S, V>(
        &mut self,
        topic: S,
        qos: mqtt_wrapper::QoS,
        retain: bool,
        payload: V,
    ) -> anyhow::Result<()>
    where
        S: Clone + Into<String>,
        V: Clone + Into<Vec<u8>>,
    {
        self.client
            .publish(topic, match_qos(qos), retain, payload.clone())
            .unwrap();
        Ok(())
    }

    fn new(config: &MqttConfig, suffix: &str, pub_tx: Sender<PublishEvent>) -> Self {
        let use_tls = config.tls.is_some_and(|tls| tls);

        let mut mqttoptions = MqttOptions::new(
            &config.client_id,
            &config.host,
            config.port.unwrap_or_else(|| {
                if use_tls {
                    return 8883;
                }
                1883
            }),
        );
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        if use_tls {
            // Use rustls-native-certs to load root certificates from the operating system.
            let mut roots = tokio_rustls::rustls::RootCertStore::empty();
            rustls_native_certs::load_native_certs()
                .expect("could not load platform certs")
                .into_iter()
                .for_each(|cert| {
                    roots.add(cert).unwrap();
                });

            let client_config = ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();

            mqttoptions.set_transport(Transport::tls_with_config(client_config.into()));
        }

        //parse the mqtt authentication options
        if let Some((username, password)) = match (&config.username, &config.password) {
            (None, None) => None,
            (None, Some(_)) => None,
            (Some(username), None) => Some((username.clone(), "".into())),
            (Some(username), Some(password)) => Some((username.clone(), password.clone())),
        } {
            mqttoptions.set_credentials(username, password);
        }

        let (client, mut connection) = Client::new(mqttoptions, 512);

        thread::spawn(move || {
            // keep polling the event loop to make sure outgoing messages get sent
            // the call to .iter() blocks and suspends the thread effectively by
            // calling .recv() under the hood. This implies that the loop terminates
            // once the client unsubs
            for event in connection.iter().flatten() {
                if let Event::Incoming(Incoming::Publish(packet)) = event {
                    let pub_event = PublishEvent {
                        topic: packet.topic,
                        qos: RumqttcQosWrapper(packet.qos).into(),
                        retain: packet.retain,
                        payload: packet.payload,
                    };
                    pub_tx.send(pub_event).unwrap();
                }
            }
        });
        Self { client }
    }
}
