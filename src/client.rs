use super::{
  util, Error, GetCommands, PostBotCommandsError, PostBotCommandsResult, Project, Result,
};

use reqwest::{header, IntoUrl, Method, Response, StatusCode, Version};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[macro_export]
macro_rules! api {
  ($e:literal) => {
    concat!("https://top.gg/api/v1", $e)
  };

  ($e:literal, $($rest:tt)*) => {
    format!($super::client::api!($e), $($rest)*)
  };
}

pub(crate) use api;

#[derive(Deserialize)]
struct ErrorJson {
  #[serde(default, alias = "message", alias = "detail")]
  message: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename = "kebab-case")]
struct Ratelimit {
  retry_after: u16,
}

/// Interact with the v1 API's endpoints.
#[must_use]
pub struct Client {
  http: reqwest::Client,
  token: String,
  id: u64,
}

impl Client {
  /// Creates a new instance.
  ///
  /// To retrieve your API token, [see this tutorial](https://github.com/top-gg-community/rust-sdk/assets/60427892/d2df5bd3-bc48-464c-b878-a04121727bff).
  ///
  /// # Panics
  ///
  /// Panics if the client uses an invalid API token.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// let client = topgg::Client::new(env!("TOPGG_TOKEN").to_string());
  /// ```
  pub fn new(token: String) -> Self {
    let id = util::parse_api_token(&token);

    Self {
      http: reqwest::Client::new(),
      token: format!("Bearer {token}"),
      id,
    }
  }

  async fn send_inner(&self, method: Method, url: impl IntoUrl, body: Vec<u8>) -> Result<Response> {
    match self
      .http
      .execute(
        self
          .http
          .request(method, url)
          .header(header::AUTHORIZATION, &self.token)
          .header(header::CONNECTION, "close")
          .header(header::CONTENT_LENGTH, body.len())
          .header(header::CONTENT_TYPE, "application/json")
          .header(
            header::USER_AGENT,
            "topgg (https://github.com/top-gg/rust-sdk) Rust",
          )
          .version(Version::HTTP_11)
          .body(body)
          .build()
          .unwrap(),
      )
      .await
    {
      Ok(response) => {
        let status = response.status();

        if status.is_success() {
          Ok(response)
        } else {
          Err(match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => panic!("Invalid API token."),

            StatusCode::NOT_FOUND => Error::NotFound(
              util::parse_json::<ErrorJson>(response)
                .await
                .ok()
                .and_then(|err| err.message),
            ),

            StatusCode::TOO_MANY_REQUESTS => util::parse_json::<Ratelimit>(response).await.map_or(
              Error::InternalServerError,
              |ratelimit| Error::Ratelimit {
                retry_after: ratelimit.retry_after,
              },
            ),

            _ => Error::InternalServerError,
          })
        }
      }

      Err(err) => Err(Error::InternalClientError(err)),
    }
  }

  async fn send<T>(&self, method: Method, url: impl IntoUrl, body: Option<Vec<u8>>) -> Result<T>
  where
    T: DeserializeOwned,
  {
    match self.send_inner(method, url, body.unwrap_or_default()).await {
      Ok(response) => util::parse_json(response).await,

      Err(err) => Err(err),
    }
  }

  /// Gets your project's information.
  ///
  /// # Panics
  ///
  /// Panics if the client uses an invalid API token.
  ///
  /// # Errors
  ///
  /// Returns [`Err`] if:
  /// - The specified bot does not exist. ([`NotFound`][super::Error::NotFound])
  /// - HTTP request failure from the client-side. ([`InternalClientError`][super::Error::InternalClientError])
  /// - HTTP request failure from the server-side. ([`InternalServerError`][super::Error::InternalServerError])
  /// - Ratelimited from sending more requests. ([`Ratelimit`][super::Error::Ratelimit])
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// let project = client.get_self().await.unwrap();
  /// ```
  pub async fn get_self(&self) -> Result<Project> {
    self.send(Method::GET, api!("/projects/@me"), None).await
  }

  /// Updates the application commands list in your Discord bot's Top.gg page.
  ///
  /// # Panics
  ///
  /// Panics if the client uses an invalid API token.
  ///
  /// # Errors
  ///
  /// Returns [`Err`] if:
  /// - Unable to retrieve the list of bot commands. ([`PostBotCommandsError::Retrieval`][super::PostBotCommandsError::Retrieval])
  /// - Unable to serialize the list of bot commands. ([`PostBotCommandsError::Serialization`][super::PostBotCommandsError::Serialization])
  /// - The list of bot commands supplied do not match [Discord API's raw JSON format](https://discord.com/developers/docs/interactions/application-commands#application-command-object). ([`Error::InvalidRequest`][super::Error::InvalidRequest])
  /// - HTTP request failure from the client-side. ([`Error::InternalClientError`][super::Error::InternalClientError])
  /// - HTTP request failure from the server-side. ([`Error::InternalServerError`][super::Error::InternalServerError])
  /// - Ratelimited from sending more requests. ([`Error::Ratelimit`][super::Error::Ratelimit])
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// // Serenity:
  /// client.post_commands(&ctx).await.unwrap();
  ///
  /// // Twilight:
  /// let application_id = bot.current_user_application().await.unwrap().model().await.unwrap().id;
  /// let interaction = bot.interaction(application_id);
  ///
  /// client.post_commands(interaction.global_commands()).await.unwrap();
  ///
  /// // Others:
  /// let commands = vec![...]; // Array of application commands that
  ///                           // can be serialized to Discord API's raw JSON format.
  /// client.post_commands(commands).await.unwrap();
  /// ```
  pub async fn post_commands<L, C, E>(&self, context: C) -> PostBotCommandsResult<(), E>
  where
    L: Serialize + DeserializeOwned,
    C: GetCommands<L, E>,
  {
    let commands = context
      .get_commands()
      .await
      .map_err(PostBotCommandsError::Retrieval)?;

    match self
      .send_inner(
        Method::POST,
        api!("/projects/@me/commands"),
        serde_json::to_vec(&commands).map_err(PostBotCommandsError::Serialization)?,
      )
      .await
    {
      Ok(_) => Ok(()),

      Err(err) => Err(PostBotCommandsError::Request(err)),
    }
  }
}
