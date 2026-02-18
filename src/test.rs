use super::Client;

use tokio::time::{sleep, Duration};

macro_rules! delayed {
  ($($b:tt)*) => {
    $($b)*
    sleep(Duration::from_secs(1)).await
  };
}

#[tokio::test]
async fn api() {
  let client = Client::new(env!("TOPGG_TOKEN").to_string());

  todo!()
}
