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

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkingConfig {
    pub host: String,
    pub port: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FunctionConfig {
    pub method: String,
    pub route: String,
    pub handler: String,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    networking: NetworkingConfig,
    functions: Vec<FunctionConfig>,
}

impl Config {
    pub fn load() -> Result<Config, Error> {
        let filename = "config.yaml";

        let mut file = File::open(&filename).context(IOError {})?;
        let reader = BufReader::new(file);

        serde_yaml::from_reader(reader).context(ParsingError {})
    }
}
