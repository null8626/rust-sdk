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

      /// The discriminator of this bot.
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

      /// A list of IDs of the guilds featured on this bot's page.
      #[serde(default, deserialize_with = "snowflake::deserialize_vec")]
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

      /// Whether this bot is [Top.gg](https://top.gg) certified or not.
      #[serde(rename = "certifiedBot")]
      is_certified: bool,

      /// A list of this bot's shards.
      #[serde(default, deserialize_with = "util::deserialize_default")]
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

      shard_count: Option<usize>,

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
        util::get_avatar(&self.avatar, self.id, Some(&self.discriminator))
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

      /// The amount of shards this bot has according to posted stats.
      #[must_use]
      #[inline(always)]
      shard_count: usize => {
        self.shard_count.unwrap_or(self.shards.len())
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
  /// A struct representing a bot's statistics.
  ///
  /// # Examples
  ///
  /// Solely from a server count:
  ///
  /// ```rust,no_run
  /// use topgg::Stats;
  ///
  /// let _stats = Stats::from(12345);
  /// ```
  ///
  /// Server count with a shard count:
  ///
  /// ```rust,no_run
  /// use topgg::Stats;
  ///
  /// let server_count = 12345;
  /// let shard_count = 10;
  /// let _stats = Stats::from_count(server_count, Some(shard_count));
  /// ```
  ///
  /// Solely from shards information:
  ///
  /// ```rust,no_run
  /// use topgg::Stats;
  ///
  /// // the shard posting this data has 456 servers.
  /// let _stats = Stats::from_shards([123, 456, 789], Some(1));
  /// ```
  #[must_use]
  #[derive(Clone, Serialize, Deserialize)]
  Stats {
    protected {
      #[serde(skip_serializing_if = "Option::is_none")]
      shard_count: Option<usize>,
      #[serde(skip_serializing_if = "Option::is_none")]
      server_count: Option<usize>,
    }

    private {
      #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "util::deserialize_default")]
      shards: Option<Vec<usize>>,
      #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "util::deserialize_default")]
      shard_id: Option<usize>,
    }

    getters(self) {
      /// An array of this bot's server count for each shard.
      #[must_use]
      #[inline(always)]
      shards: &[usize] => {
        self.shards.as_deref().unwrap_or_default()
      }

      /// The amount of shards this bot has.
      #[must_use]
      #[inline(always)]
      shard_count: usize => {
        self.shard_count.unwrap_or(self.shards().len())
      }

      /// The amount of servers this bot is in. `None` if such information is publicly unavailable.
      #[must_use]
      server_count: Option<usize> => {
        self.server_count.or_else(|| {
          self.shards.as_ref().and_then(|shards| {
            if shards.is_empty() {
              None
            } else {
              Some(shards.iter().copied().sum())
            }
          })
        })
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

  /// Creates a [`Stats`] struct based on total server and optionally, shard count data.
  pub const fn from_count(server_count: usize, shard_count: Option<usize>) -> Self {
    Self {
      server_count: Some(server_count),
      shard_count,
      shards: None,
      shard_id: None,
    }
  }

  /// Creates a [`Stats`] struct based on an array of server count per shard and optionally the index (to the array) of shard posting this data.
  ///
  /// # Panics
  ///
  /// Panics if the shard_index argument is [`Some`] yet it's out of range of the `shards` array.
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::Stats;
  ///
  /// // the shard posting this data has 456 servers.
  /// let _stats = Stats::from_shards([123, 456, 789], Some(1));
  /// ```
  pub fn from_shards<A>(shards: A, shard_index: Option<usize>) -> Self
  where
    A: IntoIterator<Item = usize>,
  {
    let mut total_server_count = 0;
    let shards = shards.into_iter();
    let mut shards_list = Vec::with_capacity(shards.size_hint().0);

    for server_count in shards {
      total_server_count += server_count;
      shards_list.push(server_count);
    }

    if let Some(index) = shard_index {
      assert!(index < shards_list.len(), "Shard index out of range.");
    }

    Self {
      server_count: Some(total_server_count),
      shard_count: Some(shards_list.len()),
      shards: Some(shards_list),
      shard_id: shard_index,
    }
  }
}

/// Creates a [`Stats`] struct solely from a server count.
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
