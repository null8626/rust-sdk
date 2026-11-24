# Top.gg Rust SDK

The community-maintained Rust SDK for Top.gg.

## Chapters

- [Installation](#installation)
- [Usage](#usage)
  - [General API calls](#general-api-calls)
  - [rocket Webhooks](#rocket-webhooks)
  - [rocket Oauth2](#rocket-oauth2)

## Installation

After that, add the following line to the `dependencies` section of your `Cargo.toml`:

```toml
topgg = "2"
```

## Usage
### General API calls
```rust,no_run
use chrono::{Duration, Utc};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
  let client = topgg::ProjectClient::new(env!("TOPGG_TOKEN").to_string());

  let project = client.get_project().await.unwrap();
  println!("{project:?}");

  let votes = client
    .get_votes(Utc::now() - Duration::hours(1))
    .await
    .unwrap();

  for vote in &*votes {
    println!("{vote:?}");
  }

  let vote = client
    .get_vote(String::from("661200758510977084"), topgg::UserSource::Discord)
    .await
    .unwrap();
  println!("{vote:?}");

  let title = String::from("My project announcement");
  let content = "Lorem ipsum".repeat(25);
  let category = topgg::AnnouncementCategory::Announcement;

  client
    .post_announcement(title, content, category)
    .await
    .unwrap();

  // Array of application commands that
  // can be serialized to Discord API's raw JSON format.
  client
    .post_commands(serde_json::json!([
      {
        "id": "1",
        "type": 1,
        "application_id": "1",
        "name": "test",
        "description": "command description",
        "default_member_permissions": "",
        "version": "1"
      }
    ]))
    .await
    .unwrap();

  client
    .post_metrics(topgg::DiscordBotMetrics {
      server_count: Some(5),
      ..Default::default()
    })
    .await
    .unwrap();

  let headline = HashMap::from_iter([
    (
      topgg::Locale::English,
      String::from("hello this is a headline"),
    ),
    (
      topgg::Locale::German,
      String::from("hallo das ist eine Überschrift"),
    ),
  ]);

  let page_content = HashMap::from_iter([
    (
      topgg::Locale::English,
      "hello this is a page content".repeat(25),
    ),
    (
      topgg::Locale::German,
      "hallo dies ist ein seiteninhalt".repeat(25),
    ),
  ]);

  client.edit_project(headline, page_content).await.unwrap();

  let metrics = topgg::DiscordBotMetrics {
    server_count: Some(5),
    ..Default::default()
  };
  let mut timestamped_metrics = topgg::TimestampedMetrics::default();

  timestamped_metrics
    .add(Utc::now() - Duration::hours(3), metrics)
    .unwrap();
  timestamped_metrics
    .add(Utc::now() - Duration::hours(2), metrics)
    .unwrap();
  timestamped_metrics
    .add(Utc::now() - Duration::hours(1), metrics)
    .unwrap();

  client
    .post_timestamped_metrics(timestamped_metrics)
    .await
    .unwrap();
}
```
### rocket Webhooks

This requires the `webhooks` feature flag to be enabled, which is disabled by default.

```rust,no_run
use rocket::{
  data::{self, Data, FromData, ToByteUnit},
  http::Status,
  request::Request,
};
use std::{io, net::Ipv4Addr, ops::Deref};

#[derive(Debug)]
struct Payload(topgg::webhooks::Payload);

impl Deref for Payload {
  type Target = topgg::webhooks::Payload;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for Payload {
  type Error = io::Error;

  async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
    let Some(signature) = req.headers().get_one("x-topgg-signature") else {
      return data::Outcome::Error((Status::Unauthorized, io::Error::other("Unauthorized")));
    };

    let limit = req.limits().get("string").unwrap_or_else(|| 8.kibibytes());

    match data.open(limit).into_string().await {
      Ok(body) => {
        if body.is_complete() {
          let body = body.into_inner();

          if topgg::webhooks::verify(
            env!("TOPGG_WEBHOOKS_SECRET"),
            signature,
            &body,
          ) && let Ok(payload) = serde_json::from_str(&body)
          {
            return data::Outcome::Success(Self(payload));
          }

          data::Outcome::Error((Status::Unauthorized, io::Error::other("Unauthorized")))
        } else {
          data::Outcome::Error((Status::PayloadTooLarge, io::Error::other("Body too large")))
        }
      }

      Err(error) => data::Outcome::Error((Status::InternalServerError, error)),
    }
  }
}

#[rocket::post("/webhooks", data = "<payload>")]
fn webhooks(payload: Payload) {
  println!("{payload:?}");
}

#[rocket::launch]
fn rocket() -> _ {
  let config = rocket::Config {
    address: Ipv4Addr::new(127, 0, 0, 1).into(),
    port: 3000,
    ..rocket::Config::debug_default()
  };

  rocket::custom(&config).mount("/", rocket::routes![webhooks])
}
```
### rocket Oauth2
```rust,no_run
use rocket::{
  http::Status,
  request::{self, FromRequest, Request},
};
use std::{collections::HashMap, io, net::Ipv4Addr, ops::Deref};

struct Application {
  oauth2: topgg::Oauth2,
  urls: HashMap<String, topgg::Oauth2URL>,
}

struct Oauth2Client(topgg::Oauth2Client);

impl Deref for Oauth2Client {
  type Target = topgg::Oauth2Client;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Oauth2Client {
  type Error = io::Error;

  async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
    let (Some(app), Some(Ok(code)), Some(Ok(state))) = (
      req.rocket().state::<Application>(),
      req.query_value::<&str>("code"),
      req.query_value::<&str>("state"),
    ) else {
      return request::Outcome::Error((Status::Unauthorized, io::Error::other("Unauthorized")));
    };

    if let Some(url) = app.urls.get(&String::from(state))
      && let Ok(session) = url.exchange(code).await
    {
      return request::Outcome::Success(Self(app.oauth2.new_client(session)));
    };

    request::Outcome::Error((Status::Unauthorized, io::Error::other("Unauthorized")))
  }
}

#[rocket::get("/topgg/oauth2")]
async fn oauth2(client: Oauth2Client) {
  let user = client.get_authorized_user().await.unwrap();
  println!("{user:?}");

  let projects = client.get_authorized_user_projects().await.unwrap();
  println!("{projects:?}");
}

#[rocket::launch]
fn rocket() -> _ {
  let mut app = Application {
    oauth2: topgg::Oauth2::new(
      env!("TOPGG_CLIENT_ID").parse().unwrap(),
      env!("TOPGG_CLIENT_SECRET").to_string(),
      String::from("http://localhost:3000/topgg/oauth2"),
      &[
        "user.identify",
        "user.projects.read",
        "user.projects.write",
        "project.information.read",
        "project.information.write",
        "project.votes.read",
        "project.metrics.write",
        "project.announcements.write",
        "project.webhooks.read",
        "project.webhooks.write",
        "project.integrations.read",
      ],
    ),
    urls: HashMap::default(),
  };

  let url = app.oauth2.new_url();

  app.urls.insert(url.get_state().to_string(), url.clone());

  println!("Please open this URL: {}", url.get_url());

  let config = rocket::Config {
    address: Ipv4Addr::new(127, 0, 0, 1).into(),
    port: 3000,
    ..rocket::Config::debug_default()
  };

  rocket::custom(&config)
    .manage(app)
    .mount("/", rocket::routes![oauth2])
}
```