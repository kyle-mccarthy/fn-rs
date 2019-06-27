mod handlers;

use actix_service::NewService;
use actix_web::dev::{MessageBody, ServiceRequest, ServiceResponse};
use actix_web::http::Method;
use actix_web::{web, App, Error};
use failure::Fail;
use fn_core::config::Config;
use handlers::{get_handler, post_handler};

#[derive(Debug, Fail)]
pub enum Errors {
    #[fail(
        display = "Failed to convert method listed in config to axtix_web::http::Method: {}",
        _0
    )]
    MethodError(String),

    #[fail(
        display = "A handler for this HTTP Method has not been implemented: {}",
        _0
    )]
    UnimplementedMethod(String),
}

pub fn bootstrap_gateway<T, B>(mut app: App<T, B>, config: &Config) -> App<T, B>
where
    B: MessageBody,
    T: NewService<
        Config = (),
        Request = ServiceRequest,
        Response = ServiceResponse<B>,
        Error = Error,
        InitError = (),
    >,
{
    for func in config.functions_iter() {
        let method = Method::from_bytes(func.method.to_uppercase().as_bytes());

        if method.is_err() {
            panic!(Errors::MethodError(func.method.clone()));
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
            _ => panic!(Errors::UnimplementedMethod(func.method.clone())),
        }
    }

    app
}
