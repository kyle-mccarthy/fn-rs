use failure::Fail;
use serde::{Deserialize, Serialize};
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
    /// HTTP method that the function handles
    pub method: String,
    /// Route that the function is bound to
    pub route: String,
    /// Path to the script which defines the function
    pub handler: String,
    /// Optional command to execute the handler, this is used when the handler script isn't executable
    /// such as with a node function
    pub cmd: Option<String>,
    /// Runtime of the function (ex: wasm, unix_socket)
    pub runtime: String,
    /// Generated automatically, used as the cache key
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
            id: Uuid::new_v4(),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Create the command for executing the function
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
    /// Attempt to load the config
    pub fn load() -> Result<Config, ConfigError> {
        // @todo probably should be configurable in some way
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

    /// Formatted address to bind the HTTP server to
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
