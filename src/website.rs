use actix_web::web::Data;
use chrono::NaiveDateTime;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}

#[derive(Default, Deserialize, Serialize, Debug, Clone)]
pub enum Status {
    Up,
    #[default]
    Down,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Website {
    pub name: String,
    pub endpoint: String,
    pub method: Method,
    pub description: Option<String>,
    #[serde(skip_deserializing)]
    #[serde(skip_serializing)]
    pub last_online: Option<NaiveDateTime>,
    #[serde(skip_deserializing)]
    pub status: Status,
    pub update_interval: Option<u32>,
    pub next_update: Option<u32>,
}

impl Website {
    pub async fn update_status(&mut self) {
        println!("Updating for {}", &self.name);

        let client = reqwest::Client::builder()
            .user_agent("TestausUPtime 0.1")
            .build()
            .unwrap();

        let rq = match self.method {
            Method::Get => client.get(&self.endpoint),
            Method::Post => client.post(&self.endpoint),
            _ => unimplemented!(),
        };

        if let Ok(rs) = rq.send().await {
            if rs.status().is_success() {
                self.status = Status::Up;
                self.last_online = Some(chrono::Local::now().naive_local());
            } else {
                self.status = Status::Down;
            }
        } else {
            self.status = Status::Down;
        }
    }
}

pub async fn updater(config: Data<Mutex<crate::Config>>) {
    let mut sites_clone = config.lock().await.websites.clone();
    for site in &mut sites_clone {
        if let Some(next) = site.next_update {
            if next == 0 {
                site.next_update = Some(site.update_interval.unwrap_or(60));
                site.update_status().await;
            } else {
                site.next_update = Some(next - 1);
            }
        } else {
            site.next_update = Some(site.update_interval.unwrap_or(60));
            site.update_status().await;
        }
    }

    config.lock().await.websites = sites_clone;
}
