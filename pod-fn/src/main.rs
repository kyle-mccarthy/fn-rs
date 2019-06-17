pub mod core;
pub mod unix_socket;

mod health;

#[macro_use]
extern crate failure;

use actix_web::http::Method;
use actix_web::{middleware, web, App, HttpServer};

use failure::{Error, Fail};

use crate::core::config::Config;

//use crate::handler::Handle;
use crate::core::request_handler::{get_handler, post_handler};
use actix_web::web::Data;
use r2d2::Pool;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Fail)]
pub enum ServerError {
    #[fail(display = "IO Error {}", _0)]
    ConfigLoadError(core::config::ConfigError),

    #[fail(display = "Error parsing config file")]
    WebError(std::io::Error),

    #[fail(display = "Invalid config {}", _0)]
    InvalidConfig(&'static str),

    #[fail(display = "Failed to convert method to bytes {}", _0)]
    MethodError(String),

    #[fail(display = "The method type is not implemented {}", _0)]
    UnimplementedMethod(String),
}

type State = Data<crate::core::state::State>;

fn main() -> Result<(), ServerError> {
    let config = Config::load().map_err(|source| ServerError::ConfigLoadError(source))?;
    let address = config.address();

    if config.functions().len() == 0 {
        return Err(ServerError::InvalidConfig(
            "Config must contain at least 1 function",
        ));
    }

    //    let socket_add = std::os::
    //    let mut stream = UnixStream::connect("/var/tmp/test_socket.sock").unwrap();

    HttpServer::new(move || {
        let app_data = web::Data::new(crate::core::state::State::new());

        let mut app = App::new()
            .wrap(middleware::Logger::default())
            .register_data(app_data)
            .route("/_ah", web::get().to(health::handle));

        for func in config.functions_iter() {
            let method = Method::from_bytes(func.method.to_uppercase().as_bytes());

            if method.is_err() {
                panic!(ServerError::MethodError(func.method.clone()));
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
                            .to_async(get_handler),
                    );
                }
                _ => panic!(ServerError::UnimplementedMethod(func.method.clone())),
            }
        }

        app
    })
    //    .workers(1)
    .bind(address)
    .map_err(|e| ServerError::WebError(e))?
    .run()
    .map_err(|e| ServerError::WebError(e))
}