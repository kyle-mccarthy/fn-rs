mod config;
mod handler;
mod request_handler;

use crate::config::Config;
use actix_web::{middleware, web, App, HttpServer};

use crate::request_handler::{web_handler, FuncConfig};
use snafu::{ensure, ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("IO Error {}", source))]
    ConfigLoadError { source: config::Error },

    #[snafu(display("Error parsing config file"))]
    WebError { source: std::io::Error },

    #[snafu(display("Invalid config {}", context))]
    InvalidConfig { context: String },
}

fn main() -> Result<(), Error> {
    let config = Config::load().context(ConfigLoadError {})?;
    let address = config.address();

    ensure!(
        config.functions().len() > 0,
        InvalidConfig {
            context: "Config must contain at least 1 function".to_string()
        }
    );

    HttpServer::new(move || {
        let mut app = App::new().wrap(middleware::Logger::default());

        for func in config.functions_iter() {
            // @todo need to actually use the method defined in the config
            app = app.service(
                web::resource(&func.route)
                    .data(FuncConfig {
                        script: func.handler.clone(),
                    })
                    .to(web_handler),
            );
        }

        app
    })
    .bind(address)
    .context(WebError {})?
    .run()
    .context(WebError {})
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
