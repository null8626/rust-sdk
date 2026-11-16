// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

mod models;

pub use models::*;

/// Verifies an incoming webhook request.
#[must_use]
pub fn verify(secret: &str, signature: &str, body: &str) -> bool {
  let mut parts = signature.split(',');

  let (Some(timestamp_pair), Some(signature_pair)) = (parts.next(), parts.next()) else {
    return false;
  };

  let (Some(timestamp), Some(signature)) = (
    timestamp_pair
      .find('=')
      .and_then(|index| timestamp_pair.get(index + 1..))
      .and_then(|timestamp| timestamp.parse::<i64>().ok()),
    signature_pair
      .find('=')
      .and_then(|index| signature_pair.get(index + 1..)),
  ) else {
    return false;
  };

  if (Utc::now().timestamp() - timestamp).abs() > 300 {
    return false;
  }

  let (Ok(decoded_signature), Ok(mut hmac)) = (
    hex::decode(signature),
    Hmac::<Sha256>::new_from_slice(secret.as_bytes()),
  ) else {
    return false;
  };

  hmac.update(format!("{timestamp}.{body}").as_bytes());
  hmac.verify_slice(&decoded_signature).is_ok()
}
