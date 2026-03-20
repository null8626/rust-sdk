use super::IncomingPayload;
use std::{
  fmt::{self, Display, Formatter},
  future::Future,
  pin::Pin,
  task::{Context, Poll, ready},
  time::{Duration, Instant},
};

use actix_web::{
  FromRequest, HttpRequest, HttpResponse, ResponseError, body::BoxBody, dev::Payload,
  http::StatusCode,
};
use chrono::{DateTime, Utc};
use futures_core::stream::Stream;
use log::warn;

#[doc(hidden)]
#[derive(Debug)]
pub enum IncomingPayloadError {
  ParseFailure,
  Unauthorized,
  Timeout,
}

impl Display for IncomingPayloadError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::ParseFailure => "Unable to parse Top.gg webhook payload.",

      Self::Unauthorized => "Unauthorized.",

      Self::Timeout => "Request timed out.",
    })
  }
}

impl ResponseError for IncomingPayloadError {
  fn error_response(&self) -> HttpResponse<BoxBody> {
    match self {
      Self::ParseFailure => HttpResponse::NoContent().body(()),

      Self::Unauthorized => HttpResponse::Unauthorized().body("Unauthorized"),

      Self::Timeout => HttpResponse::RequestTimeout().body("Request timed out"),
    }
  }

  fn status_code(&self) -> StatusCode {
    match self {
      Self::ParseFailure => StatusCode::NO_CONTENT,

      Self::Unauthorized => StatusCode::UNAUTHORIZED,

      Self::Timeout => StatusCode::REQUEST_TIMEOUT,
    }
  }
}

#[doc(hidden)]
pub struct IncomingPayloadFut {
  req: HttpRequest,
  payload: Payload,
  body: Vec<u8>,
  start: Instant,
  now: DateTime<Utc>,
}

impl IncomingPayloadFut {
  fn timed_out(&self) -> bool {
    self.start.elapsed() > Duration::from_secs(5)
  }
}

impl Future for IncomingPayloadFut {
  type Output = Result<IncomingPayload, IncomingPayloadError>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    if self.timed_out() {
      return Poll::Ready(Err(IncomingPayloadError::Timeout));
    }

    while let Some(body) = ready!(Pin::new(&mut self.payload).poll_next(cx)) {
      if self.timed_out() {
        return Poll::Ready(Err(IncomingPayloadError::Timeout));
      }

      match body {
        Ok(body) => self.body.extend_from_slice(&body),

        Err(err) => {
          warn!(
            "Unable to parse Top.gg webhook payload. Please report this bug to the SDK maintainers: {err:?}"
          );

          return Poll::Ready(Err(IncomingPayloadError::ParseFailure));
        }
      }
    }

    let headers = self.req.headers();

    if let (Some(signature), Some(trace)) = (
      headers.get("x-topgg-signature"),
      headers.get("x-topgg-trace"),
    ) && let (Ok(signature), Ok(trace)) = (signature.to_str(), trace.to_str())
      && let Some(incoming) = IncomingPayload::new(&self.now, signature, self.body.clone(), trace)
    {
      return Poll::Ready(Ok(incoming));
    }

    Poll::Ready(Err(IncomingPayloadError::Unauthorized))
  }
}

#[cfg_attr(docsrs, doc(cfg(feature = "actix-web")))]
impl FromRequest for IncomingPayload {
  type Error = IncomingPayloadError;
  type Future = IncomingPayloadFut;

  fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
    IncomingPayloadFut {
      req: req.clone(),
      payload: payload.take(),
      body: vec![],
      start: Instant::now(),
      now: Utc::now(),
    }
  }
}
