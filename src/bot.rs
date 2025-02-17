use crate::{snowflake, util};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

#[inline(always)]
pub(crate) fn deserialize_support_server<'de, D>(
  deserializer: D,
) -> Result<Option<String>, D::Error>
where
  D: Deserializer<'de>,
{
  util::deserialize_optional_string(deserializer)
    .map(|inner| inner.map(|support| format!("https://discord.com/invite/{support}")))
}

// TODO: remove these utility deprecation helpers soon

#[inline(always)]
fn deserialize_discriminator<'de, D>(_deserializer: D) -> Result<String, D::Error>
where
  D: Deserializer<'de>,
{
  Ok(String::from('0'))
}

util::debug_struct! {
  /// A struct representing a bot listed on [Top.gg](https://top.gg).
  #[must_use]
  #[derive(Clone, Deserialize)]
  Bot {
    public {
      /// The ID of this bot.
      #[serde(deserialize_with = "snowflake::deserialize")]
      id: u64,

      /// The username of this bot.
      username: String,

      #[serde(deserialize_with = "deserialize_discriminator")]
      #[deprecated(since = "1.4.3", note = "No longer supported by Top.gg API v0. At the moment, this will always be '0'.")]
      discriminator: String,

      /// The prefix of this bot.
      prefix: String,

      /// The short description of this bot.
      #[serde(rename = "shortdesc")]
      short_description: String,

      /// The long description of this bot. It can contain HTML and/or Markdown.
      #[serde(
        default,
        deserialize_with = "util::deserialize_optional_string",
        rename = "longdesc"
      )]
      long_description: Option<String>,

      /// The tags of this bot.
      #[serde(default, deserialize_with = "util::deserialize_default")]
      tags: Vec<String>,

      /// The website URL of this bot.
      #[serde(default, deserialize_with = "util::deserialize_optional_string")]
      website: Option<String>,

      /// The link to this bot's GitHub repository.
      #[serde(default, deserialize_with = "util::deserialize_optional_string")]
      github: Option<String>,

      /// A list of IDs of this bot's owners. The main owner is the first ID in the array.
      #[serde(deserialize_with = "snowflake::deserialize_vec")]
      owners: Vec<u64>,

      #[serde(deserialize_with = "util::deserialize_immediate_default")]
      #[deprecated(since = "1.4.3", note = "No longer supported by Top.gg API v0. At the moment, this will always be an empty vector.")]
      guilds: Vec<u64>,

      /// The URL for this bot's banner image.
      #[serde(
        default,
        deserialize_with = "util::deserialize_optional_string",
        rename = "bannerUrl"
      )]
      banner_url: Option<String>,

      /// The date when this bot was approved on [Top.gg](https://top.gg).
      #[serde(rename = "date")]
      approved_at: DateTime<Utc>,

      #[serde(deserialize_with = "util::deserialize_immediate_default")]
      #[deprecated(since = "1.4.3", note = "No longer supported by Top.gg API v0. At the moment, this will always be false.")]
      is_certified: bool,

      #[serde(deserialize_with = "util::deserialize_immediate_default")]
      #[deprecated(since = "1.4.3", note = "No longer supported by Top.gg API v0. At the moment, this will always be an empty vector.")]
      shards: Vec<usize>,

      /// The amount of upvotes this bot has.
      #[serde(rename = "points")]
      votes: usize,

      /// The amount of upvotes this bot has this month.
      #[serde(rename = "monthlyPoints")]
      monthly_votes: usize,

      /// The support server invite URL of this bot.
      #[serde(default, deserialize_with = "deserialize_support_server")]
      support: Option<String>,
    }

    private {
      #[serde(default, deserialize_with = "util::deserialize_optional_string")]
      avatar: Option<String>,

      #[serde(default, deserialize_with = "util::deserialize_optional_string")]
      invite: Option<String>,

      #[serde(default, deserialize_with = "util::deserialize_optional_string")]
      vanity: Option<String>,
    }

    getters(self) {
      /// Retrieves the creation date of this bot.
      #[must_use]
      #[inline(always)]
      created_at: DateTime<Utc> => {
        util::get_creation_date(self.id)
      }

      /// Retrieves the avatar URL of this bot.
      ///
      /// Its format will either be PNG or GIF if animated.
      #[must_use]
      #[inline(always)]
      avatar: String => {
        util::get_avatar(&self.avatar, self.id)
      }

      /// The invite URL of this bot.
      #[must_use]
      invite: String => {
        match &self.invite {
          Some(inv) => inv.to_owned(),
          _ => format!(
            "https://discord.com/oauth2/authorize?scope=bot&client_id={}",
            self.id
          ),
        }
      }

      #[deprecated(since = "1.4.3", note = "No longer supported by Top.gg API v0. At the moment, this will always return 0.")]
      shard_count: usize => {
        0
      }

      /// Retrieves the URL of this bot's [Top.gg](https://top.gg) page.
      #[must_use]
      #[inline(always)]
      url: String => {
        format!(
          "https://top.gg/bot/{}",
          self.vanity.as_deref().unwrap_or(&self.id.to_string())
        )
      }
    }
  }
}

util::debug_struct! {
  #[derive(Clone, Serialize, Deserialize)]
  #[deprecated(since = "1.4.3", note = "No longer has a use by Top.gg API v0. Soon, all you need is just your bot's server count (usize).")]
  Stats {
    protected {
      #[serde(skip_serializing_if = "Option::is_none")]
      server_count: Option<usize>,
    }

    getters(self) {
      #[deprecated(since = "1.4.3", note = "No longer supported by Top.gg API v0. At the moment, this will always return an empty slice.")]
      shards: &[usize] => {
        &[]
      }

      #[deprecated(since = "1.4.3", note = "No longer supported by Top.gg API v0. At the moment, this will always return 0.")]
      shard_count: usize => {
        0
      }

      server_count: Option<usize> => {
        self.server_count
      }
    }
  }
}

impl Stats {
  /// Creates a [`Stats`] struct from the cache of a serenity [`Context`][serenity::client::Context].
  #[inline(always)]
  #[cfg(feature = "serenity-cached")]
  #[cfg_attr(docsrs, doc(cfg(feature = "serenity-cached")))]
  pub fn from_context(context: &serenity::client::Context) -> Self {
    Self::from_count(
      context.cache.guilds().len(),
      Some(context.cache.shard_count() as _),
    )
  }

  #[deprecated(
    since = "1.4.3",
    note = "The shard_count argument no longer has an effect."
  )]
  pub const fn from_count(server_count: usize, _shard_count: Option<usize>) -> Self {
    Self {
      server_count: Some(server_count),
    }
  }

  #[deprecated(
    since = "1.4.3",
    note = "No longer supported by Top.gg API v0. At the moment, the shard_index argument has no effect."
  )]
  pub fn from_shards<A>(shards: A, _shard_index: Option<usize>) -> Self
  where
    A: IntoIterator<Item = usize>,
  {
    let mut total_server_count = 0;
    let shards = shards.into_iter();

    for server_count in shards {
      total_server_count += server_count;
    }

    Self {
      server_count: Some(total_server_count),
    }
  }
}

impl From<usize> for Stats {
  #[inline(always)]
  fn from(server_count: usize) -> Self {
    Self::from_count(server_count, None)
  }
}

#[derive(Deserialize)]
pub(crate) struct IsWeekend {
  pub(crate) is_weekend: bool,
}
