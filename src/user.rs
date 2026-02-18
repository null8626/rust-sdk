use super::snowflake::{self, Snowflake};

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// A user account from an external platform that is linked to a Top.gg user account.
pub enum UserSource<S> {
  Discord(S),
  Topgg(S),
}

impl<S> UserSource<S> {
  pub(crate) const fn name(&self) -> &'static str {
    match self {
      Self::Topgg(_) => "topgg",

      Self::Discord(_) => "discord",
    }
  }
}

impl<S> Snowflake for UserSource<S>
where
  S: Snowflake,
{
  fn as_snowflake(&self) -> u64 {
    match self {
      Self::Topgg(id) | Self::Discord(id) => id.as_snowflake(),
    }
  }
}

/// A project's vote information.
#[derive(Deserialize)]
pub struct Vote {
  /// The voter's ID.
  #[serde(deserialize_with = "snowflake::deserialize")]
  pub user_id: u64,

  /// The voter's ID on the project's platform.
  #[serde(deserialize_with = "snowflake::deserialize")]
  pub platform_id: u64,

  /// When the vote was cast.
  pub voted_at: DateTime<Utc>,

  /// When the vote expires and the user is required to vote again.
  pub expires_at: DateTime<Utc>,

  /// The vote's weight. 1 during weekdays, 2 during weekends.
  pub weight: u64,
}
