// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2023-2026 null8626 & Top.gg

#[cfg(feature = "api")]
use super::Error;
#[cfg(feature = "oauth2")]
use rand::Rng;
#[cfg(feature = "api")]
use reqwest::{Client as HttpClient, IntoUrl, Method, Response, StatusCode, Version, header};
#[cfg(feature = "oauth2")]
use serde::Serializer;
use serde::{Deserialize, Deserializer, de};
#[cfg(feature = "api")]
use std::ops::RangeInclusive;

#[cfg(feature = "api")]
pub fn http_client() -> HttpClient {
  #[cfg(not(test))]
  {
    HttpClient::default()
  }

  #[cfg(test)]
  {
    HttpClient::builder()
      .add_root_certificate(reqwest::Certificate::from_pem(include_bytes!("mocks/ca.pem")).unwrap())
      .build()
      .unwrap()
  }
}

#[cfg(feature = "api")]
pub async fn request(
  http: &HttpClient,
  method: Method,
  url: impl IntoUrl,
  token: Option<&str>,
  content_type: &'static str,
  body: Vec<u8>,
) -> Result<Response, Error> {
  let mut request = http
    .request(method, url)
    .header(
      header::USER_AGENT,
      "topgg (https://github.com/top-gg-community/rust-sdk) Rust",
    )
    .header(header::CONTENT_LENGTH, body.len())
    .header(header::CONTENT_TYPE, content_type)
    .version(Version::HTTP_2)
    .body(body);

  if let Some(token) = token {
    request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
  }

  let response = http
    .execute(request.build().unwrap())
    .await
    .map_err(Error::Outgoing)?;

  if let Err(error) = response.error_for_status_ref() {
    return Err(if response.status() == StatusCode::TOO_MANY_REQUESTS {
      Error::Ratelimited {
        retry_after: if let Some(retry_after2) = response.headers().get("retry-after")
          && let Ok(retry_after3) = retry_after2.to_str()
          && let Ok(retry_after4) = retry_after3.parse()
        {
          retry_after4
        } else {
          0
        },
      }
    } else {
      Error::Incoming(error)
    });
  }

  Ok(response)
}

#[cfg(feature = "api")]
pub fn enforce_constraints(actual: usize, expected: RangeInclusive<usize>) -> Result<(), Error> {
  if expected.contains(&actual) {
    Ok(())
  } else {
    Err(Error::LengthConstraints { actual, expected })
  }
}

#[cfg(feature = "oauth2")]
pub fn secrets64() -> String {
  let mut bytes = [0u8; 64];
  rand::rng().fill_bytes(&mut bytes);

  hex::encode(bytes)
}

#[cfg(feature = "oauth2")]
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn serialize_snowflake<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  serializer.serialize_str(&value.to_string())
}

pub fn deserialize_snowflake<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
  D: Deserializer<'de>,
{
  String::deserialize(deserializer).and_then(|s| s.parse().map_err(de::Error::custom))
}
