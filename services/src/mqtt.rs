use std::convert::TryFrom;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono;
use futures::prelude::*;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use log::{debug, info, warn};
use rumqttc::{
    AsyncClient, Event, Incoming, MqttOptions, Outgoing, Packet, Publish, QoS, Transport,
};
use serde::{Deserialize, Serialize};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use super::printnanny_api::ApiService;
use crate::config::{MQTTConfig, PrintNannyConfig};
use printnanny_api_client::models;

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    aud: String, // Google Cloud Project id
    iat: i64,    // Issued At (as UTC timestamp)
    exp: i64,    // Expiration
}

#[derive(Debug, Clone)]
pub struct MQTTWorker {
    config: PrintNannyConfig,
    config_topic: String,
    event_topic: String,
    command_topic: String,
    state_topic: String,
    mqttoptions: MqttOptions,
}

fn encode_jwt(private_key: &PathBuf, claims: &Claims) -> Result<String> {
    let contents =
        fs::read(private_key).context(format!("Failed to read file {:?}", private_key))?;
    let key = EncodingKey::from_ec_pem(&contents)
        .context(format!("Failed to encode EC pem from {:#?}", private_key))?;
    let result = encode(&Header::new(Algorithm::ES256), &claims, &key)?;
    Ok(result)
}

impl MQTTWorker {
    fn mqttoptions(
        cloudiot_device: &models::CloudiotDevice,
        config: &MQTTConfig,
        token: &str,
    ) -> Result<MqttOptions> {
        let mqtt_port = u16::try_from(cloudiot_device.mqtt_bridge_port)?;

        let mut mqttoptions = MqttOptions::new(
            &cloudiot_device.mqtt_client_id,
            &cloudiot_device.mqtt_bridge_hostname,
            mqtt_port,
        );
        mqttoptions.set_keep_alive(Duration::new(config.keepalive, 0));
        mqttoptions.set_credentials("unused", token);

        let mut roots = rustls::RootCertStore::empty();

        for cert in config.ca_certs.iter() {
            let root_ca_bytes =
                std::fs::read(cert).context(format!("Failed to read file {:?}", cert))?;
            let root_cert = rustls::Certificate(root_ca_bytes);
            roots.add(&root_cert)?;
        }

        let mut client_config = rumqttc::ClientConfig::new();
        client_config.root_store = roots;
        client_config.versions = vec![rustls::ProtocolVersion::TLSv1_2];
        mqttoptions.set_transport(Transport::tls_with_config(client_config.into()));
        Ok(mqttoptions)
    }

    pub async fn new() -> Result<MQTTWorker> {
        let config: PrintNannyConfig = PrintNannyConfig::new()?;
        let service = ApiService::new(config.clone())?;
        let device = match service.config.device.clone() {
            Some(d) => Ok(d),
            None => Err(anyhow!(
                "Failed to read Device info from config {:?}",
                &service.config
            )),
        }?;
        info!(
            "Initializing subscription from models::CloudiotDevice {:?}",
            device.cloudiot_device
        );
        let cloudiot_device = device.cloudiot_device.as_ref().unwrap();
        // let cloudiot_device = match device.cloudiot_device {
        //     Some(d) => d,
        //     None => {
        //         let public_key = service
        //             .device_public_key_update_or_create(device.id)
        //             .await?;
        //         let cloudiot_device = service
        //             .cloudiot_device_update_or_create(device.id, public_key.id)
        //             .await?;
        //         Box::new(cloudiot_device)
        //     }
        // };
        let gcp_project_id: String = cloudiot_device.gcp_project_id.clone();

        let iat = chrono::offset::Utc::now().timestamp(); // issued at (seconds since epoch)
        let exp = iat + 86400; // 24 hours later
        let claims = Claims {
            iat,
            exp,
            aud: gcp_project_id,
        };
        let token = encode_jwt(&config.keys.ec_private_key_file(), &claims)?;
        let mqttoptions = MQTTWorker::mqttoptions(cloudiot_device, &config.mqtt, &token)?;

        let result = MQTTWorker {
            config,
            state_topic: cloudiot_device.state_topic.clone(),
            command_topic: cloudiot_device.command_topic.clone(),
            config_topic: cloudiot_device.config_topic.clone(),
            event_topic: cloudiot_device.event_topic.clone(),
            mqttoptions,
        };
        Ok(result)
    }
    async fn handle_event(&self, event: &Publish) -> Result<()> {
        info!("Handling event {:?}", event);
        match &event.topic {
            _ if event.topic == self.config_topic => {
                warn!("Ignored msg on config topic {:?}", event)
            }
            _ if event.topic == self.event_topic => {
                warn!("Ignored msg on event topic {:?}", event)
            }
            _ if event.topic == self.state_topic => {
                warn!("Ignored msg on state topic {:?}", event)
            }
            _ if self.command_topic.contains(&event.topic) => {
                // let data = serde_json::from_slice::<models::PolymorphicCommand>(&event.payload)?;
                unimplemented!("add_to_queue not implemented in this release")
                // self.config.cmd.add_to_queue(data);
            }
            _ => warn!("Ignored published event {:?}", event),
        };
        Ok(())
    }

    // re-publish printnanny events unix sock to mqtt topic
    pub async fn publish(&self, data: &str) -> Result<()> {
        // deserialize to PolymorphicEventCreateRequest to validate fieldset
        let event: models::PolymorphicEventCreateRequest =
            serde_json::from_str(data).expect("Failed to deserialize event data");
        info!("Publishing event: {:?}", event);
        // serialize event struct as serde_json::Value to send length-delimited bytes over unix socket
        let value: serde_json::Value = serde_json::to_value(event)?;

        // open a connection to unix socket
        let stream = UnixStream::connect(&self.config.paths.events_socket)
            .await
            .context(format!(
                "Failed to connect to socket {:?}",
                &self.config.paths.events_socket
            ))?;
        // Delimit frames using a length header
        let length_delimited = FramedWrite::new(stream, LengthDelimitedCodec::new());

        // Serialize frames with JSON
        let mut serialized = tokio_serde::SymmetricallyFramed::new(
            length_delimited,
            tokio_serde::formats::SymmetricalJson::<serde_json::Value>::default(),
        );
        serialized.send(value).await?;
        Ok(())
    }

    pub async fn subscribe_mqtt(&self) -> Result<()> {
        let (client, mut eventloop) = AsyncClient::new(self.mqttoptions.clone(), 64);
        client
            .subscribe(&self.config_topic, QoS::AtLeastOnce)
            .await
            .unwrap();
        client
            .subscribe(&self.command_topic, QoS::AtLeastOnce)
            .await
            .unwrap();
        client
            .subscribe(&self.state_topic, QoS::AtLeastOnce)
            .await
            .unwrap();
        loop {
            let incoming = eventloop.poll().await?;
            match &incoming {
                Event::Incoming(Packet::PingResp) => {
                    debug!("Received = {:?}", &incoming)
                }
                Event::Outgoing(Outgoing::PingReq) => {
                    debug!("Received = {:?}", &incoming)
                }
                Event::Incoming(Incoming::Publish(e)) => self.handle_event(e).await?,
                _ => info!("Received = {:?}", &incoming),
            }
        }
    }
    pub async fn subscribe_event_socket(&self) -> Result<()> {
        let maybe_delete = std::fs::remove_file(&self.config.paths.events_socket);
        match maybe_delete {
            Ok(_) => {
                warn!(
                    "Deleted socket {:?} without mercy. Refactor this code to run 2+ concurrent socket listeners/bindings.",
                    &self.config.paths.events_socket
                );
            }
            Err(_) => {}
        };
        let listener = UnixListener::bind(&self.config.paths.events_socket)?;
        loop {
            let (mut socket, _) = listener.accept().await?;
            info!("Accepted socket connection {:?}", &socket);
            let length_delimited = FramedRead::new(&mut socket, LengthDelimitedCodec::new());
            let mut deserialized = tokio_serde::SymmetricallyFramed::new(
                length_delimited,
                tokio_serde::formats::SymmetricalJson::<serde_json::Value>::default(),
            );
            let msg = deserialized.try_next().await?;
            info!("Deserialized msg {:?}", msg);
        }
    }
}
