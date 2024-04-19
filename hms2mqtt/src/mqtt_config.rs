use std::collections::HashMap;

use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Deserializer};

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

#[derive(Debug, Deserialize, Clone, Default)]
struct SerialAliases(Vec<SerialAlias>);

#[derive(Debug, Deserialize, Clone, Default)]
struct SerialAlias {
    serial: String,
    alias: String,
}

fn deserialize_alias<'de, D>(deserializer: D) -> Result<HashMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut res = HashMap::new();
    for alias in SerialAliases::deserialize(deserializer)?.0 {
        res.insert(alias.serial, alias.alias);
    }
    Ok(res)
}

#[derive(Debug, Deserialize, Clone, Default)]
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
    /// Maps serials to an alias
    #[serde(deserialize_with = "deserialize_alias", default)]
    pub serial_aliases: HashMap<String, String>,
}

#[cfg(test)]
mod test {
    use super::MqttConfig;

    #[test]
    fn test_deserialize() {
        let conf_str = include_str!("../test/configs/test_mqtt_conf.yaml");
        let conf: MqttConfig = serde_yaml::from_str(conf_str).unwrap();

        assert_eq!(conf.host, "::1");
        assert_eq!(conf.username.unwrap(), "test");
        assert_eq!(conf.password.unwrap(), "testpw");
        assert_eq!(conf.tls, None);
        assert_eq!(conf.base_topic, "hms800wt2");
        assert!(conf.client_id.starts_with("hms-mqtt-"));
        assert_eq!(conf.serial_aliases.len(), 1);
        assert_eq!(conf.serial_aliases.get("123").unwrap(), "test_alias");
    }
}
