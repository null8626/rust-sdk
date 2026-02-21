use super::{Error, snowflake};

use base64::Engine;
use reqwest::Response;
use serde::{Deserialize, de::DeserializeOwned};

pub async fn parse_json<T>(response: Response) -> super::Result<T>
where
  T: DeserializeOwned,
{
  if let Ok(bytes) = response.bytes().await
    && let Ok(json) = serde_json::from_slice(&bytes)
  {
    return Ok(json);
  }

  Err(Error::InternalServerError)
}

#[derive(Deserialize)]
#[allow(clippy::used_underscore_binding)]
struct TokenStructure {
  #[serde(deserialize_with = "snowflake::deserialize", rename = "id")]
  _id: u64,
}

pub fn validate_api_token(token: &str) {
  if let Some(base64_section) = token.split('.').nth(1)
    && let Ok(decoded_base64) =
      base64::engine::general_purpose::STANDARD_NO_PAD.decode(base64_section)
    && serde_json::from_slice::<TokenStructure>(&decoded_base64).is_ok()
  {
    return;
  }

  panic!("Got a malformed API token.");
}
