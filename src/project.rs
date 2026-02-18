use super::snowflake;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// A project listed on Top.gg.
#[derive(Deserialize)]
pub struct Project {
  /// The project's ID.
  #[serde(deserialize_with = "snowflake::deserialize")]
  pub id: u64,

  /// The project's name sourced from the external platform.
  pub name: String,

  /// The project's platform.
  pub platform: Platform,

  /// The project's type.
  #[serde(rename = "type")]
  pub kind: ProjectType,

  /// The project's short description.
  pub headline: String,

  /// The project's tag IDs.
  pub tags: Vec<String>,

  /// The project's current vote count that affects the project's ranking.
  #[serde(rename = "votes")]
  pub current_votes: u64,

  /// The project's total vote count.
  #[serde(rename = "votes_total")]
  pub total_votes: u64,

  /// The project's review score out of 5.
  pub review_score: f64,

  /// The project's total review count.
  pub review_count: u64,
}

/// A project's platform.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
  Discord,
}

/// A project's type.
#[derive(Deserialize)]
pub enum ProjectType {
  #[serde(rename = "bot")]
  DiscordBot,

  #[serde(rename = "server")]
  DiscordServer,
}

/// Retrieves an array of application commands in [Discord API's raw JSON format](https://discord.com/developers/docs/interactions/application-commands#application-command-object). For use in [`Client::post_commands`][super::Client::post_commands].
#[async_trait::async_trait]
pub trait GetCommands<C, E>
where
  C: Serialize + DeserializeOwned,
{
  async fn get_commands(self) -> Result<Vec<C>, E>;
}

cfg_if::cfg_if! {
  if #[cfg(any(feature = "serenity", feature = "serenity-cached"))] {
    use serenity::{
      client::Context as SerenityContext,
      http::{
        HttpError as SerenityHttpError, LightMethod as SerenityHttpMethod,
        Request as SerenityHttpRequest, Route as SerenityHttpRoute,
      },
      Error as SerenityError,
    };

    #[async_trait::async_trait]
    impl GetCommands<serde_json::Value, SerenityError> for &SerenityContext {
      async fn get_commands(self) -> Result<Vec<serde_json::Value>, SerenityError> {
        let Some(application_id) = self.http.application_id() else {
          return Err(SerenityHttpError::ApplicationIdMissing.into());
        };

        self
          .http
          .fire::<_>(SerenityHttpRequest::new(
            SerenityHttpRoute::Commands { application_id },
            SerenityHttpMethod::Get,
          ))
          .await
      }
    }
  }
}

cfg_if::cfg_if! {
  if #[cfg(any(feature = "twilight", feature = "twilight-cached"))] {
    use twilight_http::{error::Error as TwilightHttpError, response::DeserializeBodyError as TwilightHttpDeserializeBodyError, request::application::command::GetGlobalCommands as TwilightGetGlobalCommands};
    use twilight_model::application::command::Command as TwilightCommand;

    #[doc(hidden)]
    #[derive(Debug)]
    pub enum TwilightGetCommandsError {
      Http(TwilightHttpError),
      Deserialize(TwilightHttpDeserializeBodyError),
    }

    #[async_trait::async_trait]
    impl GetCommands<TwilightCommand, TwilightGetCommandsError> for TwilightGetGlobalCommands<'_> {
      async fn get_commands(self) -> Result<Vec<TwilightCommand>, TwilightGetCommandsError> {
        self.await.map_err(TwilightGetCommandsError::Http)?.models().await.map_err(TwilightGetCommandsError::Deserialize)
      }
    }
  }
}
