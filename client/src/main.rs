mod config;
mod handler;
mod request_handler;

use crate::config::Config;
use actix_web::{middleware, web, App, HttpServer};
use std::env;

use snafu::{Backtrace, OptionExt, ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("IO Error {}", source))]
    ConfigError { source: config::Error },

    #[snafu(display("Error parsing config file"))]
    WebError { source: std::io::Error },
}

fn main() -> Result<(), Error> {
    let host = env_var("HOST", "127.0.0.1");
    let port = env_var("PORT", "3000");

    let address = host + ":" + &port;

    let config = Config::load().context(ConfigError {})?;

    dbg!(config);

    HttpServer::new(|| {
        let script = env::var("FUNC").expect("Target function not defined");

        App::new()
            .service(
                web::resource("/")
                    .data(request_handler::FuncConfig { script })
                    .to(request_handler::web_handler),
            )
            .wrap(middleware::Logger::default())
    })
    .bind(address)
    .context(WebError {})?
    .run()
    .context(WebError {})
}

fn env_var(key: &'static str, default: &'static str) -> String {
    match env::var(key) {
        Ok(val) => val,
        Err(_) => default.to_string(),
    }
}

#[cfg(test)]
mod test {
    use super::handler::handle;

    #[test]
    fn test_cat() {
        let res = handle("cat", "Hello, World!");

        assert!(res.error.is_none());

        let stdout = res.stdout;

        assert!(stdout.is_some());

        let stdout = stdout.unwrap();
        let stdout = String::from_utf8(stdout).unwrap();

        assert_eq!(stdout, "Hello, World!");
    }
}
