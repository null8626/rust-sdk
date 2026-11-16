// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

use std::fmt::{self, Display, Formatter};
#[cfg(feature = "api")]
use std::ops::RangeInclusive;

/// The SDK's base error type.
#[derive(Debug)]
pub enum Error {
  /// Unable to send a HTTP request to the Top.gg API.
  Outgoing(reqwest::Error),

  /// Received a non-429 unfavorable response from the Top.gg API.
  Incoming(reqwest::Error),

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

      Self::Incoming(error) => write!(
        f,
        "Received a non-429 unfavorable response from the Top.gg API: {error}"
      ),

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
      Self::Outgoing(error) | Self::Incoming(error) => Some(error),

      Self::Serialization(error) | Self::Deserialization(error) => Some(error),

      _ => None,
    }
  }
}
