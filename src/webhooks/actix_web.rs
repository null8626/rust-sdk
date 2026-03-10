use super::IncomingPayload;
use std::{
  fmt::{self, Display, Formatter},
  future::Future,
  pin::Pin,
  task::{Context, Poll, ready},
};

use actix_web::{
  FromRequest, HttpRequest, HttpResponse, ResponseError, body::BoxBody, dev::Payload,
  http::StatusCode,
};
use futures_core::stream::Stream;
use log::warn;

#[doc(hidden)]
#[derive(Debug)]
pub enum IncomingPayloadError {
  ParseFailure,
  Unauthorized,
}

impl Display for IncomingPayloadError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::ParseFailure => "Unable to parse Top.gg webhook payload.",

      Self::Unauthorized => "Unauthorized.",
    })
  }
}

impl ResponseError for IncomingPayloadError {
  fn error_response(&self) -> HttpResponse<BoxBody> {
    match self {
      Self::ParseFailure => HttpResponse::NoContent().body(()),

      Self::Unauthorized => HttpResponse::Unauthorized().body("Unauthorized"),
    }
  }

  fn status_code(&self) -> StatusCode {
    match self {
      Self::ParseFailure => StatusCode::NO_CONTENT,

      Self::Unauthorized => StatusCode::UNAUTHORIZED,
    }
  }
}

#[doc(hidden)]
pub struct IncomingPayloadFut {
  req: HttpRequest,
  payload: Payload,
  body: Vec<u8>,
}

impl Future for IncomingPayloadFut {
  type Output = Result<IncomingPayload, IncomingPayloadError>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    while let Some(body) = ready!(Pin::new(&mut self.payload).poll_next(cx)) {
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
      && let Some(incoming) = IncomingPayload::new(signature, self.body.clone(), trace)
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
    }
  }
}
