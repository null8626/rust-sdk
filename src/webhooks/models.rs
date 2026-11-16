// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

use super::super::{Platform, util};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

/// A project.
#[derive(Clone, Debug, Deserialize)]
pub struct Project {
  /// The project's Top.gg ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub id: u64,

  /// The project's Discord client ID or guild ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub platform_id: u64,

  /// The project's platform (always [`Platform::Discord`]).
  pub platform: Platform,
}

/// A user.
#[derive(Clone, Debug, Deserialize)]
pub struct User {
  /// The user's Top.gg ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub id: u64,

  /// The user's Discord ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub platform_id: u64,

  /// The user's Discord username.
  pub name: String,

  /// The user's avatar URL.
  #[serde(rename = "avatar_url")]
  pub avatar: String,
}

/// A vote.create webhook payload. Top.gg fires this event each time a user upvotes your project. Use it to reward voters, update leaderboards, or log activity.
#[derive(Clone, Debug, Deserialize)]
pub struct VoteCreate {
  /// The vote's Top.gg ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub id: u64,

  /// The vote's weight (1 normally, 2 during weekend multiplier.)
  pub weight: u8,

  /// When the vote was cast.
  pub created_at: DateTime<Utc>,

  /// When the user can vote again.
  pub expires_at: DateTime<Utc>,

  /// The voted project.
  pub project: Project,

  /// The voter.
  pub user: User,

  /// The parsed query parameters appended to the /:id/vote page URL.
  #[serde(default)]
  pub query: HashMap<String, String>,
}

/// A webhook.test webhook payload. This event is always available and lets you verify your endpoint is reachable before real votes arrive. Send a test from your project's Webhooks page in the dashboard.
#[derive(Clone, Debug, Deserialize)]
pub struct Test {
  /// The voted project.
  pub project: Project,

  /// The voter.
  pub user: User,
}

/// A integration.create webhook payload that fires when a user visits your integration page on Top.gg and clicks the Connect button.
#[derive(Clone, Debug, Deserialize)]
pub struct IntegrationCreate {
  /// The connection's Top.gg ID. Store this to track and delete.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub connection_id: u64,

  /// The webhook secret (prefixed whs_) used to verify all future webhook.
  pub webhook_secret: String,

  /// The connection's project.
  pub project: Project,

  /// The connection's user.
  pub user: User,
}

/// A integration.delete webhook payload that fires when a user removes the managed webhook from their dashboard.
#[derive(Clone, Debug, Deserialize)]
pub struct IntegrationDelete {
  /// The connection's Top.gg ID.
  #[serde(deserialize_with = "util::deserialize_snowflake")]
  pub connection_id: u64,
}

/// A webhook payload.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Payload {
  /// A vote.create webhook payload. Top.gg fires this event each time a user upvotes your project. Use it to reward voters, update leaderboards, or log activity.
  #[serde(rename = "vote.create")]
  VoteCreate(VoteCreate),

  /// A webhook.test webhook payload. This event is always available and lets you verify your endpoint is reachable before real votes arrive. Send a test from your project's Webhooks page in the dashboard.
  #[serde(rename = "webhook.test")]
  Test(Test),

  /// A integration.create webhook payload that fires when a user visits your integration page on Top.gg and clicks the Connect button.
  #[serde(rename = "integration.create")]
  IntegrationCreate(IntegrationCreate),

  /// A integration.delete webhook payload that fires when a user removes the managed webhook from their dashboard.
  #[serde(rename = "integration.delete")]
  IntegrationDelete(IntegrationDelete),
}
