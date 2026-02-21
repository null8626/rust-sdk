# [topgg](https://crates.io/crates/topgg) [![crates.io][crates-io-image]][crates-io-url] [![crates.io downloads][crates-io-downloads-image]][crates-io-url]

[crates-io-image]: https://img.shields.io/crates/v/topgg?style=flat-square
[crates-io-downloads-image]: https://img.shields.io/crates/d/topgg?style=flat-square
[crates-io-url]: https://crates.io/crates/topgg

The official Rust SDK for the [Top.gg API](https://docs.top.gg).

## Getting Started

Make sure to have a [Top.gg API](https://docs.top.gg) token handy. If not, then [view this tutorial on how to retrieve yours](https://github.com/top-gg/rust-sdk/assets/60427892/d2df5bd3-bc48-464c-b878-a04121727bff). After that, add the following line to the `dependencies` section of your `Cargo.toml`:

```toml
topgg = "2"
```

For more information, please read [the documentation](https://docs.rs/topgg)!

## Features

This library provides several feature flags that can be enabled/disabled in `Cargo.toml`. Such as:

- **`api`**: Interacting with the [Top.gg API](https://docs.top.gg) and accessing the `top.gg/api/*` endpoints. (enabled by default)
- **`webhook`**: Accessing the [serde deserializable](https://docs.rs/serde/latest/serde/de/trait.DeserializeOwned.html) `topgg::Payload` struct.
  - **`actix-web`**: Wrapper for working with the [actix-web](https://actix.rs/) web framework.
  - **`axum`**: Wrapper for working with the [axum](https://crates.io/crates/axum) web framework.
  - **`rocket`**: Wrapper for working with the [rocket](https://rocket.rs/) web framework.
  - **`warp`**: Wrapper for working with the [warp](https://crates.io/crates/warp) web framework.
- **`serenity`**: Extra helpers for working with [serenity](https://crates.io/crates/serenity).
- **`twilight`**: Extra helpers for working with [twilight](https://twilight.rs).

## Examples

### Getting your project's information

```rust,no_run
let project = client.get_self().await.unwrap();
```

### Getting your project's vote information of a user

#### Discord ID

```rust,no_run
let vote = client.get_vote(UserSource::Discord(661200758510977084)).await.unwrap();
```

#### Top.gg ID

```rust,no_run
let vote = client.get_vote(UserSource::Topgg(8226924471638491136)).await.unwrap();
```

### Getting a cursor-based paginated list of votes for your project

```rust,no_run
use chrono::{TimeZone, Utc};

let since = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single().unwrap();
let first_page = client.get_votes(since).await.unwrap();

for vote in first_page.iter() {
  println!("{vote:?}");
}

let second_page = first_page.next().await.unwrap();

for vote in second_page.iter() {
  println!("{vote:?}");
}
```

### Posting your bot's application commands list

#### Serenity

```rust,no_run
client.post_commands(&ctx).await.unwrap();
```

#### Twilight

```rust,no_run
let application_id = bot.current_user_application().await.unwrap().model().await.unwrap().id;
let interaction = bot.interaction(application_id);

client.post_commands(interaction.global_commands()).await.unwrap();
```

#### Raw

```rust,no_run
let commands = json!([{
  "id": "1",
  "type": 1,
  "application_id": "1",
  "name": "test",
  "description": "command description",
  "default_member_permissions": "",
  "version": "1"
}]); // Array of application commands that
     // can be serialized to Discord API's raw JSON format.

client.post_commands(commands).await.unwrap();
```

### Generating widget URLs

#### Large

```rust,no_run
let widget_url = topgg::widget::large(topgg::ProjectType::DiscordBot, 574652751745777665);
```

#### Votes

```rust,no_run
let widget_url = topgg::widget::votes(topgg::ProjectType::DiscordBot, 574652751745777665);
```

#### Owner

```rust,no_run
let widget_url = topgg::widget::owner(topgg::ProjectType::DiscordBot, 574652751745777665);
```

#### Social

```rust,no_run
let widget_url = topgg::widget::social(topgg::ProjectType::DiscordBot, 574652751745777665);
```

### Webhooks

#### Actix-web

In your `Cargo.toml`:

```toml
[dependencies]
topgg = { version = "2", default-features = false, features = ["actix-web"] }
```

In your code:

```rust,no_run
use topgg::IncomingPayload;
use std::io;

use actix_web::{
  error::{Error, ErrorUnauthorized},
  get, post, App, HttpServer,
};

#[get("/")]
async fn index() -> &'static str {
  "Hello, World!"
}

#[post("/webhook")]
async fn webhook(payload: IncomingPayload) -> Result<&'static str, Error> {
  match payload.authenticate(env!("TOPGG_WEBHOOK_SECRET")) {
    Some(payload) => {
      println!("{payload:?}");

      Ok("ok")
    }

    _ => Err(ErrorUnauthorized("401")),
  }
}

#[actix_web::main]
async fn main() -> io::Result<()> {
  HttpServer::new(|| App::new().service(index).service(webhook))
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

#### Axum

In your `Cargo.toml`:

```toml
[dependencies]
topgg = { version = "2", default-features = false, features = ["axum"] }
```

In your code:

```rust,no_run
use topgg::Payload;
use std::sync::Arc;

use axum::{http::status::StatusCode, response::{IntoResponse, Response}, routing::get, Router};
use tokio::net::TcpListener;

struct MyTopggListener {}

#[async_trait::async_trait]
impl topgg::axum::Listener for MyTopggListener {
  async fn callback(self: Arc<Self>, payload: Payload, _trace: &str) -> Response {
    println!("{payload:?}");

    (StatusCode::NO_CONTENT, ()).into_response()
  }
}

async fn index() -> &'static str {
  "Hello, World!"
}

#[tokio::main]
async fn main() {
  let state = Arc::new(MyTopggListener {});

  let router = Router::new().route("/", get(index)).nest(
    "/webhook",
    topgg::axum::webhook(Arc::clone(&state), env!("TOPGG_WEBHOOK_SECRET").to_string()),
  );

  let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

  axum::serve(listener, router).await.unwrap();
}
```

#### Rocket

In your `Cargo.toml`:

```toml
[dependencies]
topgg = { version = "2", default-features = false, features = ["rocket"] }
```

In your code:

```rust,no_run
use topgg::IncomingPayload;

use rocket::{get, http::Status, launch, post, routes, Build, Rocket};

#[get("/")]
fn index() -> &'static str {
  "Hello, World!"
}

#[post("/webhook", data = "<payload>")]
fn webhook(payload: IncomingPayload) -> Status {
  match payload.authenticate(env!("TOPGG_WEBHOOK_SECRET")) {
    Some(payload) => {
      println!("{payload:?}");

      Status::Ok
    },
    _ => {
      println!("found an unauthorized attacker.");

      Status::Unauthorized
    }
  }
}

#[launch]
fn rocket() -> Rocket<Build> {
  rocket::build().mount("/", routes![index, webhook])
}
```

#### Warp

In your `Cargo.toml`:

```toml
[dependencies]
topgg = { version = "2", default-features = false, features = ["warp"] }
```

In your code:

```rust,no_run
use std::net::SocketAddr;

use warp::{http::StatusCode, reply, Filter};

#[tokio::main]
async fn main() {
  // POST /webhook
  let webhook = topgg::warp::webhook(
    "webhook",
    env!("TOPGG_WEBHOOK_SECRET").to_string()
  ).then(|payload, _trace| async move {
    match payload {
      Some(payload) => {
        println!("{payload:?}");

        reply::with_status("", StatusCode::NO_CONTENT)
      },

      None => reply::with_status("Unauthorized", StatusCode::UNAUTHORIZED)
    }
  });

  let routes = warp::get().map(|| "Hello, World!").or(webhook);

  let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

  warp::serve(routes).run(addr).await
}
```
