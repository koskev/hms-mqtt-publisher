// TODO: support CA33 command to take over metrics consumption
// TODO: support publishing to S-Miles cloud, too

mod logging;
mod rumqttc_wrapper;

use clap::Parser;
use core::panic;
use hms2mqtt::home_assistant::HomeAssistant;
use hms2mqtt::inverter::FakeInverter;
use hms2mqtt::inverter::HMSInverter;
use hms2mqtt::inverter::Inverter;
use hms2mqtt::metric_collector::MetricCollector;
use hms2mqtt::mqtt::Mqtt;
use hms2mqtt::mqtt_config;
use hms2mqtt::simple_mqtt::SimpleMqtt;
use mqtt_config::MqttConfig;
use rumqttc_wrapper::RumqttcWrapper;
use serde_derive::Deserialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use log::info;

// TODO: update once https://togithub.com/serde-rs/serde/issues/368 is closed
fn default_update_interval() -> u64 {
    30_500
}

#[derive(Debug, Deserialize)]
struct Config {
    inverter_hosts: Vec<String>,
    #[serde(default = "default_update_interval")]
    update_interval: u64,
    home_assistant: Option<MqttConfig>,
    simple_mqtt: Option<MqttConfig>,
    mqtt: Option<MqttConfig>,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Use a fake inverter for every inverter in config
    #[arg(short, long)]
    fake: bool,

    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

fn load_config(path: &PathBuf) -> Result<Config, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let extension = path.extension().unwrap_or_default();
    let config;
    if extension == "toml" {
        config = toml::from_str(&contents)?;
    } else if extension == "yaml" || extension == "yml" {
        config = serde_yaml::from_str(&contents)?;
    } else {
        // TODO: proper error
        panic!("Unknown config file");
    }
    Ok(config)
}

fn main() {
    logging::init_logger();
    let args = Cli::parse();
    info!("Running revision: {}", env!("GIT_HASH"));

    // TODO: proper error handling
    let config: Config = load_config(&args.config).expect("Failed to load config");

    info!("inverter hosts: {:?}", config.inverter_hosts);
    let mut inverters: Vec<Box<dyn Inverter>> = config
        .inverter_hosts
        .iter()
        .map(|host| {
            let res: Box<dyn Inverter> = if args.fake {
                Box::new(FakeInverter { sn: host.clone() })
            } else {
                Box::new(HMSInverter::new(host))
            };
            res
        })
        .collect();

    let mut output_channels: Vec<Box<dyn MetricCollector>> = Vec::new();
    if let Some(config) = config.home_assistant {
        info!("Publishing to Home Assistant");
        output_channels.push(Box::new(HomeAssistant::<RumqttcWrapper>::new(&config)));
    }

    if let Some(config) = config.mqtt {
        info!("Publishing to MQTT broker");
        output_channels.push(Box::new(Mqtt::<RumqttcWrapper>::new(&config)));
    }

    if let Some(config) = config.simple_mqtt {
        info!("Publishing to simple MQTT broker");
        output_channels.push(Box::new(SimpleMqtt::<RumqttcWrapper>::new(&config)));
    }

    loop {
        inverters.iter_mut().for_each(|inverter| {
            if let Some(r) = inverter.update_state() {
                output_channels.iter_mut().for_each(|channel| {
                    channel.publish(&r);
                })
            }
        });

        // TODO: the sleep has to move into the Inverter struct in an async implementation
        thread::sleep(Duration::from_millis(config.update_interval));
    }
}
