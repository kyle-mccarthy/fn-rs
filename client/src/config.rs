use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("IO Error {}", source))]
    IOError { source: std::io::Error },

    #[snafu(display("Error parsing config file"))]
    ParsingError { source: serde_yaml::Error },
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
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    networking: NetworkingConfig,
    functions: Vec<FunctionConfig>,
}

impl Config {
    pub fn load() -> Result<Config, Error> {
        let filename = "config.yaml";

        let file = File::open(&filename).context(IOError {})?;
        let reader = BufReader::new(file);

        serde_yaml::from_reader(reader).context(ParsingError {})
    }

    pub fn functions(&self) -> &Vec<FunctionConfig> {
        &self.functions
    }

    pub fn functions_iter(&self) -> core::slice::Iter<FunctionConfig> {
        self.functions.iter()
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
