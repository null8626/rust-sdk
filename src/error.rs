// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

use reqwest::Response;
use serde::Deserialize;
use std::fmt::{self, Display, Formatter};
#[cfg(feature = "api")]
use std::ops::RangeInclusive;

/// The SDK's base error type.
#[derive(Debug)]
pub enum Error {
  /// Unable to send a HTTP request to the Top.gg API.
  Outgoing(reqwest::Error),

  /// Unable to read the HTTP response body.
  ReadBody(reqwest::Error),

  /// Received a non-429 unfavorable response from the Top.gg API.
  Incoming(RequestError),

  /// The client is being ratelimited from sending more HTTP requests to the Top.gg API.
  Ratelimited {
    /// The amount of seconds before the ratelimit is lifted.
    retry_after: u16,
  },

  /// A array length was outside the expected length range.
  LengthConstraints {
    /// The actual length.
    actual: usize,

    /// The length range the array length was expected to be within.
    expected: RangeInclusive<usize>,
  },

  /// Unable to serialize the request's JSON body.
  Serialization(serde_json::Error),

  /// Unable to deserialize JSON response from the Top.gg API.
  Deserialization(serde_json::Error),

  /// Attempted to traverse to the next page when [`Paginated::is_last`][super::models::Paginated::is_last] is `true`.
  LastPage,
}

impl Display for Error {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Self::Outgoing(error) => write!(
        f,
        "Unable to send a HTTP request to the Top.gg API: {error}"
      ),

      Self::ReadBody(error) => write!(f, "Unable to read the HTTP response body: {error}"),

      Self::Incoming(error) => Display::fmt(error, f),

      Self::Ratelimited { retry_after } => write!(
        f,
        "The client is being ratelimited from sending more HTTP requests to the Top.gg API. Try again in {retry_after} seconds"
      ),

      Self::LengthConstraints { actual, expected } => write!(
        f,
        "A array length ({actual}) was outside the expected length range ({expected:?})."
      ),

      Self::Serialization(error) => {
        write!(f, "Unable to serialize the request's JSON body: {error}")
      }

      Self::Deserialization(error) => write!(
        f,
        "Unable to deserialize JSON response from the Top.gg API: {error}"
      ),

      Self::LastPage => {
        f.write_str("Attempted to traverse to the next page when Paginated::is_last is true.")
      }
    }
  }
}

impl std::error::Error for Error {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::Outgoing(error) | Self::ReadBody(error) => Some(error),

      Self::Incoming(error) => Some(error),

      Self::Serialization(error) | Self::Deserialization(error) => Some(error),

      _ => None,
    }
  }
}

/// Thrown upon HTTP request failure.
#[derive(Debug, Deserialize)]
pub struct RequestError {
  /// The URI identifying the error type.
  pub r#type: Option<String>,

  /// The error's short human-readable summary.
  pub title: Option<String>,

  /// The error's HTTP status code.
  pub status: u16,

  /// The human-readable explanation of this specific occurrence.
  pub detail: Option<String>,

  /// The error's code (e.g. `PROJECT_NOT_FOUND`.)
  #[serde(default, alias = "error")]
  pub code: Option<String>,
}

impl RequestError {
  pub(super) async fn new(response: Response) -> Self {
    let status = response.status().as_u16();

    if let Ok(bytes) = response.bytes().await
      && let Ok(data) = serde_json::from_slice(&bytes)
    {
      data
    } else {
      Self {
        r#type: None,
        title: None,
        status,
        detail: None,
        code: None,
      }
    }
  }
}

impl Display for RequestError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Got {}: {}",
      self.status,
      self.detail.as_deref().unwrap_or("No description")
    )
  }
}

impl std::error::Error for RequestError {}
