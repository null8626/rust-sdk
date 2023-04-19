use crate::{
  bot::{Bot, BotStats, Bots, IsWeekend, NewBotStats, QueryLike},
  http::{Http, GET, POST},
  user::{User, Voted, Voter},
  Result, SnowflakeLike,
};
use core::mem::transmute;

cfg_if::cfg_if! {
  if #[cfg(feature = "autoposter")] {
    use crate::Autoposter;
    use std::sync::Arc;

    type SyncedClient = Arc<InnerClient>;
  } else {
    type SyncedClient = InnerClient;
  }
}

pub(crate) struct InnerClient {
  http: Http,
}

// this is implemented here because autoposter needs to access this function from a different thread

impl InnerClient {
  pub(crate) async fn post_bot_stats(&self, id: u64, new_stats: &NewBotStats) -> Result<()> {
    let path = format!("/bots/{id}/stats");
    let body = unsafe { serde_json::to_string(&new_stats).unwrap_unchecked() };

    self.http.request(POST, &path, Some(&body)).await?;

    Ok(())
  }
}

/// A struct representing a [top.gg](https://top.gg) API client instance.
pub struct Client {
  inner: SyncedClient,
}

impl Client {
  /// Creates a brand new client instance from a [top.gg](https://top.gg) token.
  ///
  /// You can get a [top.gg](https://top.gg) token if you own a listed discord bot on [top.gg](https://top.gg) (open the edit page, see in `Webhooks` section)
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::Client;
  ///
  /// #[tokio::main]
  /// async fn main() {
  ///   let token = env!("TOPGG_TOKEN").to_owned();
  ///   let _client = Client::new(token);
  /// }
  /// ```
  #[must_use]
  #[inline(always)]
  pub fn new(token: String) -> Self {
    let inner = InnerClient {
      http: Http::new(token),
    };

    #[cfg(feature = "autoposter")]
    let inner = Arc::new(inner);

    Self { inner }
  }

  /// Fetches a user from a Discord ID if available.
  ///
  /// # Panics
  ///
  /// Panics if the following conditions are met:
  /// - The ID argument is a string but not numeric
  /// - The client uses an invalid [top.gg](https://top.gg) API token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to the [top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The requested user does not exist ([`NotFound`][crate::Error::NotFound])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::Client;
  ///
  /// #[tokio::main]
  /// async fn main() {
  ///   let token = env!("TOPGG_TOKEN").to_owned();
  ///   let client = Client::new(token);
  ///   
  ///   let user = client.get_user(661200758510977084u64).await.unwrap();
  ///   
  ///   assert_eq!(user.username, "null");
  ///   assert_eq!(user.discriminator, "8626");
  ///   assert_eq!(user.id, 661200758510977084u64);
  ///   
  ///   println!("{:?}", user);
  /// }
  /// ```
  pub async fn get_user<I>(&self, id: I) -> Result<User>
  where
    I: SnowflakeLike,
  {
    let path = format!("/users/{}", id.as_snowflake());

    self.inner.http.request(GET, &path, None).await
  }

  /// Fetches a listed discord bot from a Discord ID if available.
  ///
  /// # Panics
  ///
  /// Panics if the following conditions are met:
  /// - The ID argument is a string but not numeric
  /// - The client uses an invalid [top.gg](https://top.gg) API token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to the [top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The requested discord bot is not listed on [top.gg](https://top.gg) ([`NotFound`][crate::Error::NotFound])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::Client;
  ///
  /// #[tokio::main]
  /// async fn main() {
  ///   let token = env!("TOPGG_TOKEN").to_owned();
  ///   let client = Client::new(token);
  ///   
  ///   let bot = client.get_bot(264811613708746752u64).await.unwrap();
  ///   
  ///   assert_eq!(bot.username, "Luca");
  ///   assert_eq!(bot.discriminator, "1375");
  ///   assert_eq!(bot.id, 264811613708746752u64);
  ///   
  ///   println!("{:?}", bot);
  /// }
  /// ```
  pub async fn get_bot<I>(&self, id: I) -> Result<Bot>
  where
    I: SnowflakeLike,
  {
    let path = format!("/bots/{}", id.as_snowflake());

    self.inner.http.request(GET, &path, None).await
  }

  /// Fetches your own discord bot's statistics.
  ///
  /// # Panics
  ///
  /// Panics if the client uses an invalid [top.gg](https://top.gg) API token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to the [top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The requested discord bot is not listed on [top.gg](https://top.gg) ([`NotFound`][crate::Error::NotFound])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::Client;
  ///
  /// #[tokio::main]
  /// async fn main() {
  ///   let token = env!("TOPGG_TOKEN").to_owned();
  ///   let client = Client::new(token);
  ///   
  ///   let stats = client.get_bot_stats().await.unwrap();
  ///   
  ///   println!("{:?}", stats);
  /// }
  /// ```
  #[inline(always)]
  pub async fn get_bot_stats(&self) -> Result<BotStats> {
    self.inner.http.request(GET, "/bots/stats", None).await
  }

  /// Posts an owned discord bot's statistics.
  ///
  /// # Panics
  ///
  /// Panics if the following conditions are met:
  /// - The ID argument is a string but not numeric
  /// - The client uses an invalid [top.gg](https://top.gg) API token (unauthorized)
  /// - The client posts statistics to an external discord bot not owned by the owner. (forbidden)
  ///
  /// # Errors
  ///
  /// Errors if the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to the [top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::{Client, NewBotStats};
  ///
  /// #[tokio::main]
  /// async fn main() {
  ///   let token = env!("TOPGG_TOKEN").to_owned();
  ///   let client = Client::new(token);
  ///   let my_bot_id = 123456789u64;
  ///
  ///   let server_count = 1234; // be TRUTHFUL!
  ///   let shard_count = 10;
  ///
  ///   let stats = NewBotStats::count_based(server_count, Some(shard_count));
  ///
  ///   client.post_bot_stats(my_bot_id, stats).await.unwrap();
  /// }
  /// ```
  #[inline(always)]
  pub async fn post_bot_stats<I>(&self, id: I, new_stats: NewBotStats) -> Result<()>
  where
    I: SnowflakeLike,
  {
    self
      .inner
      .post_bot_stats(id.as_snowflake(), &new_stats)
      .await
  }

  /// Creates a new autoposter instance for this client which lets you automate the process of posting your own bot's statistics to the [top.gg](https://top.gg) API.
  ///
  /// # Panics
  ///
  /// Panics if the following conditions are met:
  /// - The ID argument is a string but not numeric
  /// - The delay argument is shorter than 15 minutes (900 seconds)
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::{Autoposter, Client, NewBotStats};
  ///
  /// #[tokio::main]
  /// async fn main() {
  ///   let token = env!("TOPGG_TOKEN").to_owned();
  ///   let client = Client::new(token);
  ///   let my_bot_id = 123456789u64;
  ///
  ///   // make sure to make this autoposter instance live
  ///   // throughout most of the bot's lifetime to keep running!
  ///   let autoposter = client.new_autoposter(my_bot_id, 1800);
  ///
  ///   // ... then in some on ready/new guild event ...
  ///   let server_count = 12345;
  ///   let stats = NewBotStats::count_based(server_count, None);
  ///   autoposter.feed(stats).await;
  /// }
  /// ```
  #[cfg(feature = "autoposter")]
  #[must_use]
  pub fn new_autoposter<I, D>(&self, id: I, seconds_delay: D) -> Autoposter
  where
    I: SnowflakeLike,
    D: Into<u64>,
  {
    let seconds_delay = seconds_delay.into();

    if seconds_delay < 900 {
      panic!("the delay mustn't be shorter than 15 minutes (900 seconds)");
    }

    Autoposter::new(&self.inner, id.as_snowflake(), seconds_delay)
  }

  /// Fetches an owned discord bot's last 1000 voters if available.
  ///
  /// # Panics
  ///
  /// Panics if the following conditions are met:
  /// - The ID argument is a string but not numeric
  /// - The client uses an invalid [top.gg](https://top.gg) API token (unauthorized)
  /// - The client requests an external discord bot not owned by the owner. (forbidden)
  ///
  /// # Errors
  ///
  /// Errors if the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to the [top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::Client;
  ///
  /// #[tokio::main]
  /// async fn main() {
  ///   let token = env!("TOPGG_TOKEN").to_owned();
  ///   let client = Client::new(token);
  ///   let my_bot_id = 123456789u64;
  ///   
  ///   for voter in client.get_bot_voters(my_bot_id).await.unwrap() {
  ///     println!("{:?}", voter);
  ///   }
  /// }
  /// ```
  pub async fn get_bot_voters<I>(&self, id: I) -> Result<Vec<Voter>>
  where
    I: SnowflakeLike,
  {
    let path = format!("/bots/{}/votes", id.as_snowflake());

    self.inner.http.request(GET, &path, None).await
  }

  /// Queries/searches through the [top.gg](https://top.gg) database to look for matching listed discord bots.
  ///
  /// # Panics
  ///
  /// Panics if the following conditions are met:
  /// - The ID argument is a string but not numeric
  /// - The client uses an invalid [top.gg](https://top.gg) API token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to the [top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The requested discord bot is not listed on [top.gg](https://top.gg) ([`NotFound`][crate::Error::NotFound])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::{Client, Filter, Query};
  ///
  /// #[tokio::main]
  /// async fn main() {
  ///   let token = env!("TOPGG_TOKEN").to_owned();
  ///   let client = Client::new(token);
  ///   
  ///   // inputting a string searches a bot that matches that username
  ///   for bot in client.get_bots("shiro").await.unwrap() {
  ///     println!("{:?}", bot);
  ///   }
  ///
  ///   // advanced query with filters
  ///   let filter = Filter::new()
  ///     .username("shiro")
  ///     .certified(true);
  ///
  ///   let query = Query::new()
  ///     .limit(250)
  ///     .skip(50)
  ///     .filter(filter);
  ///
  ///   for bot in client.get_bots(query).await.unwrap() {
  ///     println!("{:?}", bot);
  ///   }
  /// }
  /// ```
  pub async fn get_bots<Q>(&self, query: Q) -> Result<Vec<Bot>>
  where
    Q: QueryLike,
  {
    let path = format!("/bots{}", query.into_query_string());

    Ok(
      self
        .inner
        .http
        .request::<Bots>(GET, &path, None)
        .await?
        .results,
    )
  }

  /// Checks if the specified user has voted for an owned discord bot.
  ///
  /// # Panics
  ///
  /// Panics if the following conditions are met:
  /// - The bot ID argument is a string and it's not a valid ID (expected things like `"123456789"`)
  /// - The user ID argument is a string and it's not a valid ID (expected things like `"123456789"`)
  /// - The client uses an invalid [top.gg](https://top.gg) API token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to the [top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::Client;
  ///
  /// #[tokio::main]
  /// async fn main() {
  ///   let token = env!("TOPGG_TOKEN").to_owned();
  ///   let client = Client::new(token);
  ///   
  ///   let my_bot_id = 123456789u64;
  ///   let user_id = 661200758510977084u64;
  ///
  ///   if client.has_voted(my_bot_id, user_id).await.unwrap() {
  ///     println!("checks out");
  ///   }
  /// }
  /// ```
  pub async fn has_voted<B, U>(&self, bot_id: B, user_id: U) -> Result<bool>
  where
    B: SnowflakeLike,
    U: SnowflakeLike,
  {
    let path = format!(
      "/bots/{}/votes?userId={}",
      bot_id.as_snowflake(),
      user_id.as_snowflake()
    );

    Ok(unsafe {
      transmute(
        self
          .inner
          .http
          .request::<Voted>(GET, &path, None)
          .await?
          .voted,
      )
    })
  }

  /// Checks if the weekend multiplier is active.
  ///
  /// # Panics
  ///
  /// Panics if the client uses an invalid [top.gg](https://top.gg) API token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to the [top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::Client;
  ///
  /// #[tokio::main]
  /// async fn main() {
  ///   let token = env!("TOPGG_TOKEN").to_owned();
  ///   let client = Client::new(token);
  ///   
  ///   if client.is_weekend().await.unwrap() {
  ///     println!("guess what? it's the weekend! woohoo! 🎉🎉🎉🎉");
  ///   }
  /// }
  /// ```
  pub async fn is_weekend(&self) -> Result<bool> {
    Ok(unsafe {
      transmute(
        self
          .inner
          .http
          .request::<IsWeekend>(GET, "/weekend", None)
          .await?
          .is_weekend,
      )
    })
  }
}
