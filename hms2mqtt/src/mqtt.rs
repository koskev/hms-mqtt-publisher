use crate::{
    metric_collector::MetricCollector,
    mqtt_config::MqttConfig,
    mqtt_wrapper::{MqttWrapper, QoS},
    protos::hoymiles::RealData::HMSStateResponse,
};

use chrono::prelude::DateTime;
use chrono::Local;
use log::{debug, warn};
use std::{
    collections::HashMap,
    sync::mpsc::channel,
    time::{Duration, UNIX_EPOCH},
};

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

impl<MQTT: MqttWrapper> MetricCollector for Mqtt<MQTT> {
    fn publish(&mut self, hms_state: &HMSStateResponse) {
        debug!("{hms_state}");

        let d = UNIX_EPOCH + Duration::from_secs(hms_state.time as u64);
        let datetime = DateTime::<Local>::from(d);
        let inverter_local_time = datetime.format("%Y-%m-%d %H:%M:%S.%f").to_string();

        let mut topic_payload_pairs: HashMap<String, String> = HashMap::new();

        let pv_current_power = hms_state.pv_current_power as f32 / 10.;
        let pv_daily_yield = hms_state.pv_daily_yield;

        let serial = self
            .config
            .serial_alias
            .get(&hms_state.dtu_sn)
            .unwrap_or(&hms_state.dtu_sn);

        let base_topic = format!("{}/dtu/{}", self.config.base_topic, serial);

        // TODO: this section bears a lot of repetition. Investigate if there's a more idiomatic way to get the same result, perhaps using a macro
        topic_payload_pairs.insert(
            format!("{}/inverter_local_time", base_topic),
            inverter_local_time,
        );

        topic_payload_pairs.insert(
            format!("{}/current_power", base_topic),
            pv_current_power.to_string(),
        );
        topic_payload_pairs.insert(
            format!("{}/daily_yield", base_topic),
            pv_daily_yield.to_string(),
        );

        for inverter_state in &hms_state.inverter_state {
            let pv_grid_voltage = inverter_state.grid_voltage as f32 / 10.;
            let pv_grid_freq = inverter_state.grid_freq as f32 / 100.;
            let pv_inv_temperature = inverter_state.temperature as f32 / 10.;
            let base_topic = format!("{}/inverter/{}", base_topic, inverter_state.inv_id);

            topic_payload_pairs.insert(
                format!("{base_topic}/grid_voltage"),
                pv_grid_voltage.to_string(),
            );
            topic_payload_pairs.insert(format!("{base_topic}/grid_freq"), pv_grid_freq.to_string());
            topic_payload_pairs.insert(
                format!("{base_topic}/temperature"),
                pv_inv_temperature.to_string(),
            );
        }

        for port_state in hms_state.port_state.iter() {
            let pv_port_voltage = port_state.pv_vol as f32 / 10.;
            let pv_port_curr = port_state.pv_cur as f32 / 100.;
            let pv_port_power = port_state.pv_power as f32 / 10.;
            let pv_port_energy = port_state.pv_energy_total as f32;
            let pv_port_daily_yield = port_state.pv_daily_yield as f32;
            let base_topic = format!("{}/port/{}", base_topic, port_state.pv_port);
            topic_payload_pairs
                .insert(format!("{base_topic}/voltage"), pv_port_voltage.to_string());
            topic_payload_pairs.insert(format!("{base_topic}/curr"), pv_port_curr.to_string());
            topic_payload_pairs.insert(format!("{base_topic}/power"), pv_port_power.to_string());
            topic_payload_pairs.insert(format!("{base_topic}/energy"), pv_port_energy.to_string());
            topic_payload_pairs.insert(
                format!("{base_topic}/daily_yield"),
                pv_port_daily_yield.to_string(),
            );
        }

        topic_payload_pairs
            .into_iter()
            .for_each(|(topic, payload)| {
                debug!("Publishing to {} value: {}", topic, payload);
                if let Err(e) = self.client.publish(topic, QoS::AtMostOnce, true, payload) {
                    warn!("mqtt error: {e:?}")
                }
            });
    }
}
