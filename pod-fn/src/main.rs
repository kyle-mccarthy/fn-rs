mod config;
mod handler;
mod request_handler;

use crate::config::Config;
use actix_web::http::Method;
use actix_web::{middleware, web, App, HttpServer};

use crate::request_handler::{get_handler, post_handler};
use snafu::{ensure, ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("IO Error {}", source))]
    ConfigLoadError { source: config::Error },

    #[snafu(display("Error parsing config file"))]
    WebError { source: std::io::Error },

    #[snafu(display("Invalid config {}", context))]
    InvalidConfig { context: String },

    #[snafu(display("Failed to convert method to bytes {}", context))]
    MethodError { context: String },

    #[snafu(display("The method type is not implemented {}", context))]
    UnimplementedMethod { context: String },
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
            let method = Method::from_bytes(func.method.to_uppercase().as_bytes());

            if method.is_err() {
                panic!(MethodError {
                    context: func.method.clone()
                });
            }

            let method = method.unwrap();

            match method {
                Method::POST => {
                    app = app.service(
                        web::resource(&func.route)
                            .data(func.clone())
                            .to_async(post_handler),
                    );
                }
                Method::GET => {
                    app = app.service(
                        web::resource(&func.route)
                            .data(func.clone())
                            .to(get_handler),
                    );
                }
                _ => panic!(UnimplementedMethod {
                    context: func.method.clone()
                }),
            }
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
    use super::config::FunctionConfig;
    use super::handler::handle;

    #[test]
    fn test_cat() {
        let config =
            FunctionConfig::new("GET".to_string(), "/".to_string(), "cat".to_string(), None);

        let res = handle(&config, "Hello, World!");

        assert!(res.error.is_none());

        let stdout = res.stdout;

        assert!(stdout.is_some());

        let stdout = stdout.unwrap();
        let stdout = String::from_utf8(stdout).unwrap();

        assert_eq!(stdout, "Hello, World!");
    }
}
