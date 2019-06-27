mod health;

use actix_web::{middleware, web, App, HttpServer};

use failure::Fail;

use fn_core::config::{Config, ConfigError};
use fn_gateway::bootstrap_gateway;

#[derive(Debug, Fail)]
pub enum Errors {
    #[fail(display = "IO Error {}", _0)]
    ConfigLoadError(ConfigError),

    #[fail(display = "Error parsing config file")]
    WebError(std::io::Error),

    #[fail(display = "Invalid config {}", _0)]
    InvalidConfig(&'static str),
}

fn main() -> Result<(), Errors> {
    let config = Config::load().map_err(|source| Errors::ConfigLoadError(source))?;
    let address = config.address();

    if config.functions().len() == 0 {
        return Err(Errors::InvalidConfig(
            "Config must contain at least 1 function",
        ));
    }

    HttpServer::new(move || {
        // registering the data here allows for each thread to have their own function runtime cache
        // this is particularly useful when using unix sockets, since each thread will create their
        // own function process
        let app_data = web::Data::new(fn_core::state::State::new());

        let mut app = App::new()
            .wrap(middleware::Logger::default())
            .register_data(app_data.clone())
            .route("/_ah", web::get().to(health::handle));

        // this will panic if something goes wrong during the bootstrapping process
        app = bootstrap_gateway(app, &config);

        app
    })
    .workers(1)
    .bind(address)
    .map_err(|e| Errors::WebError(e))?
    .run()
    .map_err(|e| Errors::WebError(e))
}
