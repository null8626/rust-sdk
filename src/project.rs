use super::snowflake;

use serde::Deserialize;

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

/// A [`Project`]'s platform.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
  Discord,
}

/// A [`Project`]'s type.
#[derive(Deserialize)]
pub enum ProjectType {
  #[serde(rename = "bot")]
  DiscordBot,

  #[serde(rename = "server")]
  DiscordServer,
}
