#![allow(dead_code)]

mod website;

use actix_web::{error, middleware, web, App, Error, HttpResponse, HttpServer, Result};
use clokwerk::{AsyncScheduler, TimeUnits};
use serde_derive::{Deserialize, Serialize};
use std::io::Read;
use tera::Tera;
use tokio::sync::Mutex;
use website::Website;

#[macro_use]
extern crate actix_web;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    websites: Vec<Website>,
}

#[get("/")]
async fn index(
    tmpl: web::Data<Tera>,
    config: web::Data<Mutex<Config>>,
) -> Result<HttpResponse, Error> {
    let res = tmpl
        .render(
            "index.html",
            &tera::Context::from_serialize(&config.lock().await.clone()).unwrap(),
        )
        .map_err(|e| error::ErrorInternalServerError(format!("Template error: {}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(res))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    HttpServer::new(move || {
        let tera = Tera::new("templates/**").unwrap();

        let mut file = std::fs::File::open("settings.toml").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let config: Config = toml::from_str(&contents).unwrap();

        let mutexconfig = web::Data::new(Mutex::new(config));

        let app = App::new()
            .app_data(web::Data::new(tera))
            .app_data(web::Data::clone(&mutexconfig))
            .wrap(middleware::Logger::new(
                r#"%{r}a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %Dms"#,
            ))
            .service(index);

        let mut scheduler = AsyncScheduler::new();

        scheduler
            .every(1.seconds())
            .run(move || website::updater(web::Data::clone(&mutexconfig)));

        tokio::spawn(async move {
            loop {
                scheduler.run_pending().await;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });

        app
    })
    .bind(("127.0.0.1", 6900))?
    .run()
    .await
}
