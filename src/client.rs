use crate::{
  bot::{Bot, Bots, GetBots, IsWeekend, Stats},
  user::{User, Voted, Voter},
  util, Error, Result, Snowflake,
};
use reqwest::{header, IntoUrl, Method, Response, StatusCode, Version};
use serde::{de::DeserializeOwned, Deserialize};

cfg_if::cfg_if! {
  if #[cfg(feature = "autoposter")] {
    use crate::autoposter;
    use std::sync::Arc;

    type SyncedClient = Arc<InnerClient>;
  } else {
    type SyncedClient = InnerClient;
  }
}

#[derive(Deserialize)]
#[serde(rename = "kebab-case")]
struct Ratelimit {
  retry_after: u16,
}

macro_rules! api {
  ($e:literal) => {
    concat!("https://top.gg/api", $e)
  };

  ($e:literal, $($rest:tt)*) => {
    format!(api!($e), $($rest)*)
  };
}

#[derive(Debug)]
pub struct InnerClient {
  http: reqwest::Client,
  token: String,
}

// this is implemented here because autoposter needs to access this struct from a different thread.
impl InnerClient {
  pub(crate) fn new(mut token: String) -> Self {
    token.insert_str(0, "Bearer ");

    Self {
      http: reqwest::Client::new(),
      token,
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
            StatusCode::UNAUTHORIZED => panic!("Invalid Top.gg API token."),
            StatusCode::NOT_FOUND => Error::NotFound,
            StatusCode::TOO_MANY_REQUESTS => match util::parse_json::<Ratelimit>(response).await {
              Ok(ratelimit) => Error::Ratelimit {
                retry_after: ratelimit.retry_after,
              },
              _ => Error::InternalServerError,
            },
            _ => Error::InternalServerError,
          })
        }
      }

      Err(err) => Err(Error::InternalClientError(err)),
    }
  }

  #[inline(always)]
  pub(crate) async fn send<T>(
    &self,
    method: Method,
    url: impl IntoUrl,
    body: Option<Vec<u8>>,
  ) -> Result<T>
  where
    T: DeserializeOwned,
  {
    match self.send_inner(method, url, body.unwrap_or_default()).await {
      Ok(response) => util::parse_json(response).await,
      Err(err) => Err(err),
    }
  }

  pub(crate) async fn post_server_count(&self, server_count: usize) -> Result<()> {
    self
      .send_inner(
        Method::POST,
        api!("/bots/stats"),
        serde_json::to_vec(&Stats {
          server_count: Some(server_count),
        })
        .unwrap(),
      )
      .await
      .map(|_| ())
  }
}

/// A struct representing a [Top.gg API](https://docs.top.gg) client instance.
#[must_use]
#[derive(Debug)]
pub struct Client {
  inner: SyncedClient,
}

impl Client {
  /// Creates a brand new client instance from a [Top.gg](https://top.gg) token.
  ///
  /// To get your [Top.gg](https://top.gg) token, [view this tutorial](https://github.com/top-gg/rust-sdk/assets/60427892/d2df5bd3-bc48-464c-b878-a04121727bff).
  #[inline(always)]
  pub fn new(token: String) -> Self {
    let inner = InnerClient::new(token);

    #[cfg(feature = "autoposter")]
    let inner = Arc::new(inner);

    Self { inner }
  }

  /// Fetches a user from a Discord ID.
  ///
  /// # Panics
  ///
  /// Panics if any of the following conditions are met:
  /// - The ID argument is a string but not numeric
  /// - The client uses an invalid [Top.gg API](https://docs.top.gg) token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if any of the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to [Top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [Top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The requested user does not exist ([`NotFound`][crate::Error::NotFound])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  pub async fn get_user<I>(&self, id: I) -> Result<User>
  where
    I: Snowflake,
  {
    self
      .inner
      .send(Method::GET, api!("/users/{}", id.as_snowflake()), None)
      .await
  }

  /// Fetches a listed bot from a Discord ID.
  ///
  /// # Panics
  ///
  /// Panics if any of the following conditions are met:
  /// - The ID argument is a string but not numeric
  /// - The client uses an invalid [Top.gg API](https://docs.top.gg) token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if any of the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to [Top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [Top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The requested bot is not listed on [Top.gg](https://top.gg) ([`NotFound`][crate::Error::NotFound])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  pub async fn get_bot<I>(&self, id: I) -> Result<Bot>
  where
    I: Snowflake,
  {
    self
      .inner
      .send(Method::GET, api!("/bots/{}", id.as_snowflake()), None)
      .await
  }

  /// Fetches your bot's posted server count.
  ///
  /// # Panics
  ///
  /// Panics if the client uses an invalid [Top.gg API](https://docs.top.gg) token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if any of the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to [Top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [Top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  pub async fn get_server_count(&self) -> Result<Option<usize>> {
    self
      .inner
      .send(Method::GET, api!("/bots/stats"), None)
      .await
      .map(|stats: Stats| stats.server_count)
  }

  /// Posts your bot's server count.
  ///
  /// # Panics
  ///
  /// Panics if the client uses an invalid [Top.gg API](https://docs.top.gg) token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if any of the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to [Top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [Top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  #[inline(always)]
  pub async fn post_server_count(&self, server_count: usize) -> Result<()> {
    self.inner.post_server_count(server_count).await
  }

  /// Fetches your bot's last 1000 voters.
  ///
  /// # Panics
  ///
  /// Panics if the client uses an invalid [Top.gg API](https://docs.top.gg) token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if any of the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to [Top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [Top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  pub async fn get_voters(&self) -> Result<Vec<Voter>> {
    self
      .inner
      .send(Method::GET, api!("/bots/votes"), None)
      .await
  }

  pub(crate) async fn get_bots_inner(&self, query: String) -> Result<Vec<Bot>> {
    self
      .inner
      .send::<Bots>(Method::GET, api!("/bots{}", query), None)
      .await
      .map(|res| res.results)
  }

  /// Queries/searches through the [Top.gg](https://top.gg) database to look for matching listed Discord bots.
  ///
  /// # Panics
  ///
  /// Panics if any of the client uses an invalid [Top.gg API](https://docs.top.gg) token (unauthorized).
  ///
  /// # Errors
  ///
  /// Errors if any of the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to [Top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [Top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  ///
  /// # Examples
  ///
  /// Basic usage:
  ///
  /// ```rust,no_run
  /// use topgg::{Client, GetBots};
  ///
  /// let client = Client::new(env!("TOPGG_TOKEN").to_string());
  ///
  /// let bots = client
  ///   .get_bots()
  ///   .limit(250)
  ///   .skip(50)
  ///   .username("shiro")
  ///   .sort_by_monthly_votes()
  ///   .await;
  ///
  /// for bot in bots {
  ///   println!("{:?}", bot);
  /// }
  /// ```
  #[inline(always)]
  pub fn get_bots(&self) -> GetBots<'_> {
    GetBots::new(self)
  }

  /// Checks if the specified user has voted your bot.
  ///
  /// # Panics
  ///
  /// Panics if any of the following conditions are met:
  /// - The user ID argument is a string and it's not a valid ID (expected things like `"123456789"`)
  /// - The client uses an invalid [Top.gg API](https://docs.top.gg) token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if any of the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to [Top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [Top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  pub async fn has_voted<I>(&self, user_id: I) -> Result<bool>
  where
    I: Snowflake,
  {
    self
      .inner
      .send::<Voted>(
        Method::GET,
        api!("/bots/check?userId={}", user_id.as_snowflake()),
        None,
      )
      .await
      .map(|res| res.voted != 0)
  }

  /// Checks if the weekend multiplier is active.
  ///
  /// # Panics
  ///
  /// Panics if the client uses an invalid [Top.gg API](https://docs.top.gg) token (unauthorized)
  ///
  /// # Errors
  ///
  /// Errors if any of the following conditions are met:
  /// - An internal error from the client itself preventing it from sending a HTTP request to [Top.gg](https://top.gg) ([`InternalClientError`][crate::Error::InternalClientError])
  /// - An unexpected response from the [Top.gg](https://top.gg) servers ([`InternalServerError`][crate::Error::InternalServerError])
  /// - The client is being ratelimited from sending more HTTP requests ([`Ratelimit`][crate::Error::Ratelimit])
  pub async fn is_weekend(&self) -> Result<bool> {
    self
      .inner
      .send::<IsWeekend>(Method::GET, api!("/weekend"), None)
      .await
      .map(|res| res.is_weekend)
  }
}

cfg_if::cfg_if! {
  if #[cfg(feature = "autoposter")] {
    impl autoposter::AsClientSealed for Client {
      #[inline(always)]
      fn as_client(&self) -> Arc<InnerClient> {
        Arc::clone(&self.inner)
      }
    }

    impl autoposter::AsClient for Client {}
  }
}
