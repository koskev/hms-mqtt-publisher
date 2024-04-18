use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_derive::Deserialize;

fn default_topic() -> String {
    "hms800wt2".into()
}

fn default_client_id() -> String {
    format!(
        "hms-mqtt-{}",
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(5)
            .map(char::from)
            .collect::<String>()
    )
}

#[derive(Debug, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub tls: Option<bool>,
    #[serde(default = "default_topic")]
    pub base_topic: String,
    #[serde(default = "default_client_id")]
    pub client_id: String,
}
