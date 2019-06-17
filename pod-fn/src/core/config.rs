use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use uuid::Uuid;

use std::process::Command;

#[derive(Debug, Fail)]
pub enum ConfigError {
    #[fail(display = "IO Error {}", _0)]
    IOError(std::io::Error),

    #[fail(display = "Error parsing config file")]
    ParsingError(serde_yaml::Error),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkingConfig {
    #[serde(default = Config::default_host())]
    pub host: String,
    #[serde(default = Config::default_port())]
    pub port: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FunctionConfig {
    pub method: String,
    pub route: String,
    pub handler: String,
    pub cmd: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub runtime: String,

    #[serde(default = "uuid::Uuid::new_v4")]
    pub id: Uuid,
}

impl FunctionConfig {
    #[allow(dead_code)]
    pub fn new(
        method: String,
        route: String,
        handler: String,
        cmd: Option<String>,
        runtime: String,
    ) -> FunctionConfig {
        FunctionConfig {
            method,
            route,
            handler,
            cmd,
            runtime,
            headers: None,
            id: Uuid::new_v4(),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn cmd(&self) -> Command {
        match &self.cmd {
            Some(cmd) => {
                let mut command = Command::new(cmd.as_str());
                command.arg(&self.handler);
                command
            }
            _ => Command::new(&self.handler),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    networking: NetworkingConfig,
    functions: Vec<FunctionConfig>,
}

impl Config {
    pub fn load() -> Result<Config, ConfigError> {
        let filename = "config.yaml";

        let file = File::open(&filename).map_err(|e| ConfigError::IOError(e))?;
        let reader = BufReader::new(file);

        serde_yaml::from_reader(reader).map_err(|e| ConfigError::ParsingError(e))
    }

    pub fn functions(&self) -> &Vec<FunctionConfig> {
        &self.functions
    }

    pub fn functions_iter(&self) -> core::slice::Iter<FunctionConfig> {
        (&self.functions).iter()
    }

    pub fn address(&self) -> String {
        format!("{}:{}", &self.networking.host, &self.networking.port)
    }

    #[allow(dead_code)]
    pub fn default_host() -> String {
        return "0.0.0.0".to_string();
    }

    #[allow(dead_code)]
    pub fn default_port() -> String {
        return "80".to_string();
    }
}
