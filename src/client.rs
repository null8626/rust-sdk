// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

use super::{
  AnnouncementCategory, BaseWebhook, Error, Integration, Locale, Paginated, PartialProject,
  PartialVote, Project, TimestampedMetrics, UserSource, Vote, Webhook, util,
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
#[cfg(feature = "oauth2")]
use serde::Deserializer;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[cfg(feature = "oauth2")]
use sha2::{Digest, Sha256};
#[cfg(any(test, feature = "oauth2"))]
use std::sync::Arc;
use std::{collections::HashMap, fmt::Debug, ops::Deref};
#[cfg(feature = "oauth2")]
use tokio::sync::{RwLock, RwLockReadGuard};

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
  /// Ensures that the current session is ready for an HTTP request.
  async fn ensure_session(&self) -> Result<(), Error> {
    Ok(())
  }

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
  session: Arc<RwLock<Oauth2Session>>,
  #[cfg(test)]
  server: Arc<MockServer>,
}

#[cfg(feature = "oauth2")]
#[async_trait::async_trait]
impl HttpProvider for Oauth2HttpProvider {
  #[allow(clippy::significant_drop_tightening)]
  async fn ensure_session(&self) -> Result<(), Error> {
    let mut session = self.session.write().await;

    if !session.has_expired() {
      return Ok(());
    }

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

  async fn get_token(&self) -> TokenGuard<'_> {
    TokenGuard::Guard(self.session.read().await)
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

struct BaseClient<P> {
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

    self.provider.ensure_session().await?;

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
}

impl<P> Clone for BaseClient<P>
where
  P: HttpProvider + Clone,
{
  fn clone(&self) -> Self {
    Self {
      provider: self.provider.clone(),
      #[cfg(test)]
      server: self.server.clone(),
    }
  }
}

/// An API client using a project token.
pub struct ProjectClient {
  base: BaseClient<DirectHttpProvider>,
}

impl ProjectClient {
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
    self
      .base
      .send(Method::GET, api!(self.base.server, "/projects/@me"), vec![])
      .await
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
      .base
      .send_inner(Method::PATCH, api!(self.base.server, "/projects/@me"), body)
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
      .base
      .send_inner(
        Method::POST,
        api!(self.base.server, "/projects/@me/announcements"),
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
  pub async fn post_metrics<M>(&self, metrics: M) -> Result<(), Error>
  where
    M: Serialize,
  {
    let body = serde_json::to_vec(&metrics).map_err(Error::Serialization)?;

    self
      .base
      .send_inner(
        Method::PATCH,
        api!(self.base.server, "/projects/@me/metrics"),
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
    metrics: TimestampedMetrics<M>,
  ) -> Result<(), Error>
  where
    M: Clone + Debug + Default + Serialize,
  {
    util::enforce_constraints(metrics.len(), 1..=100)?;

    let body = serde_json::to_vec(&PostTimestampedMetrics { data: metrics })
      .map_err(Error::Serialization)?;

    self
      .base
      .send_inner(
        Method::POST,
        api!(self.base.server, "/projects/@me/metrics/batch"),
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
  pub async fn post_commands<C>(&self, commands: C) -> Result<(), Error>
  where
    C: Serialize,
  {
    let body = serde_json::to_vec(&commands).map_err(Error::Serialization)?;

    self
      .base
      .send_inner(
        Method::PUT,
        api!(self.base.server, "/projects/@me/commands"),
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
  pub async fn get_votes(&self, since: DateTime<Utc>) -> Result<Paginated<Vote>, Error> {
    self
      .base
      .send(
        Method::GET,
        api!(
          self.base.server,
          "/projects/@me/votes?startDate={}",
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
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  #[allow(clippy::missing_panics_doc)]
  pub async fn get_votes_after(
    &self,
    reference: &Paginated<Vote>,
  ) -> Result<Paginated<Vote>, Error> {
    if reference.is_last() {
      return Err(Error::LastPage);
    }

    self
      .base
      .send(
        Method::GET,
        api!(
          self.base.server,
          "/projects/@me/votes?cursor={}",
          reference.cursor.as_deref().unwrap()
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
    user_id: u64,
    source: UserSource,
  ) -> Result<Option<PartialVote>, Error> {
    match self
      .base
      .send::<PartialVote>(
        Method::GET,
        api!(
          self.base.server,
          "/projects/@me/votes/{}?source={}",
          user_id,
          source.as_str()
        ),
        vec![],
      )
      .await
    {
      Ok(vote) => Ok(Some(vote)),

      Err(error) => {
        if let Error::Incoming(error) = &error
          && error.status == 404
          && matches!(
            error.detail.as_deref(),
            Some("VOTE_NOT_FOUND" | "VOTE_EXPIRED")
          )
        {
          Ok(None)
        } else {
          Err(error)
        }
      }
    }
  }
}

/// A base client for application and oauth2 access tokens.
#[derive(Clone)]
pub struct BaseApplicationClient<P: HttpProvider> {
  base: BaseClient<P>,
}

impl<P> BaseApplicationClient<P>
where
  P: HttpProvider,
{
  const fn new(provider: P, #[cfg(test)] server: Arc<MockServer>) -> Self {
    Self {
      base: BaseClient {
        provider,
        #[cfg(test)]
        server,
      },
    }
  }

  /// Fetches the projects the current credential covers.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_projects(&self) -> Result<Paginated<PartialProject>, Error> {
    self
      .base
      .send(Method::GET, api!(self.base.server, "/projects"), vec![])
      .await
  }

  /// Fetches the projects the current credential covers after a previously returned paginated list.
  ///
  /// # Errors
  ///
  /// If [`Paginated::is_last`] is `true`, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  #[allow(clippy::missing_panics_doc)]
  pub async fn get_projects_after(
    &self,
    reference: &Paginated<PartialProject>,
  ) -> Result<Paginated<PartialProject>, Error> {
    if reference.is_last() {
      return Err(Error::LastPage);
    }

    self
      .base
      .send(
        Method::GET,
        api!(
          self.base.server,
          "/projects?cursor={}",
          reference.cursor.as_deref().unwrap()
        ),
        vec![],
      )
      .await
  }

  /// Fetches a project associated with the current token.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_project(&self, id: u64) -> Result<Project, Error> {
    self
      .base
      .send(
        Method::GET,
        api!(self.base.server, "/projects/{}", id),
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
    id: u64,
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
      .base
      .send_inner(
        Method::PATCH,
        api!(self.base.server, "/projects/{}", id),
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
    id: u64,
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
      .base
      .send_inner(
        Method::POST,
        api!(self.base.server, "/projects/{}/announcements", id),
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
  pub async fn post_metrics<M>(&self, id: u64, metrics: M) -> Result<(), Error>
  where
    M: Serialize,
  {
    let body = serde_json::to_vec(&metrics).map_err(Error::Serialization)?;

    self
      .base
      .send_inner(
        Method::PATCH,
        api!(self.base.server, "/projects/{}/metrics", id),
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
    id: u64,
    metrics: TimestampedMetrics<M>,
  ) -> Result<(), Error>
  where
    M: Clone + Debug + Default + Serialize,
  {
    util::enforce_constraints(metrics.len(), 1..=100)?;

    let body = serde_json::to_vec(&PostTimestampedMetrics { data: metrics })
      .map_err(Error::Serialization)?;

    self
      .base
      .send_inner(
        Method::POST,
        api!(self.base.server, "/projects/{}/metrics/batch", id),
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
  pub async fn post_commands<C>(&self, id: u64, commands: C) -> Result<(), Error>
  where
    C: Serialize,
  {
    let body = serde_json::to_vec(&commands).map_err(Error::Serialization)?;

    self
      .base
      .send_inner(
        Method::PUT,
        api!(self.base.server, "/projects/{}/commands", id),
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
  pub async fn get_votes(&self, id: u64, since: DateTime<Utc>) -> Result<Paginated<Vote>, Error> {
    self
      .base
      .send(
        Method::GET,
        api!(
          self.base.server,
          "/projects/{}/votes?startDate={}",
          id,
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
  #[allow(clippy::missing_panics_doc)]
  pub async fn get_votes_after(
    &self,
    id: u64,
    reference: &Paginated<Vote>,
  ) -> Result<Paginated<Vote>, Error> {
    if reference.is_last() {
      return Err(Error::LastPage);
    }

    self
      .base
      .send(
        Method::GET,
        api!(
          self.base.server,
          "/projects/{}/votes?cursor={}",
          id,
          reference.cursor.as_deref().unwrap()
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
    id: u64,
    user_id: u64,
    source: UserSource,
  ) -> Result<Option<PartialVote>, Error> {
    match self
      .base
      .send::<PartialVote>(
        Method::GET,
        api!(
          self.base.server,
          "/projects/{}/votes/{}?source={}",
          id,
          user_id,
          source.as_str()
        ),
        vec![],
      )
      .await
    {
      Ok(vote) => Ok(Some(vote)),

      Err(error) => {
        if let Error::Incoming(error) = &error
          && error.status == 404
          && matches!(
            error.detail.as_deref(),
            Some("VOTE_NOT_FOUND" | "VOTE_EXPIRED")
          )
        {
          Ok(None)
        } else {
          Err(error)
        }
      }
    }
  }

  /// Fetches the integrations available for a project and whether each one is connected.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_integrations(&self, id: u64) -> Result<Vec<Integration>, Error> {
    self
      .base
      .send(
        Method::GET,
        api!(self.base.server, "/projects/{}/integrations", id),
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
    project_id: u64,
    integration_id: u64,
  ) -> Result<(), Error> {
    self
      .base
      .send_inner(
        Method::PUT,
        api!(
          self.base.server,
          "/projects/{}/integrations/{}",
          project_id,
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
    project_id: u64,
    integration_id: u64,
  ) -> Result<(), Error> {
    self
      .base
      .send_inner(
        Method::DELETE,
        api!(
          self.base.server,
          "/projects/{}/integrations/{}",
          project_id,
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
  pub async fn get_webhooks(&self, id: u64) -> Result<Vec<Webhook>, Error> {
    self
      .base
      .send(
        Method::GET,
        api!(self.base.server, "/projects/{}/webhooks", id),
        vec![],
      )
      .await
  }

  /// Creates a vote webhook on the project.
  ///
  /// # Errors
  ///
  /// If the client is unable to serialize the request's JSON body, the client received an unfavorable response from the API, or the client is ratelimited from sending more requests.
  pub async fn create_webhook(&self, id: u64, webhook: BaseWebhook) -> Result<Webhook, Error> {
    let body = serde_json::to_vec(&webhook).map_err(Error::Serialization)?;

    self
      .base
      .send(
        Method::POST,
        api!(self.base.server, "/projects/{}/webhooks", id),
        body,
      )
      .await
  }

  /// Deletes a webhook the application created.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn delete_webhook(&self, project_id: u64, webhook_id: u64) -> Result<(), Error> {
    self
      .base
      .send_inner(
        Method::DELETE,
        api!(
          self.base.server,
          "/projects/{}/webhooks/{}",
          project_id,
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
    project_id: u64,
    webhook_id: u64,
  ) -> Result<String, Error> {
    let response: RotateWebhookSecret = self
      .base
      .send(
        Method::POST,
        api!(
          self.base.server,
          "/projects/{}/webhooks/{}/rotate",
          project_id,
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
  pub async fn test_webhook(&self, project_id: u64, webhook_id: u64) -> Result<(), Error> {
    self
      .base
      .send_inner(
        Method::POST,
        api!(
          self.base.server,
          "/projects/{}/webhooks/{}/test",
          project_id,
          webhook_id
        ),
        vec![],
      )
      .await?;

    Ok(())
  }
}

/// An API client using an application token.
pub struct ApplicationClient {
  base: BaseApplicationClient<DirectHttpProvider>,
}

impl ApplicationClient {
  /// Creates the client.
  #[must_use]
  pub fn new(token: String, #[cfg(test)] server: Arc<MockServer>) -> Self {
    Self {
      base: BaseApplicationClient::new(
        DirectHttpProvider {
          token,
          http: util::http_client(),
        },
        #[cfg(test)]
        server,
      ),
    }
  }
}

impl Deref for ApplicationClient {
  type Target = BaseApplicationClient<DirectHttpProvider>;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}

#[cfg(feature = "oauth2")]
fn deserialize_expires_in<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
  D: Deserializer<'de>,
{
  i64::deserialize(deserializer).map(|seconds| Utc::now() + Duration::seconds(seconds))
}

/// An oauth2 session.
#[cfg(feature = "oauth2")]
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(doc_cfg, doc(cfg(feature = "oauth2")))]
pub struct Oauth2Session {
  /// The session's access token.
  #[serde(rename = "access_token")]
  pub token: String,

  /// The session's refresh token.
  pub refresh_token: String,

  /// When the session's access token expires.
  #[serde(rename = "expires_in", deserialize_with = "deserialize_expires_in")]
  pub expires_at: DateTime<Utc>,
}

#[cfg(feature = "oauth2")]
impl Oauth2Session {
  /// Whether the session's access token has expired.
  #[must_use]
  pub fn has_expired(&self) -> bool {
    Utc::now() >= self.expires_at
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
    Oauth2Client {
      base: BaseApplicationClient::new(
        Oauth2HttpProvider {
          oauth2: Arc::clone(&self.base),
          session: Arc::new(RwLock::new(session)),
          #[cfg(test)]
          server: server.clone(),
        },
        #[cfg(test)]
        server,
      ),
    }
  }
}

/// A thread-safe oauth2 API client.
#[derive(Clone)]
#[cfg(feature = "oauth2")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "oauth2")))]
pub struct Oauth2Client {
  base: BaseApplicationClient<Oauth2HttpProvider>,
}

#[cfg(feature = "oauth2")]
impl Oauth2Client {
  /// Retrieves the client's oauth2 session enclosed within a guard.
  pub async fn get_session(&self) -> RwLockReadGuard<'_, Oauth2Session> {
    self.base.base.provider.session.read().await
  }

  /// Fetches the user who authorized the application.
  ///
  /// # Errors
  ///
  /// If the client received an unfavorable response from the API or is ratelimited from sending more requests.
  pub async fn get_authorized_user(&self) -> Result<User, Error> {
    self
      .base
      .base
      .send(
        Method::GET,
        api!(self.base.base.server, "/users/@me"),
        vec![],
      )
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
      .base
      .send(
        Method::GET,
        api!(self.base.base.server, "/users/@me/projects"),
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
      .base
      .send_inner(
        Method::POST,
        api!(self.base.base.server, "/users/@me/projects"),
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
    let session = self.get_session().await;
    let body = format!(
      "client_id={}&client_secret={}&token={}",
      self.base.base.provider.oauth2.client_id,
      self.base.base.provider.oauth2.client_secret,
      urlencoding::encode(&session.refresh_token)
    )
    .into_bytes();

    drop(session);

    util::request(
      &self.base.base.provider.oauth2.http,
      Method::POST,
      api!(self.base.base.server, "/oauth2/revoke"),
      None,
      "application/x-www-form-urlencoded",
      body,
    )
    .await?;

    Ok(())
  }
}

#[cfg(feature = "oauth2")]
impl Deref for Oauth2Client {
  type Target = BaseApplicationClient<Oauth2HttpProvider>;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}
