// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

use std::fmt::{self, Display, Formatter};
#[cfg(feature = "api")]
use std::ops::RangeInclusive;

/// The SDK's base error type.
#[derive(Debug)]
pub enum Error {
  /// Unable to send a HTTP request to the Top.gg API.
  #[cfg(feature = "api")]
  Outgoing(reqwest::Error),

  /// Received a non-429 unfavorable response from the Top.gg API.
  #[cfg(feature = "api")]
  Incoming(reqwest::Error),

  /// The client is being ratelimited from sending more HTTP requests to the Top.gg API.
  #[cfg(feature = "api")]
  Ratelimited {
    /// The amount of seconds before the ratelimit is lifted.
    retry_after: u16,
  },

  /// A array length was outside the expected length range.
  #[cfg(feature = "api")]
  LengthConstraints {
    /// The actual length.
    actual: usize,

    /// The length range the array length was expected to be within.
    expected: RangeInclusive<usize>,
  },

  /// Unable to serialize the request's JSON body.
  #[cfg(feature = "api")]
  Serialization(serde_json::Error),

  /// Unable to deserialize JSON response from the Top.gg API.
  #[cfg(feature = "api")]
  Deserialization(serde_json::Error),

  /// Unable to deserialize an enum value from the Top.gg API.
  EnumDeserialization(String),

  /// Attempted to traverse to the next page when [`Paginated::is_last`][super::models::Paginated::is_last] is `true`.
  #[cfg(feature = "api")]
  LastPage,
}

impl Display for Error {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      #[cfg(feature = "api")]
      Self::Outgoing(error) => write!(
        f,
        "Unable to send a HTTP request to the Top.gg API: {error}"
      ),

      #[cfg(feature = "api")]
      Self::Incoming(error) => write!(
        f,
        "Received a non-429 unfavorable response from the Top.gg API: {error}"
      ),

      #[cfg(feature = "api")]
      Self::Ratelimited { retry_after } => write!(
        f,
        "The client is being ratelimited from sending more HTTP requests to the Top.gg API. Try again in {retry_after} seconds"
      ),

      #[cfg(feature = "api")]
      Self::LengthConstraints { actual, expected } => write!(
        f,
        "A array length ({actual}) was outside the expected length range ({expected:?})."
      ),

      #[cfg(feature = "api")]
      Self::Serialization(error) => {
        write!(f, "Unable to serialize the request's JSON body: {error}")
      }

      #[cfg(feature = "api")]
      Self::Deserialization(error) => write!(
        f,
        "Unable to deserialize JSON response from the Top.gg API: {error}"
      ),

      Self::EnumDeserialization(value) => write!(
        f,
        "Unable to deserialize an enum value from the Top.gg API: '{value}'"
      ),

      #[cfg(feature = "api")]
      Self::LastPage => {
        f.write_str("Attempted to traverse to the next page when Paginated::is_last is true.")
      }
    }
  }
}

impl std::error::Error for Error {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    #[cfg(feature = "api")]
    match self {
      Self::Outgoing(error) | Self::Incoming(error) => Some(error),

      Self::Serialization(error) | Self::Deserialization(error) => Some(error),

      _ => None,
    }

    #[cfg(not(feature = "api"))]
    None
  }
}
