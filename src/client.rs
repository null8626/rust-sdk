// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

use super::{
  AnnouncementCategory, BaseWebhook, Error, Integration, Locale, Paginated, PartialProject,
  PartialVote, Project, Snowflake, TimestampedMetrics, UserSource, Vote, Webhook, util,
};
#[cfg(feature = "oauth2")]
use super::{ProjectSubmission, User, UserProject};
#[cfg(feature = "oauth2")]
use base64::engine::{Engine, general_purpose::URL_SAFE_NO_PAD};
#[cfg(feature = "oauth2")]
use chrono::Duration;
use chrono::{DateTime, Utc};
#[cfg(test)]
use httpmock::MockServer;
use reqwest::{Client as HttpClient, IntoUrl, Method, Response};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[cfg(feature = "oauth2")]
use sha2::{Digest, Sha256};
#[cfg(any(test, feature = "oauth2"))]
use std::sync::Arc;
use std::{collections::HashMap, fmt::Debug, ops::Deref};
#[cfg(feature = "oauth2")]
use tokio::{
  sync::{RwLock, RwLockReadGuard},
  task::{self, JoinHandle},
  time::sleep,
};
#[cfg(feature = "oauth2")]
use tokio_util::sync::CancellationToken;

#[cfg(not(test))]
macro_rules! api {
  ($server:expr, $e:literal) => {
    concat!("https://top.gg/api/v1", $e)
  };

  ($server:expr, $e:literal, $($rest:tt)*) => {
    format!(api!($server, $e), $($rest)*)
  };
}

#[cfg(test)]
macro_rules! api {
  ($server:expr, $e:literal) => {
    $server.url($e)
  };

  ($server:expr, $e:literal, $($rest:tt)*) => {
    $server.url(format!($e, $($rest)*))
  };
}

#[cfg(not(test))]
#[cfg(feature = "oauth2")]
const REFRESH_INTERVAL: Duration = Duration::weeks(1);

#[cfg(test)]
#[cfg(feature = "oauth2")]
const REFRESH_INTERVAL: Duration = Duration::milliseconds(5500);

#[doc(hidden)]
#[cfg(not(feature = "oauth2"))]
pub type TokenGuard<'t> = &'t str;

#[doc(hidden)]
#[cfg(feature = "oauth2")]
pub enum TokenGuard<'t> {
  /// Direct access.
  Direct(&'t str),

  /// Access through [`RwLock`].
  Guard(RwLockReadGuard<'t, Oauth2Session>),
}

#[cfg(feature = "oauth2")]
impl Deref for TokenGuard<'_> {
  type Target = str;

  fn deref(&self) -> &Self::Target {
    match self {
      Self::Direct(token) => token,

      Self::Guard(guard) => &guard.token,
    }
  }
}

#[doc(hidden)]
#[async_trait::async_trait]
pub trait HttpProvider: Sync {
  /// Retrieves the client's token.
  async fn get_token(&self) -> TokenGuard<'_>;

  /// Retrieves the client's underlying [`reqwest::Client`] instance.
  fn get_http(&self) -> &HttpClient;
}

#[doc(hidden)]
pub struct DirectHttpProvider {
  http: HttpClient,
  token: String,
}

#[async_trait::async_trait]
impl HttpProvider for DirectHttpProvider {
  async fn get_token(&self) -> TokenGuard<'_> {
    #[cfg(feature = "oauth2")]
    {
      TokenGuard::Direct(&self.token)
    }

    #[cfg(not(feature = "oauth2"))]
    &self.token
  }

  fn get_http(&self) -> &HttpClient {
    &self.http
  }
}

#[doc(hidden)]
#[derive(Clone)]
#[cfg(feature = "oauth2")]
pub struct Oauth2HttpProvider {
  oauth2: Arc<BaseOauth2>,
  session: Arc<Oauth2SessionWrapper>,
  #[cfg(test)]
  server: Arc<MockServer>,
}

#[cfg(feature = "oauth2")]
impl Oauth2HttpProvider {
  #[allow(clippy::significant_drop_tightening)]
  async fn refresh_token(&self) -> Result<(), Error> {
    let mut session = self.session.session.write().await;
    let body = format!(
      "grant_type=refresh_token&client_id={}&client_secret={}&refresh_token={}",
      self.oauth2.client_id,
      self.oauth2.client_secret,
      urlencoding::encode(&session.refresh_token)
    )
    .into_bytes();

    let response = util::request(
      &self.oauth2.http,
      Method::POST,
      api!(self.server, "/oauth2/token"),
      None,
      "application/x-www-form-urlencoded",
      body,
    )
    .await?;

    match response.bytes().await {
      Ok(bytes) => match serde_json::from_slice::<Oauth2Session>(&bytes) {
        Ok(new_session) => {
          *session = new_session;

          Ok(())
        }

        Err(error) => Err(Error::Deserialization(error)),
      },

      Err(error) => Err(Error::ReadBody(error)),
    }
  }
}

#[cfg(feature = "oauth2")]
#[async_trait::async_trait]
impl HttpProvider for Oauth2HttpProvider {
  async fn get_token(&self) -> TokenGuard<'_> {
    TokenGuard::Guard(self.session.session.read().await)
  }

  fn get_http(&self) -> &HttpClient {
    &self.oauth2.http
  }
}

#[derive(Serialize)]
#[allow(clippy::redundant_pub_crate)]
pub(super) struct EditProject {
  pub(super) headline: HashMap<Locale, String>,
  pub(super) page_content: HashMap<Locale, String>,
}

#[derive(Serialize)]
#[allow(clippy::redundant_pub_crate)]
pub(super) struct PostAnnouncement {
  pub(super) title: String,
  pub(super) content: String,
  pub(super) category: AnnouncementCategory,
}

#[derive(Serialize)]
#[allow(clippy::redundant_pub_crate)]
pub(super) struct PostTimestampedMetrics<M: Clone + Debug + Default + Serialize> {
  pub(super) data: TimestampedMetrics<M>,
}

#[derive(Deserialize)]
struct RotateWebhookSecret {
  secret: String,
}

/// A base API client.
pub struct BaseClient<P> {
  provider: P,
  #[cfg(test)]
  server: Arc<MockServer>,
}

impl<P> BaseClient<P>
where
  P: HttpProvider,
{
  #[cfg_attr(not(feature = "oauth2"), allow(clippy::borrow_deref_ref))]
  async fn send_inner(
    &self,
    method: Method,
    url: impl IntoUrl,
    body: Vec<u8>,
  ) -> Result<Response, Error> {
    let http = self.provider.get_http();
    let token = self.provider.get_token().await;

    util::request(http, method, url, Some(&*token), "application/json", body).await
  }

  async fn send<T>(&self, method: Method, url: impl IntoUrl, body: Vec<u8>) -> Result<T, Error>
  where
    T: DeserializeOwned,
  {
    let response = self.send_inner(method, url, body).await?;

    match response.bytes().await {
      Ok(bytes) => serde_json::from_slice(&bytes).map_err(Error::Deserialization),

      Err(error) => Err(Error::ReadBody(error)),
    }
  }

  /// Fetches the projects the current credential covers.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_projects(&self) -> Result<Paginated<PartialProject>, Error> {
    self
      .send(Method::GET, api!(self.server, "/projects"), vec![])
      .await
  }

  /// Fetches the projects the current credential covers after a previously returned paginated list.
  ///
  /// # Errors
  ///
  /// If [`Paginated::is_last`] is `true`, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn get_projects_after(
    &self,
    reference: &Paginated<PartialProject>,
  ) -> Result<Paginated<PartialProject>, Error> {
    if reference.is_last() {
      return Err(Error::LastPage);
    }

    self
      .send(
        Method::GET,
        api!(self.server, "/projects?cursor={}", reference.cursor),
        vec![],
      )
      .await
  }

  /// Fetches a project associated with the current token.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_project(&self, id: impl Into<Snowflake>) -> Result<Project, Error> {
    self
      .send(
        Method::GET,
        api!(self.server, "/projects/{}", id.into()),
        vec![],
      )
      .await
  }

  /// Updates the headline and/or page content for a project. Both fields are locale-keyed, so you can set content for multiple languages in a single request.
  ///
  /// # Errors
  ///
  /// If the `headline` or `page_content` length does not match the required ranges, the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn edit_project(
    &self,
    id: impl Into<Snowflake>,
    headline: HashMap<Locale, String>,
    page_content: HashMap<Locale, String>,
  ) -> Result<(), Error> {
    for value in headline.values() {
      util::enforce_constraints(value.len(), 3..=140)?;
    }

    for value in page_content.values() {
      util::enforce_constraints(value.len(), 300..=50000)?;
    }

    let body = serde_json::to_vec(&EditProject {
      headline,
      page_content,
    })
    .map_err(Error::Serialization)?;

    self
      .send_inner(
        Method::PATCH,
        api!(self.server, "/projects/{}", id.into()),
        body,
      )
      .await?;

    Ok(())
  }

  /// Creates a new announcement for a project. Announcements appear on a project's page and can be used to notify users about updates, new features, or other news.
  ///
  /// # Errors
  ///
  /// If the `title` or `content` length does not match the required ranges, the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn post_announcement(
    &self,
    id: impl Into<Snowflake>,
    title: String,
    content: String,
    category: AnnouncementCategory,
  ) -> Result<(), Error> {
    util::enforce_constraints(title.len(), 3..=100)?;
    util::enforce_constraints(content.len(), 100..=2000)?;

    let body = serde_json::to_vec(&PostAnnouncement {
      title,
      content,
      category,
    })
    .map_err(Error::Serialization)?;

    self
      .send_inner(
        Method::POST,
        api!(self.server, "/projects/{}/announcements", id.into()),
        body,
      )
      .await?;

    Ok(())
  }

  /// Submits a single metrics payload for a project. Use this to push fresh numbers after an event such as joining or leaving a guild player connecting.
  ///
  /// # Errors
  ///
  /// If the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn post_metrics<M>(&self, id: impl Into<Snowflake>, metrics: M) -> Result<(), Error>
  where
    M: Serialize,
  {
    let body = serde_json::to_vec(&metrics).map_err(Error::Serialization)?;

    self
      .send_inner(
        Method::PATCH,
        api!(self.server, "/projects/{}/metrics", id.into()),
        body,
      )
      .await?;

    Ok(())
  }

  /// Submits up to 100 metrics entries in a single request.
  ///
  /// # Errors
  ///
  /// If the `metrics` length does not match the required ranges, the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn post_timestamped_metrics<M>(
    &self,
    id: impl Into<Snowflake>,
    metrics: TimestampedMetrics<M>,
  ) -> Result<(), Error>
  where
    M: Clone + Debug + Default + Serialize,
  {
    util::enforce_constraints(metrics.len(), 1..=100)?;

    let body = serde_json::to_vec(&PostTimestampedMetrics { data: metrics })
      .map_err(Error::Serialization)?;

    self
      .send_inner(
        Method::POST,
        api!(self.server, "/projects/{}/metrics/batch", id.into()),
        body,
      )
      .await?;

    Ok(())
  }

  /// Overwrites the list of slash command definitions for a bot project on Top.gg. This is only applicable to bot-type projects on the discord platform.
  ///
  /// # Errors
  ///
  /// If the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn post_commands<C>(&self, id: impl Into<Snowflake>, commands: C) -> Result<(), Error>
  where
    C: Serialize,
  {
    let body = serde_json::to_vec(&commands).map_err(Error::Serialization)?;

    self
      .send_inner(
        Method::PUT,
        api!(self.server, "/projects/{}/commands", id.into()),
        body,
      )
      .await?;

    Ok(())
  }

  /// Fetches a cursor-paginated list of votes for a project, ordered by creation date (oldest first within each page.)
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_votes(
    &self,
    id: impl Into<Snowflake>,
    since: DateTime<Utc>,
  ) -> Result<Paginated<Vote>, Error> {
    self
      .send(
        Method::GET,
        api!(
          self.server,
          "/projects/{}/votes?startDate={}",
          id.into(),
          urlencoding::encode(&since.to_rfc3339())
        ),
        vec![],
      )
      .await
  }

  /// Fetches a cursor-paginated list of votes for a project, ordered by creation date (oldest first within each page), after a previously returned paginated list.
  ///
  /// # Errors
  ///
  /// If [`Paginated::is_last`] is `true`, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn get_votes_after(
    &self,
    id: impl Into<Snowflake>,
    reference: &Paginated<Vote>,
  ) -> Result<Paginated<Vote>, Error> {
    if reference.is_last() {
      return Err(Error::LastPage);
    }

    self
      .send(
        Method::GET,
        api!(
          self.server,
          "/projects/{}/votes?cursor={}",
          id.into(),
          reference.cursor
        ),
        vec![],
      )
      .await
  }

  /// Fetches the most recent vote for a specific user.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_vote(
    &self,
    id: impl Into<Snowflake>,
    user_id: impl Into<Snowflake>,
    source: UserSource,
  ) -> Result<Option<PartialVote>, Error> {
    match self
      .send::<PartialVote>(
        Method::GET,
        api!(
          self.server,
          "/projects/{}/votes/{}?source={}",
          id.into(),
          user_id.into(),
          source.as_str()
        ),
        vec![],
      )
      .await
    {
      Ok(vote) => Ok(Some(vote)),

      Err(Error::Incoming(error)) => {
        if error.status == 404 && matches!(error.detail.as_deref(), Some("VOTE_EXPIRED")) {
          Ok(None)
        } else {
          Err(Error::Incoming(error))
        }
      }

      Err(error) => Err(error),
    }
  }

  /// Fetches the integrations available for a project and whether each one is connected.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_integrations(
    &self,
    id: impl Into<Snowflake>,
  ) -> Result<Vec<Integration>, Error> {
    self
      .send(
        Method::GET,
        api!(self.server, "/projects/{}/integrations", id.into()),
        vec![],
      )
      .await
  }

  /// Connects an integration to the project. Top.gg runs the integration handshake and starts delivering events to it.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn connect_integration(
    &self,
    project_id: impl Into<Snowflake>,
    integration_id: u64,
  ) -> Result<(), Error> {
    self
      .send_inner(
        Method::PUT,
        api!(
          self.server,
          "/projects/{}/integrations/{}",
          project_id.into(),
          integration_id
        ),
        vec![],
      )
      .await?;

    Ok(())
  }

  /// Disconnects an integration the application connected. The integration receives an integration.delete event.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn disconnect_integration(
    &self,
    project_id: impl Into<Snowflake>,
    integration_id: u64,
  ) -> Result<(), Error> {
    self
      .send_inner(
        Method::DELETE,
        api!(
          self.server,
          "/projects/{}/integrations/{}",
          project_id.into(),
          integration_id,
        ),
        vec![],
      )
      .await?;

    Ok(())
  }

  /// Fetches the webhooks the application created on the project.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_webhooks(&self, id: impl Into<Snowflake>) -> Result<Vec<Webhook>, Error> {
    self
      .send(
        Method::GET,
        api!(self.server, "/projects/{}/webhooks", id.into()),
        vec![],
      )
      .await
  }

  /// Creates a vote webhook on the project.
  ///
  /// # Errors
  ///
  /// If the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn create_webhook(
    &self,
    id: impl Into<Snowflake>,
    webhook: BaseWebhook,
  ) -> Result<Webhook, Error> {
    let body = serde_json::to_vec(&webhook).map_err(Error::Serialization)?;

    self
      .send(
        Method::POST,
        api!(self.server, "/projects/{}/webhooks", id.into()),
        body,
      )
      .await
  }

  /// Deletes a webhook the application created.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn delete_webhook(
    &self,
    project_id: impl Into<Snowflake>,
    webhook_id: u64,
  ) -> Result<(), Error> {
    self
      .send_inner(
        Method::DELETE,
        api!(
          self.server,
          "/projects/{}/webhooks/{}",
          project_id.into(),
          webhook_id
        ),
        vec![],
      )
      .await?;

    Ok(())
  }

  /// Replaces the signing secret of a webhook the application created. The previous secret stops working immediately.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn rotate_webhook_secret(
    &self,
    project_id: impl Into<Snowflake>,
    webhook_id: u64,
  ) -> Result<String, Error> {
    let response: RotateWebhookSecret = self
      .send(
        Method::POST,
        api!(
          self.server,
          "/projects/{}/webhooks/{}/rotate",
          project_id.into(),
          webhook_id
        ),
        vec![],
      )
      .await?;

    Ok(response.secret)
  }

  /// Sends a webhook.test event to a webhook the application created.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn test_webhook(
    &self,
    project_id: impl Into<Snowflake>,
    webhook_id: u64,
  ) -> Result<(), Error> {
    self
      .send_inner(
        Method::POST,
        api!(
          self.server,
          "/projects/{}/webhooks/{}/test",
          project_id.into(),
          webhook_id
        ),
        vec![],
      )
      .await?;

    Ok(())
  }
}

impl<P> Clone for BaseClient<P>
where
  P: Clone,
{
  fn clone(&self) -> Self {
    Self {
      provider: self.provider.clone(),
      #[cfg(test)]
      server: self.server.clone(),
    }
  }
}

impl<P> Deref for BaseClient<P> {
  type Target = P;

  fn deref(&self) -> &Self::Target {
    &self.provider
  }
}

/// A project or application API client.
pub struct Client {
  base: BaseClient<DirectHttpProvider>,
}

impl Client {
  /// Creates the client.
  #[must_use]
  pub fn new(token: String, #[cfg(test)] server: Arc<MockServer>) -> Self {
    Self {
      base: BaseClient {
        provider: DirectHttpProvider {
          token,
          http: util::http_client(),
        },
        #[cfg(test)]
        server,
      },
    }
  }

  /// Fetches a project associated with the current token.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_project(&self) -> Result<Project, Error> {
    self.base.get_project(Snowflake::Me).await
  }

  /// Updates the headline and/or page content for a project. Both fields are locale-keyed, so you can set content for multiple languages in a single request.
  ///
  /// # Errors
  ///
  /// If the `headline` or `page_content` length does not match the required ranges, the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn edit_project(
    &self,
    headline: HashMap<Locale, String>,
    page_content: HashMap<Locale, String>,
  ) -> Result<(), Error> {
    self
      .base
      .edit_project(Snowflake::Me, headline, page_content)
      .await
  }

  /// Creates a new announcement for a project. Announcements appear on a project's page and can be used to notify users about updates, new features, or other news.
  ///
  /// # Errors
  ///
  /// If the `title` or `content` length does not match the required ranges, the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn post_announcement(
    &self,
    title: String,
    content: String,
    category: AnnouncementCategory,
  ) -> Result<(), Error> {
    self
      .base
      .post_announcement(Snowflake::Me, title, content, category)
      .await
  }

  /// Submits a single metrics payload for a project. Use this to push fresh numbers after an event such as joining or leaving a guild player connecting.
  ///
  /// # Errors
  ///
  /// If the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn post_metrics<M>(&self, metrics: M) -> Result<(), Error>
  where
    M: Serialize,
  {
    self.base.post_metrics(Snowflake::Me, metrics).await
  }

  /// Submits up to 100 metrics entries in a single request.
  ///
  /// # Errors
  ///
  /// If the `metrics` length does not match the required ranges, the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn post_timestamped_metrics<M>(
    &self,
    metrics: TimestampedMetrics<M>,
  ) -> Result<(), Error>
  where
    M: Clone + Debug + Default + Serialize,
  {
    self
      .base
      .post_timestamped_metrics(Snowflake::Me, metrics)
      .await
  }

  /// Overwrites the list of slash command definitions for a bot project on Top.gg. This is only applicable to bot-type projects on the discord platform.
  ///
  /// # Errors
  ///
  /// If the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn post_commands<C>(&self, commands: C) -> Result<(), Error>
  where
    C: Serialize,
  {
    self.base.post_commands(Snowflake::Me, commands).await
  }

  /// Fetches a cursor-paginated list of votes for a project, ordered by creation date (oldest first within each page.)
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_votes(&self, since: DateTime<Utc>) -> Result<Paginated<Vote>, Error> {
    self.base.get_votes(Snowflake::Me, since).await
  }

  /// Fetches a cursor-paginated list of votes for a project, ordered by creation date (oldest first within each page), after a previously returned paginated list.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_votes_after(
    &self,
    reference: &Paginated<Vote>,
  ) -> Result<Paginated<Vote>, Error> {
    self.base.get_votes_after(Snowflake::Me, reference).await
  }

  /// Fetches the most recent vote for a specific user.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_vote(
    &self,
    user_id: impl Into<Snowflake>,
    source: UserSource,
  ) -> Result<Option<PartialVote>, Error> {
    self.base.get_vote(Snowflake::Me, user_id, source).await
  }
}

impl Deref for Client {
  type Target = BaseClient<DirectHttpProvider>;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}

#[cfg(feature = "oauth2")]
fn default_token_expires_at() -> DateTime<Utc> {
  Utc::now() + REFRESH_INTERVAL
}

/// An oauth2 session.
#[cfg(feature = "oauth2")]
#[derive(Debug, Deserialize)]
#[cfg_attr(doc_cfg, doc(cfg(feature = "oauth2")))]
pub struct Oauth2Session {
  /// The session's access token.
  #[serde(rename = "access_token")]
  pub token: String,

  /// The session's refresh token.
  #[serde(rename = "refresh_token")]
  pub refresh_token: String,

  /// When the session's access token expires.
  #[serde(skip_deserializing, default = "default_token_expires_at")]
  pub token_expires_at: DateTime<Utc>,
}

#[cfg(feature = "oauth2")]
impl Oauth2Session {
  /// Whether the session's access token has expired.
  #[must_use]
  pub fn has_expired(&self) -> bool {
    Utc::now() >= self.token_expires_at
  }
}

#[cfg(feature = "oauth2")]
struct Oauth2SessionWrapper {
  session: RwLock<Oauth2Session>,
  cancel: CancellationToken,
}

#[cfg(feature = "oauth2")]
impl Oauth2SessionWrapper {
  async fn delay(&self, duration: Duration) -> bool {
    let Ok(duration) = duration.to_std() else {
      return true;
    };

    tokio::select! {
      () = self.cancel.cancelled() => false,

      () = sleep(duration) => true,
    }
  }
}

#[cfg(feature = "oauth2")]
impl Drop for Oauth2SessionWrapper {
  fn drop(&mut self) {
    self.cancel.cancel();
  }
}

#[cfg(feature = "oauth2")]
struct BaseOauth2URL {
  state: String,
  code_verifier: String,
  url: String,
}

/// A thread-safe oauth2 URL.
#[derive(Clone)]
#[cfg(feature = "oauth2")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "oauth2")))]
pub struct Oauth2URL {
  oauth2: Arc<BaseOauth2>,
  url: Arc<BaseOauth2URL>,
}

#[cfg(feature = "oauth2")]
impl Oauth2URL {
  /// Retrieves the URL's state.
  #[must_use]
  pub fn get_state(&self) -> &str {
    &self.url.state
  }

  /// Retrieves the URL's code verifier.
  #[must_use]
  pub fn get_code_verifier(&self) -> &str {
    &self.url.code_verifier
  }

  /// Retrieves the full URL.
  #[must_use]
  pub fn get_url(&self) -> &str {
    &self.url.url
  }

  /// Exchanges an authorization code for an oauth2 session.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn exchange(&self, code: &str) -> Result<Oauth2Session, Error> {
    let body = format!("grant_type=authorization_code&client_id={}&client_secret={}&code={}&redirect_uri={}&code_verifier={}", self.oauth2.client_id, self.oauth2.client_secret, urlencoding::encode(code), urlencoding::encode(&self.oauth2.redirect_uri), urlencoding::encode(&self.url.code_verifier)).into_bytes();

    let response = util::request(
      &self.oauth2.http,
      Method::POST,
      api!(self.oauth2.server, "/oauth2/token"),
      None,
      "application/x-www-form-urlencoded",
      body,
    )
    .await?;

    match response.bytes().await {
      Ok(bytes) => serde_json::from_slice(&bytes).map_err(Error::Deserialization),

      Err(error) => Err(Error::ReadBody(error)),
    }
  }
}

#[cfg(feature = "oauth2")]
struct BaseOauth2 {
  client_id: u64,
  client_secret: String,
  redirect_uri: String,
  scope: String,
  http: HttpClient,
  #[cfg(test)]
  server: Arc<MockServer>,
}

/// A thread-safe oauth2 manager.
#[derive(Clone)]
#[cfg(feature = "oauth2")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "oauth2")))]
pub struct Oauth2 {
  base: Arc<BaseOauth2>,
}

#[cfg(feature = "oauth2")]
impl Oauth2 {
  /// Creates the manager.
  #[must_use]
  pub fn new(
    client_id: u64,
    client_secret: String,
    redirect_uri: String,
    scopes: &[&str],
    #[cfg(test)] server: Arc<MockServer>,
  ) -> Self {
    Self {
      base: Arc::new(BaseOauth2 {
        client_id,
        client_secret,
        redirect_uri,
        scope: scopes.join("+"),
        http: util::http_client(),
        #[cfg(test)]
        server,
      }),
    }
  }

  /// Creates an oauth2 URL from an existing state and code verifier.
  #[must_use]
  pub fn new_url_from(&self, state: String, code_verifier: String) -> Oauth2URL {
    let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));

    let url = format!(
      "https://top.gg/oauth2/authorize?response_type=code&code_challenge_method=S256&client_id={}&redirect_uri={}&state={}&code_challenge={}&scope={}",
      self.base.client_id,
      urlencoding::encode(&self.base.redirect_uri),
      urlencoding::encode(&state),
      urlencoding::encode(&code_challenge),
      self.base.scope
    );

    Oauth2URL {
      oauth2: self.base.clone(),
      url: Arc::new(BaseOauth2URL {
        state,
        code_verifier,
        url,
      }),
    }
  }

  /// Creates an oauth2 URL.
  #[must_use]
  pub fn new_url(&self) -> Oauth2URL {
    self.new_url_from(util::secrets64(), util::secrets64())
  }

  /// Creates an oauth2 API client from an oauth2 session.
  #[must_use]
  pub fn new_client(
    &self,
    session: Oauth2Session,
    #[cfg(test)] server: Arc<MockServer>,
  ) -> Oauth2Client {
    let base = BaseClient {
      provider: Oauth2HttpProvider {
        oauth2: Arc::clone(&self.base),
        session: Arc::new(Oauth2SessionWrapper {
          session: RwLock::new(session),
          cancel: CancellationToken::default(),
        }),
        #[cfg(test)]
        server: server.clone(),
      },
      #[cfg(test)]
      server,
    };
    let provider = base.provider.clone();

    Oauth2Client {
      base,
      _thread: Arc::new(task::spawn(async move {
        let now = Utc::now();
        let starting_delay = {
          let session3 = provider.session.session.read().await;

          session3.token_expires_at - now - Duration::seconds(5)
        };

        let delay = REFRESH_INTERVAL - Duration::seconds(5);

        if provider.session.delay(starting_delay).await {
          loop {
            let _ = provider.refresh_token().await;

            if !provider.session.delay(delay).await {
              break;
            }
          }
        }
      })),
    }
  }
}

/// A thread-safe oauth2 API client.
#[derive(Clone)]
#[cfg(feature = "oauth2")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "oauth2")))]
pub struct Oauth2Client {
  base: BaseClient<Oauth2HttpProvider>,
  _thread: Arc<JoinHandle<()>>,
}

#[cfg(feature = "oauth2")]
impl Oauth2Client {
  /// Retrieves the client's oauth2 session enclosed within a guard.
  pub async fn get_session(&self) -> RwLockReadGuard<'_, Oauth2Session> {
    self.base.provider.session.session.read().await
  }

  /// Whether the client's session access token has expired.
  pub async fn has_expired(&self) -> bool {
    self.get_session().await.has_expired()
  }

  /// Fetches the user who authorized the application.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_authorized_user(&self) -> Result<User, Error> {
    self
      .base
      .send(Method::GET, api!(self.base.server, "/users/@me"), vec![])
      .await
  }

  /// Fetches the projects owned by the user who authorized the application.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_authorized_user_projects(&self) -> Result<Vec<UserProject>, Error> {
    self
      .base
      .send(
        Method::GET,
        api!(self.base.server, "/users/@me/projects"),
        vec![],
      )
      .await
  }

  /// Creates a draft project for the user who authorized the application. Name, icon, and missing descriptions are fetched from the platform. The project starts as a draft; the user completes the listing and submits it for review in their Top.gg dashboard.
  ///
  /// # Errors
  ///
  /// If the `headline` or `page_content` length does not match the required ranges, the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn submit_project(&self, submission: ProjectSubmission) -> Result<(), Error> {
    util::enforce_constraints(submission.headline.len(), 3..=140)?;
    util::enforce_constraints(submission.page_content.len(), 300..=50000)?;

    let body = serde_json::to_vec(&submission).map_err(Error::Serialization)?;

    self
      .base
      .send_inner(
        Method::POST,
        api!(self.base.server, "/users/@me/projects"),
        body,
      )
      .await?;

    Ok(())
  }

  /// Revokes the client's access token.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn revoke_token(&self) -> Result<(), Error> {
    let session = self.base.provider.session.session.read().await;
    let body = format!(
      "client_id={}&client_secret={}&token={}",
      self.oauth2.client_id,
      self.oauth2.client_secret,
      urlencoding::encode(&session.refresh_token)
    )
    .into_bytes();

    drop(session);

    util::request(
      &self.oauth2.http,
      Method::POST,
      api!(self.base.server, "/oauth2/revoke"),
      None,
      "application/x-www-form-urlencoded",
      body,
    )
    .await?;

    self.base.session.cancel.cancel();

    Ok(())
  }
}

#[cfg(feature = "oauth2")]
impl Deref for Oauth2Client {
  type Target = BaseClient<Oauth2HttpProvider>;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}
