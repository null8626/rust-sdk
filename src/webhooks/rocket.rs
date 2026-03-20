use super::IncomingPayload;
use std::time::Duration;

use chrono::Utc;
use log::warn;
use rocket::{
  data::{Data, FromData, Outcome, ToByteUnit},
  http::Status,
  request::Request,
};
use tokio::time::timeout;

#[cfg_attr(docsrs, doc(cfg(feature = "rocket")))]
#[rocket::async_trait]
impl<'r> FromData<'r> for IncomingPayload {
  type Error = ();

  async fn from_data(request: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
    let now = Utc::now();
    let headers = request.headers();

    if let (Some(signature), Some(trace)) = (
      headers.get_one("x-topgg-signature"),
      headers.get_one("x-topgg-trace"),
    ) {
      return match timeout(Duration::from_secs(5), data.open(2.mebibytes()).into_bytes()).await {
        Ok(Ok(body)) => {
          Self::new(&now, signature, body.into_inner(), trace).map_or_else(|| {
            warn!(
              "Unable to parse Top.gg webhook payload. Please report this bug to the SDK maintainers."
            );

            Outcome::Error((Status::NoContent, ()))
          }, Outcome::Success)
        },

        Err(_) => Outcome::Error((Status::RequestTimeout, ())),

        _ => Outcome::Error((Status::BadRequest, ())),
      };
    }

    Outcome::Error((Status::Unauthorized, ()))
  }
}
