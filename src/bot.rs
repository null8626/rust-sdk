use crate::{snowflake, util, Client};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
  cmp::min,
  future::{Future, IntoFuture},
  pin::Pin,
};

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
      /// The application ID of this bot.
      #[serde(rename = "clientid", deserialize_with = "snowflake::deserialize")]
      id: u64,

      /// The Top.gg user ID of this bot.
      #[serde(rename = "id", deserialize_with = "snowflake::deserialize")]
      topgg_id: u64,

      /// The username of this bot.
      #[serde(rename = "username")]
      name: String,

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

#[derive(Serialize, Deserialize)]
pub(crate) struct Stats {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) server_count: Option<usize>,
}

#[derive(Deserialize)]
pub(crate) struct Bots {
  pub(crate) results: Vec<Bot>,
}

#[derive(Deserialize)]
pub(crate) struct IsWeekend {
  pub(crate) is_weekend: bool,
}

/// A struct for configuring the query in [`get_bots`][crate::Client::get_bots] before being sent to the [Top.gg API](https://docs.top.gg) by `await`ing it.
#[must_use]
pub struct GetBots<'a> {
  client: &'a Client,
  query: String,
  search: String,
  sort: Option<&'static str>,
}

macro_rules! get_bots_method {
  ($(
    $(#[doc = $doc:literal])*
    $input_name:ident: $input_type:ty = $property:ident($($format:tt)*);
  )*) => {$(
    $(#[doc = $doc])*
    pub fn $input_name(mut self, $input_name: $input_type) -> Self {
      self.$property.push_str(&format!($($format)*));
      self
    }
  )*};
}

macro_rules! get_bots_sort {
  ($(
    $(#[doc = $doc:literal])*
    $func_name:ident: $api_name:ident,
  )*) => {$(
    $(#[doc = $doc])*
    pub fn $func_name(mut self) -> Self {
      self.sort.replace(stringify!($api_name));
      self
    }
  )*};
}

impl<'a> GetBots<'a> {
  #[inline(always)]
  pub(crate) fn new(client: &'a Client) -> Self {
    Self {
      client,
      query: String::from('?'),
      search: String::new(),
      sort: None,
    }
  }

  get_bots_sort! {
    /// Sorts results based on each bot's ID.
    sort_by_id: id,

    /// Sorts results based on each bot's approval date.
    sort_by_approval_date: date,

    /// Sorts results based on each bot's monthly vote count.
    sort_by_monthly_votes: monthlyPoints,
  }

  get_bots_method! {
    /// Sets the maximum amount of bots to be queried. This cannot be more than 500.
    limit: u16 = query("limit={}&", min(limit, 500));

    /// Sets the amount of bots to be skipped during the query. This cannot be more than 499.
    skip: u16 = query("offset={}&", min(skip, 499));

    /// Queries only Discord bots that has this username.
    username: &str = search("username%3A%20{}%20", urlencoding::encode(username));

    /// Queries only Discord bots that has this prefix.
    prefix: &str = search("prefix%3A%20{}%20", urlencoding::encode(prefix));

    /// Queries only Discord bots that has this vote count.
    votes: usize = search("points%3A%20{votes}%20");

    /// Queries only Discord bots that has this monthly vote count.
    monthly_votes: usize = search("monthlyPoints%3A%20{monthly_votes}%20");

    /// Queries only Discord bots that has this [Top.gg](https://top.gg) vanity URL.
    vanity: &str = search("vanity%3A%20{}%20", urlencoding::encode(vanity));
  }
}

impl<'a> IntoFuture for GetBots<'a> {
  type Output = crate::Result<Vec<Bot>>;
  type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;

  fn into_future(self) -> Self::IntoFuture {
    let mut query = self.query;

    if let Some(sort) = self.sort {
      query.push_str(&format!("sort={sort}&"));
    }

    if !self.search.is_empty() {
      query.push_str(&format!("search={}", self.search));
    } else {
      query.pop();
    }

    Box::pin(self.client.get_bots_inner(query))
  }
}
