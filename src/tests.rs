// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

#[cfg(feature = "webhooks")]
use super::webhooks;
#[cfg(feature = "api")]
use super::{
  AnnouncementCategory, ApplicationClient, BaseWebhook, DiscordBotMetrics, EditProject, Error,
  Locale, PostAnnouncement, PostTimestampedMetrics, ProjectClient, TimestampedMetrics, UserSource,
};
#[cfg(feature = "oauth2")]
use super::{Oauth2, Platform, ProjectSubmission, ProjectType};
#[cfg(feature = "api")]
use chrono::Duration;
use chrono::Utc;
#[cfg(feature = "webhooks")]
use hmac::{Hmac, KeyInit, Mac};
#[cfg(feature = "api")]
use httpmock::{Method, MockServer};
#[cfg(feature = "webhooks")]
use sha2::Sha256;
#[cfg(feature = "oauth2")]
use std::time::Duration as StdDuration;
#[cfg(feature = "api")]
use std::{collections::HashMap, sync::Arc};
#[cfg(feature = "oauth2")]
use tokio::time::sleep;

#[cfg(feature = "api")]
macro_rules! mock_server_project_client {
  () => {{
    let server = Arc::new(MockServer::start());
    let client = ProjectClient::new(String::from("token"), server.clone());

    (server, client)
  }};
}

#[cfg(feature = "api")]
macro_rules! mock_server_application_client {
  () => {{
    let server = Arc::new(MockServer::start());
    let client = ApplicationClient::new(String::from("token"), server.clone());

    (server, client)
  }};
}

#[cfg(feature = "api")]
macro_rules! mock_get {
  ($server:ident, $endpoint_path:literal, $response_path:literal) => {
    $server.mock(|when, then| {
      when
        .method(Method::GET)
        .path($endpoint_path)
        .header("content-type", "application/json");

      then
        .status(200)
        .header("content-type", "application/json")
        .body(include_str!(concat!("mocks/", $response_path)));
    })
  };
}

#[cfg(feature = "api")]
macro_rules! mock_endpoint {
  ($server:ident, $method:expr, $endpoint_path:literal) => {
    $server.mock(|when, then| {
      when
        .method($method)
        .path($endpoint_path)
        .header("content-type", "application/json");

      then.status(204);
    })
  };

  ($server:ident, $method:expr, $endpoint_path:literal, $expected_json_body:ident) => {
    $server.mock(|when, then| {
      when
        .method($method)
        .path($endpoint_path)
        .json_body($expected_json_body)
        .header("content-type", "application/json");

      then.status(204);
    })
  };
}

#[cfg(feature = "oauth2")]
macro_rules! mock_oauth2_server_client {
  () => {{
    let server = Arc::new(MockServer::start());
    let oauth2 = Oauth2::new(
      12345,
      String::from("secret"),
      String::from("http://localhost:3000/topgg/oauth2"),
      &[
        "user.identify",
        "user.projects.read",
        "user.projects.write",
        "project.information.read",
        "project.information.write",
        "project.votes.read",
        "project.metrics.write",
        "project.announcements.write",
        "project.webhooks.read",
        "project.webhooks.write",
        "project.integrations.read",
        "project.integrations.write",
      ],
      server.clone(),
    );
    let url = oauth2.new_url();

    (server, oauth2, url)
  }};
}

#[cfg(feature = "oauth2")]
macro_rules! mock_oauth2_refresh_token {
  ($server:ident) => {
    $server.mock(|when, then| {
      when
        .method(Method::POST)
        .path("/oauth2/token")
        .header("content-type", "application/x-www-form-urlencoded");

      then
        .status(200)
        .header("content-type", "application/json")
        .body(include_str!("mocks/post_oauth2_token.json"));
    })
  };
}

#[cfg(feature = "webhooks")]
macro_rules! webhook_payload_works {
  ($response_path:literal, $matching_pattern:pat) => {{
    let payload = serde_json::from_str(include_str!(concat!("mocks/", $response_path))).unwrap();

    assert!(matches!(payload, $matching_pattern));
  }};
}

#[tokio::test]
#[cfg(feature = "api")]
async fn get_projects() {
  let (server, client) = mock_server_application_client!();
  let endpoint = mock_get!(server, "/projects", "get_projects.json");

  let projects = client.get_projects().await.unwrap();

  endpoint.assert();

  assert!(projects.is_last());
  assert!(matches!(
    client.get_projects_after(&projects).await,
    Err(Error::LastPage)
  ));
}

#[tokio::test]
#[cfg(feature = "api")]
async fn get_projects_after() {
  let (server, client) = mock_server_application_client!();
  let endpoint = mock_get!(server, "/projects", "get_projects_cursor.json");

  let projects = client.get_projects().await.unwrap();

  assert!(!projects.is_last());

  let _ = client.get_projects_after(&projects).await.unwrap();

  endpoint.assert_calls(2);
}

#[tokio::test]
#[cfg(feature = "api")]
async fn get_project() {
  let (server, client) = mock_server_project_client!();
  let endpoint = mock_get!(server, "/projects/@me", "get_projects_id.json");

  let _ = client.get_project().await.unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn edit_project() {
  let (server, client) = mock_server_project_client!();

  let headline = HashMap::from_iter([
    (Locale::English, String::from("hello this is a headline")),
    (
      Locale::German,
      String::from("hallo das ist eine Überschrift"),
    ),
  ]);

  let page_content = HashMap::from_iter([
    (Locale::English, "hello this is a page content".repeat(25)),
    (Locale::German, "hallo dies ist ein seiteninhalt".repeat(25)),
  ]);

  let expected_json_body = serde_json::to_value(EditProject {
    headline: headline.clone(),
    page_content: page_content.clone(),
  })
  .unwrap();
  let endpoint = mock_endpoint!(server, Method::PATCH, "/projects/@me", expected_json_body);

  client
    .edit_project(headline.clone(), page_content)
    .await
    .unwrap();

  endpoint.assert();

  let faulty_page_content = HashMap::from_iter([
    (
      Locale::English,
      String::from("hello this is a page content"),
    ),
    (
      Locale::German,
      String::from("hallo dies ist ein seiteninhalt"),
    ),
  ]);

  assert!(matches!(
    client.edit_project(headline, faulty_page_content).await,
    Err(Error::LengthConstraints { .. })
  ));
}

#[tokio::test]
#[cfg(feature = "api")]
async fn post_announcement() {
  let (server, client) = mock_server_project_client!();

  let title = String::from("My project announcement");
  let content = "Lorem ipsum".repeat(25);
  let category = AnnouncementCategory::Announcement;

  let expected_json_body = serde_json::to_value(PostAnnouncement {
    title: title.clone(),
    content: content.clone(),
    category,
  })
  .unwrap();
  let endpoint = mock_endpoint!(
    server,
    Method::POST,
    "/projects/@me/announcements",
    expected_json_body
  );

  client
    .post_announcement(title.clone(), content, category)
    .await
    .unwrap();

  endpoint.assert();

  let faulty_content = String::from("Lorem ipsum");

  assert!(matches!(
    client
      .post_announcement(title, faulty_content, category)
      .await,
    Err(Error::LengthConstraints { .. })
  ));
}

#[tokio::test]
#[cfg(feature = "api")]
async fn post_commands() {
  let (server, client) = mock_server_project_client!();

  let commands = serde_json::json!([
    {
      "id": "1",
      "type": 1,
      "application_id": "1",
      "name": "test",
      "description": "command description",
      "default_member_permissions": "",
      "version": "1"
    }
  ]);
  let commands2 = commands.clone();
  let endpoint = mock_endpoint!(server, Method::PUT, "/projects/@me/commands", commands2);

  client.post_commands(commands).await.unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn post_metrics() {
  let (server, client) = mock_server_project_client!();

  let metrics = DiscordBotMetrics {
    server_count: Some(5),
    ..Default::default()
  };
  let expected_json_body = serde_json::to_value(metrics).unwrap();
  let endpoint = mock_endpoint!(
    server,
    Method::PATCH,
    "/projects/@me/metrics",
    expected_json_body
  );

  client.post_metrics(metrics).await.unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn post_timestamped_metrics() {
  let (server, client) = mock_server_project_client!();

  let metrics = DiscordBotMetrics {
    server_count: Some(5),
    ..Default::default()
  };
  let mut timestamped_metrics = TimestampedMetrics::default();

  timestamped_metrics
    .add(Utc::now() - Duration::hours(5), metrics)
    .unwrap();
  timestamped_metrics
    .add(Utc::now() - Duration::hours(4), metrics)
    .unwrap();
  timestamped_metrics
    .add(Utc::now() - Duration::hours(3), metrics)
    .unwrap();
  timestamped_metrics
    .add(Utc::now() - Duration::hours(2), metrics)
    .unwrap();
  timestamped_metrics
    .add(Utc::now() - Duration::hours(1), metrics)
    .unwrap();

  let expected_json_body = serde_json::to_value(PostTimestampedMetrics {
    data: timestamped_metrics.clone(),
  })
  .unwrap();
  let endpoint = mock_endpoint!(
    server,
    Method::POST,
    "/projects/@me/metrics/batch",
    expected_json_body
  );

  client
    .post_timestamped_metrics(timestamped_metrics)
    .await
    .unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn get_votes() {
  let (server, client) = mock_server_project_client!();
  let endpoint = mock_get!(server, "/projects/@me/votes", "get_projects_id_votes.json");

  let _ = client
    .get_votes(Utc::now() - Duration::hours(1))
    .await
    .unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn get_votes_after() {
  let (server, client) = mock_server_project_client!();
  let endpoint = mock_get!(
    server,
    "/projects/@me/votes",
    "get_projects_id_votes_cursor.json"
  );

  let votes = client
    .get_votes(Utc::now() - Duration::hours(1))
    .await
    .unwrap();

  assert!(!votes.is_last());

  let _ = client.get_votes_after(&votes).await.unwrap();

  endpoint.assert_calls(2);
}

#[tokio::test]
#[cfg(feature = "api")]
async fn get_vote() {
  let (server, client) = mock_server_project_client!();
  let endpoint = mock_get!(
    server,
    "/projects/@me/votes/661200758510977084",
    "get_projects_id_votes_id.json"
  );

  let _ = client
    .get_vote(661200758510977084, UserSource::Discord)
    .await
    .unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn get_integrations() {
  let (server, client) = mock_server_application_client!();
  let endpoint = mock_get!(
    server,
    "/projects/364806029876555776/integrations",
    "get_projects_id_integrations.json"
  );

  let _ = client.get_integrations(364806029876555776).await.unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn connect_integration() {
  let (server, client) = mock_server_application_client!();
  let endpoint = mock_endpoint!(
    server,
    Method::PUT,
    "/projects/364806029876555776/integrations/364806029876555776"
  );

  client
    .connect_integration(364806029876555776, 364806029876555776)
    .await
    .unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn disconnect_integration() {
  let (server, client) = mock_server_application_client!();
  let endpoint = mock_endpoint!(
    server,
    Method::DELETE,
    "/projects/364806029876555776/integrations/364806029876555776"
  );

  client
    .disconnect_integration(364806029876555776, 364806029876555776)
    .await
    .unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn get_webhooks() {
  let (server, client) = mock_server_application_client!();
  let endpoint = mock_get!(
    server,
    "/projects/364806029876555776/webhooks",
    "get_projects_id_webhooks.json"
  );

  let _ = client.get_webhooks(364806029876555776).await.unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn create_webhook() {
  let (server, client) = mock_server_application_client!();
  let webhook = BaseWebhook {
    label: String::from("test webhook"),
    url: String::from("http://localhost:3000/webhooks"),
  };
  let expected_json_body = serde_json::to_value(&webhook).unwrap();

  let endpoint = server.mock(|when, then| {
    when
      .method(Method::POST)
      .path("/projects/364806029876555776/webhooks")
      .json_body(expected_json_body)
      .header("content-type", "application/json");

    then
      .status(201)
      .header("content-type", "application/json")
      .body(include_str!("mocks/post_projects_id_webhooks.json"));
  });

  let _ = client
    .create_webhook(364806029876555776, webhook)
    .await
    .unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn delete_webhook() {
  let (server, client) = mock_server_application_client!();
  let endpoint = mock_endpoint!(
    server,
    Method::DELETE,
    "/projects/364806029876555776/webhooks/364806029876555776"
  );

  client
    .delete_webhook(364806029876555776, 364806029876555776)
    .await
    .unwrap();

  endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "api")]
async fn rotate_webhook_secret() {
  let (server, client) = mock_server_application_client!();

  let endpoint = server.mock(|when, then| {
    when
      .method(Method::POST)
      .path("/projects/364806029876555776/webhooks/364806029876555776/rotate")
      .header("content-type", "application/json");

    then
      .status(200)
      .header("content-type", "application/json")
      .body(include_str!(
        "mocks/post_projects_id_webhooks_id_rotate.json"
      ));
  });

  let secret = client
    .rotate_webhook_secret(364806029876555776, 364806029876555776)
    .await
    .unwrap();

  endpoint.assert();

  assert_eq!(secret, "whs_...");
}

#[tokio::test]
#[cfg(feature = "api")]
async fn test_webhook() {
  let (server, client) = mock_server_application_client!();
  let endpoint = mock_endpoint!(
    server,
    Method::POST,
    "/projects/364806029876555776/webhooks/364806029876555776/test"
  );

  client
    .test_webhook(364806029876555776, 364806029876555776)
    .await
    .unwrap();

  endpoint.assert();
}

#[test]
#[cfg(feature = "api")]
fn widgets() {
  assert_eq!(
    super::widget::large(Platform::Discord, ProjectType::Bot, 12345, false),
    "https://top.gg/api/v1/widgets/large/discord/bot/12345"
  );
  assert_eq!(
    super::widget::votes(Platform::Discord, ProjectType::Server, 12345, true),
    "https://top.gg/api/v1/widgets/small/votes/discord/server/12345?hideAvatar=true"
  );
  assert_eq!(
    super::widget::owner(Platform::Roblox, ProjectType::Game, 12345, false),
    "https://top.gg/api/v1/widgets/small/owner/roblox/game/12345"
  );
  assert_eq!(
    super::widget::social(Platform::Discord, ProjectType::Bot, 12345, true),
    "https://top.gg/api/v1/widgets/small/social/discord/bot/12345?hideAvatar=true"
  );
}

#[tokio::test]
#[cfg(feature = "oauth2")]
async fn oauth2_url() {
  let (server, oauth2, url) = mock_oauth2_server_client!();
  let url2 = oauth2.new_url_from(
    String::from(url.get_state()),
    String::from(url.get_code_verifier()),
  );

  assert_eq!(url.get_url(), url2.get_url());

  let endpoint = mock_oauth2_refresh_token!(server);
  let session = url.exchange("code").await.unwrap();

  endpoint.assert();

  assert!(!session.has_expired());

  assert_eq!(session.token, "eyJhbGciOiJIUzI1NiIs...");
  assert_eq!(session.refresh_token, "k2Ll3...");

  let previous_expires_at = session.expires_at.timestamp();
  let client = oauth2.new_client(session, server.clone());

  sleep(StdDuration::from_millis(1500)).await;

  let get_authorized_user_projects_endpoint =
    mock_get!(server, "/users/@me/projects", "get_users_me_projects.json");
  let _ = client.get_authorized_user_projects().await.unwrap();

  assert!(endpoint.calls() >= 2);
  get_authorized_user_projects_endpoint.assert();
  assert_ne!(
    client.get_session().await.expires_at.timestamp(),
    previous_expires_at
  );
}

#[tokio::test]
#[cfg(feature = "oauth2")]
async fn get_authorized_user() {
  let (server, oauth2, url) = mock_oauth2_server_client!();
  let refresh_token_endpoint = mock_oauth2_refresh_token!(server);
  let session = url.exchange("code").await.unwrap();

  refresh_token_endpoint.assert();

  let client = oauth2.new_client(session, server.clone());

  let get_authorized_user_endpoint = mock_get!(server, "/users/@me", "get_users_me.json");
  let _ = client.get_authorized_user().await.unwrap();

  get_authorized_user_endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "oauth2")]
async fn get_authorized_user_projects() {
  let (server, oauth2, url) = mock_oauth2_server_client!();
  let refresh_token_endpoint = mock_oauth2_refresh_token!(server);
  let session = url.exchange("code").await.unwrap();

  refresh_token_endpoint.assert();

  let client = oauth2.new_client(session, server.clone());

  let get_authorized_user_projects_endpoint =
    mock_get!(server, "/users/@me/projects", "get_users_me_projects.json");
  let _ = client.get_authorized_user_projects().await.unwrap();

  get_authorized_user_projects_endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "oauth2")]
async fn submit_project() {
  let (server, oauth2, url) = mock_oauth2_server_client!();
  let refresh_token_endpoint = mock_oauth2_refresh_token!(server);
  let session = url.exchange("code").await.unwrap();

  refresh_token_endpoint.assert();

  let client = oauth2.new_client(session, server.clone());

  let submission = ProjectSubmission {
    platform: Platform::Discord,
    r#type: ProjectType::Bot,
    platform_id: 12345,
    headline: String::from("hello this is a headline"),
    page_content: "hello this is a page content".repeat(20),
  };
  let expected_json_body = serde_json::to_value(&submission).unwrap();

  let submit_project_endpoint = mock_endpoint!(
    server,
    Method::POST,
    "/users/@me/projects",
    expected_json_body
  );
  client.submit_project(submission).await.unwrap();

  let faulty_submission = ProjectSubmission {
    platform: Platform::Discord,
    r#type: ProjectType::Bot,
    platform_id: 12345,
    headline: String::from("hello this is a headline"),
    page_content: String::from("hello this is a page content"),
  };

  assert!(matches!(
    client.submit_project(faulty_submission).await,
    Err(Error::LengthConstraints { .. })
  ));

  submit_project_endpoint.assert();
}

#[tokio::test]
#[cfg(feature = "oauth2")]
async fn revoke_token() {
  let (server, oauth2, url) = mock_oauth2_server_client!();
  let refresh_token_endpoint = mock_oauth2_refresh_token!(server);
  let session = url.exchange("code").await.unwrap();

  refresh_token_endpoint.assert();

  let client = oauth2.new_client(session, server.clone());

  let revoke_token_endpoint = server.mock(|when, then| {
    when
      .method(Method::POST)
      .path("/oauth2/revoke")
      .header("content-type", "application/x-www-form-urlencoded");

    then.status(204);
  });
  client.revoke_token().await.unwrap();

  revoke_token_endpoint.assert();
}

#[test]
#[cfg(feature = "webhooks")]
fn parse_webhook_payloads() {
  webhook_payload_works!("vote_create_payload.json", webhooks::Payload::VoteCreate(_));
  webhook_payload_works!("test_payload.json", webhooks::Payload::Test(_));
  webhook_payload_works!(
    "integration_create_payload.json",
    webhooks::Payload::IntegrationCreate(_)
  );
  webhook_payload_works!(
    "integration_delete_payload.json",
    webhooks::Payload::IntegrationDelete(_)
  );
}

#[test]
#[cfg(feature = "webhooks")]
fn verify_webhooks() {
  let secret = "abcdefghijklmnopqrstuvwxyz";
  let now = Utc::now().timestamp();
  let body = "{\"type\":\"webhook.test\",\"data\":{\"user\":{\"id\":\"8226924471638491136\",\"platform_id\":\"661200758510977084\",\"name\":\"null8626\",\"avatar_url\":\"https://cdn.discordapp.com/avatars/661200758510977084/4354318859c319c7ca39753260fdd350.png\"},\"project\":{\"id\":\"364806029876555776\",\"type\":\"bot\",\"platform\":\"discord\",\"platform_id\":\"1026525568344264724\"}}}";

  let mut hmac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
  hmac.update(format!("{now}.{body}").as_bytes());

  let expected_signature = hex::encode(hmac.finalize().into_bytes());

  assert!(!webhooks::verify(secret, "", body));
  assert!(!webhooks::verify(
    secret,
    &format!("t={},v1={expected_signature}", now - 400),
    body
  ));
  assert!(!webhooks::verify(
    secret,
    &format!("t={now},v1={}", &expected_signature[..10]),
    body
  ));
  assert!(!webhooks::verify(
    secret,
    &format!("t=,v1={expected_signature}"),
    body
  ));

  assert!(webhooks::verify(
    secret,
    &format!("t={now},v1={expected_signature}"),
    body
  ));
}
