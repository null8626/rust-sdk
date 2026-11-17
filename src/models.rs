// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

#[cfg(feature = "api")]
use super::util;
#[cfg(feature = "api")]
use chrono::{DateTime, Utc};
#[cfg(feature = "api")]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;
#[cfg(feature = "api")]
use std::{
  fmt::{self, Debug, Display, Formatter},
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

macro_rules! custom_enum_serde {
  (
    $(#[$enum_metas:meta])*
    pub enum $enum_name:ident {
      $(
        $(#[$enum_prop_metas:meta])*
        $enum_prop_name:ident => $enum_prop_mapping:ident,
      )*
    }
  ) => {
    $(#[$enum_metas])*
    pub enum $enum_name {
      $(
        $(#[$enum_prop_metas])*
        $enum_prop_name,
      )*

      /// Unknown.
      Unknown,
    }

    impl FromStr for $enum_name {
      type Err = ();

      fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
          $(stringify!($enum_prop_mapping) => Ok(Self::$enum_prop_name),)*

          _ => Ok(Self::Unknown),
        }
      }
    }

    impl<'de> Deserialize<'de> for $enum_name {
      fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
      where
        D: Deserializer<'de>,
      {
        let value = String::deserialize(deserializer)?;

        Ok(FromStr::from_str(&value).unwrap())
      }
    }

    impl $enum_name {
      pub(super) const fn as_str(&self) -> &'static str {
        match self {
          $(Self::$enum_prop_name => stringify!($enum_prop_mapping),)*

          Self::Unknown => "unknown",
        }
      }
    }

    impl Serialize for $enum_name {
      fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
      where
        S: Serializer,
      {
        serializer.serialize_str(self.as_str())
      }
    }
  };
}

#[cfg(feature = "api")]
macro_rules! custom_enum_de {
  (
    $(#[$enum_metas:meta])*
    pub enum $enum_name:ident {
      $(
        $(#[$enum_prop_metas:meta])*
        $enum_prop_name:ident => $enum_prop_mapping:ident,
      )*
    }
  ) => {
    $(#[$enum_metas])*
    pub enum $enum_name {
      $(
        $(#[$enum_prop_metas])*
        $enum_prop_name,
      )*

      /// Unknown.
      Unknown,
    }

    impl FromStr for $enum_name {
      type Err = ();

      fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
          $(stringify!($enum_prop_mapping) => Ok(Self::$enum_prop_name),)*

          _ => Ok(Self::Unknown),
        }
      }
    }

    impl<'de> Deserialize<'de> for $enum_name {
      fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
      where
        D: Deserializer<'de>,
      {
        let value = String::deserialize(deserializer)?;

        Ok(FromStr::from_str(&value).unwrap())
      }
    }
  };
}

#[cfg(feature = "api")]
macro_rules! custom_enum_ser {
  (
    $(#[$enum_metas:meta])*
    pub enum $enum_name:ident {
      $(
        $(#[$enum_prop_metas:meta])*
        $enum_prop_name:ident => $enum_prop_mapping:ident,
      )*
    }
  ) => {
    $(#[$enum_metas])*
    pub enum $enum_name {
      $(
        $(#[$enum_prop_metas])*
        $enum_prop_name,
      )*
    }

    impl $enum_name {
      pub(super) const fn as_str(&self) -> &'static str {
        match self {
          $(Self::$enum_prop_name => stringify!($enum_prop_mapping),)*
        }
      }
    }

    impl Serialize for $enum_name {
      fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
      where
        S: Serializer,
      {
        serializer.serialize_str(self.as_str())
      }
    }
  };
}

custom_enum_serde! {
  /// A project's platform.
  #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
  pub enum Platform {
    /// Discord.
    Discord => discord,

    /// Roblox.
    Roblox => roblox,
  }
}

#[cfg(feature = "api")]
custom_enum_serde! {
  /// A project's type.
  #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
  pub enum ProjectType {
    /// A bot.
    Bot => bot,

    /// A server.
    Server => server,

    /// A game.
    Game => game,
  }
}

#[cfg(feature = "api")]
custom_enum_ser! {
  /// A supported locale.
  #[non_exhaustive]
  #[derive(Copy, Clone, Eq, Hash, PartialEq)]
  pub enum Locale {
    /// English.
    English => en,

    /// German.
    German => de,

    /// French.
    French => fr,

    /// Portuguese.
    Portuguese => pt,

    /// Turkish.
    Turkish => tr,

    /// Hindi.
    Hindi => hi,

    /// Japanese.
    Japanese => ja,

    /// Arabic.
    Arabic => ar,

    /// Dutch.
    Dutch => nl,

    /// Korean.
    Korean => ko,

    /// Italian.
    Italian => it,

    /// Spanish.
    Spanish => es,

    /// Russian.
    Russian => ru,

    /// Ukrainian.
    Ukrainian => uk,

    /// Vietnamese.
    Vietnamese => vi,

    /// Chinese Simplified.
    ChineseSimplified => zh,
  }
}

#[cfg(feature = "api")]
custom_enum_ser! {
  /// An announcement category.
  #[derive(Copy, Clone, Eq, Hash, PartialEq)]
  pub enum AnnouncementCategory {
    /// A generic announcement.
    Announcement => announcement,

    /// An event.
    Event => event,

    /// A new feature.
    NewFeature => new_feature,
  }
}

#[cfg(feature = "api")]
custom_enum_ser! {
  /// A user's source platform.
  #[non_exhaustive]
  #[derive(Copy, Clone, Eq, Hash, PartialEq)]
  pub enum UserSource {
    /// The user came from Top.gg
    Topgg => topgg,

    /// The user came from Discord.
    Discord => discord,
  }
}

#[cfg(feature = "api")]
custom_enum_de! {
  /// A project's review status.
  #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
  pub enum ReviewStatus {
    /// The project is currently a draft.
    Draft => draft,

    /// The project is currently in the reviewer queue.
    InReview => in_review,

    /// The project is approved and listed.
    Approved => approved,
  }
}

/// A partial project.
#[cfg(feature = "api")]
#[derive(Clone, Debug, Deserialize)]
pub struct PartialProject {
  /// The project's Top.gg ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub id: u64,

  /// The project's platform ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub platform_id: u64,

  /// The project's name in its platform.
  pub name: String,

  /// The project's platform.
  pub platform: Platform,

  /// The project's type.
  pub r#type: ProjectType,
}

/// A project.
#[cfg(feature = "api")]
#[derive(Clone, Debug, Deserialize)]
pub struct Project {
  /// The project's Top.gg ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub id: u64,

  /// The project's name in its platform.
  pub name: String,

  /// The project's platform.
  pub platform: Platform,

  /// The project's type.
  pub r#type: ProjectType,

  /// The project's headline.
  pub headline: String,

  /// The project's tags.
  pub tags: Vec<String>,

  /// The project's current vote count.
  #[serde(rename = "votes")]
  pub current_votes: usize,

  /// The project's vote count.
  #[serde(rename = "votes_total")]
  pub total_votes: usize,

  /// The project's review score.
  pub review_score: f32,

  /// The project's review count.
  pub review_count: usize,
}

/// A Discord bot's metrics.
#[derive(Copy, Clone, Debug, Default, Serialize)]
#[cfg(feature = "api")]
pub struct DiscordBotMetrics {
  /// The bot's server count.
  pub server_count: Option<usize>,

  /// The bot's shard count.
  pub shard_count: Option<usize>,
}

/// A Discord server's metrics.
#[derive(Copy, Clone, Debug, Default, Serialize)]
#[cfg(feature = "api")]
pub struct DiscordServerMetrics {
  /// The server's total member count.
  pub member_count: Option<usize>,

  /// The server's online member count.
  pub online_count: Option<usize>,
}

/// A Roblox game's metrics.
#[derive(Copy, Clone, Debug, Default, Serialize)]
#[cfg(feature = "api")]
pub struct RobloxGameMetrics {
  /// The game's player count.
  pub player_count: usize,
}

#[cfg(feature = "api")]
#[derive(Clone, Debug, Serialize)]
struct RawTimestampedMetrics {
  timestamp: DateTime<Utc>,
  metrics: serde_json::Value,
}

/// A project's timestamped metrics.
#[cfg(feature = "api")]
#[derive(Clone)]
pub struct TimestampedMetrics<M: Clone + Debug + Default + Serialize> {
  data: Vec<RawTimestampedMetrics>,
  _phantom: PhantomData<M>,
}

#[cfg(feature = "api")]
impl<M> TimestampedMetrics<M>
where
  M: Clone + Debug + Default + Serialize,
{
  /// Inserts a new timestamped metrics entry.
  ///
  /// # Errors
  ///
  /// If unable to serialize the metrics data to a JSON.
  pub fn add(&mut self, timestamp: DateTime<Utc>, metrics: M) -> Result<(), serde_json::Error> {
    self.data.push(RawTimestampedMetrics {
      timestamp,
      metrics: serde_json::to_value(metrics)?,
    });

    Ok(())
  }

  /// Retrieves the amount of timestamped metrics entries.
  #[must_use]
  pub const fn len(&self) -> usize {
    self.data.len()
  }

  /// Whether there are no timestamped metrics entries.
  #[must_use]
  pub const fn is_empty(&self) -> bool {
    self.data.is_empty()
  }

  /// Clears all timestamped metrics entries.
  pub fn clear(&mut self) {
    self.data.clear();
  }
}

#[cfg(feature = "api")]
impl<M> Default for TimestampedMetrics<M>
where
  M: Clone + Debug + Default + Serialize,
{
  fn default() -> Self {
    Self {
      data: vec![],
      _phantom: PhantomData,
    }
  }
}

#[cfg(feature = "api")]
impl<M> Serialize for TimestampedMetrics<M>
where
  M: Clone + Debug + Default + Serialize,
{
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    self.data.serialize(serializer)
  }
}

/// A partial vote.
#[cfg(feature = "api")]
#[derive(Clone, Debug, Deserialize)]
pub struct PartialVote {
  /// The vote's weight (1 normally, 2 during weekend multiplier.)
  pub weight: u8,

  /// When the vote was cast.
  pub created_at: DateTime<Utc>,

  /// When the user can vote again.
  pub expires_at: DateTime<Utc>,
}

/// A vote.
#[cfg(feature = "api")]
#[derive(Clone, Debug, Deserialize)]
pub struct Vote {
  /// The vote's weight (1 normally, 2 during weekend multiplier.)
  pub weight: u8,

  /// When the vote was cast.
  pub created_at: DateTime<Utc>,

  /// When the user can vote again.
  pub expires_at: DateTime<Utc>,

  /// The voter's Top.gg ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub user_id: u64,

  /// The voter's platform ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub platform_id: u64,
}

/// An integration.
#[cfg(feature = "api")]
#[derive(Clone, Debug, Deserialize)]
pub struct Integration {
  /// The integration's Top.gg ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub id: u64,

  /// The integration's name.
  pub name: String,

  /// The integration's description.
  pub description: String,

  /// The integration's icon URL.
  pub icon_url: String,

  /// Whether the integration is connected to the project.
  pub connected: bool,
}

/// A base webhook.
#[cfg(feature = "api")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BaseWebhook {
  /// The webhook's label.
  pub label: String,

  /// The webhook's URL.
  pub url: String,
}

/// A webhook.
#[cfg(feature = "api")]
#[derive(Clone, Debug, Deserialize)]
pub struct Webhook {
  /// The webhook's label.
  pub label: String,

  /// The webhook's URL.
  pub url: String,

  /// The webhook's Top.gg ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub id: u64,

  /// The webhook's secret.
  #[serde(default)]
  pub secret: Option<String>,
}

/// A user's platform connection.
#[cfg(feature = "api")]
#[derive(Clone, Debug, Deserialize)]
pub struct UserConnection {
  /// The connection's platform.
  pub platform: Platform,

  /// The user's platform ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub id: u64,
}

/// A user.
#[cfg(feature = "oauth2")]
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(doc_cfg, doc(cfg(feature = "oauth2")))]
pub struct User {
  /// The user's Top.gg ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub id: u64,

  /// The user's username.
  pub username: String,

  /// The user's avatar URL.
  #[serde(default)]
  pub avatar: Option<String>,

  /// The user's platform connections.
  pub connections: Vec<UserConnection>,
}

/// A user's project.
#[cfg(feature = "oauth2")]
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(doc_cfg, doc(cfg(feature = "oauth2")))]
pub struct UserProject {
  /// The project's headline.
  pub headline: String,

  /// The project's review status.
  pub review_status: ReviewStatus,
}

/// A project submission.
#[derive(Clone, Debug, Serialize)]
#[cfg(feature = "oauth2")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "oauth2")))]
pub struct ProjectSubmission {
  /// The project's platform.
  pub platform: Platform,

  /// The project's type.
  pub r#type: ProjectType,

  /// The project's platform ID.
  #[serde(serialize_with = "util::serialize_snowflake")]
  pub platform_id: u64,

  /// The project's headline.
  pub headline: String,

  /// The project's page content.
  pub page_content: String,
}

/// An API call result that may contain paginated data.
#[cfg(feature = "api")]
#[derive(Clone, Debug, Deserialize)]
pub struct Paginated<T: DeserializeOwned> {
  pub(super) cursor: String,

  #[serde(alias = "projects")]
  data: Vec<T>,
}

#[cfg(feature = "api")]
impl<T> Paginated<T>
where
  T: DeserializeOwned,
{
  /// Whether this is the last page.
  #[must_use]
  pub const fn is_last(&self) -> bool {
    self.data.is_empty()
  }
}

#[cfg(feature = "api")]
impl<T> Deref for Paginated<T>
where
  T: DeserializeOwned,
{
  type Target = Vec<T>;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

#[cfg(feature = "api")]
impl<T> DerefMut for Paginated<T>
where
  T: DeserializeOwned,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.data
  }
}

/// A Top.gg snowflake.
#[cfg(feature = "api")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Snowflake {
  /// The snowflake's value.
  Snowflake(u64),

  /// `@me`.
  Me,
}

#[cfg(feature = "api")]
impl From<u64> for Snowflake {
  fn from(value: u64) -> Self {
    Self::Snowflake(value)
  }
}

#[cfg(feature = "api")]
impl FromStr for Snowflake {
  type Err = <u64 as FromStr>::Err;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    if value == "@me" {
      return Ok(Self::Me);
    }

    Ok(Self::Snowflake(value.parse()?))
  }
}

#[cfg(feature = "api")]
impl Display for Snowflake {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Self::Snowflake(value) => Display::fmt(value, f),

      Self::Me => f.write_str("@me"),
    }
  }
}
