use super::IncomingPayload;

use log::warn;
use rocket::{
  data::{Data, FromData, Outcome, ToByteUnit},
  http::Status,
  request::Request,
};

#[cfg_attr(docsrs, doc(cfg(feature = "rocket")))]
#[rocket::async_trait]
impl<'r> FromData<'r> for IncomingPayload {
  type Error = ();

  async fn from_data(request: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
    let headers = request.headers();

    if let (Some(signature), Some(trace)) = (
      headers.get_one("x-topgg-signature"),
      headers.get_one("x-topgg-trace"),
    ) {
      if let Ok(body) = data.open(2.mebibytes()).into_bytes().await {
        return Self::new(signature, body.into_inner(), trace).map_or_else(|| {
          warn!(
            "Unable to parse Top.gg webhook payload. Please report this bug to the SDK maintainers."
          );

          Outcome::Error((Status::NoContent, ()))
        }, Outcome::Success);
      }

      return Outcome::Error((Status::BadRequest, ()));
    }

    Outcome::Error((Status::Unauthorized, ()))
  }
}
