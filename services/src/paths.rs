use std::path::{ PathBuf };
use std::fs::{ OpenOptions };

use anyhow::{ Result };
use serde::{ Serialize, Deserialize };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintNannyPath {
    pub backups: PathBuf,
    pub base: PathBuf,
    pub data: PathBuf,
    pub keys: PathBuf,
    pub ca_certs: PathBuf,

    // this struct
    pub paths_json: PathBuf,

    // api config
    pub api_config_json: PathBuf,
    
    pub janus_config: PathBuf,
    pub janus_admin_secret: PathBuf,
    pub janus_token: PathBuf,

    pub private_key: PathBuf,
    pub public_key: PathBuf,
    pub ca_cert: PathBuf,
    pub ca_cert_backup: PathBuf,
}

impl PrintNannyPath {
    pub fn new(base_str: &str) -> Self {
        let base = PathBuf::from(base_str);
 
        let backups = base.join("backups");
        let data = base.join("data");
        let keys = base.join("keys");

        let ca_certs = base.join("ca-certificates");
        let ca_cert= ca_certs.join("gtsltsr.crt");
        let ca_cert_backup = ca_certs.join("GSR4.crt");

        let device_info_json = data.join("device_info.json");
        let api_config_json = data.join("api_config.json");
        let paths_json = data.join("paths.json");

        let private_key = data.join("ecdsa256_pkcs8.pem");
        let public_key = data.join("ecdsa_public.pem");

        let janus_config = PathBuf::from("/etc/janus");
        let janus_admin_secret = janus_config.join("janus_admin_secret");
        let janus_token = janus_config.join("janus_token");

        Self { 
            api_config_json,
            backups,
            base,
            ca_cert_backup,
            ca_cert,
            ca_certs,
            data,
            keys,
            paths_json,
            private_key,
            public_key,
            janus_admin_secret,
            janus_config,
            janus_token
        }
    }
}

impl PrintNannyPath {
    pub fn save(&self) -> Result<()>{
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.paths_json)?;
        serde_json::to_writer(&file, &self)?;
        Ok(())
    }
}