use crate::Client;
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

  delayed! {
    let user = client.get_user(661200758510977084).await.unwrap();

    assert_eq!(user.username, "null");
    assert_eq!(user.id, 661200758510977084);
  }

  delayed! {
    let bot = client.get_bot(264811613708746752).await.unwrap();

    assert_eq!(bot.username, "Luca");
    assert_eq!(bot.id, 264811613708746752);
  }

  delayed! {
    let _bots = client
      .get_bots()
      .limit(250)
      .skip(50)
      .username("shiro")
      .sort_by_monthly_votes()
      .await
      .unwrap();
  }

  delayed! {
    client
    .post_server_count(2)
    .await
    .unwrap();
  }

  delayed! {
    assert_eq!(client.get_server_count().await.unwrap().unwrap(), 2);
  }

  delayed! {
    let _has_voted = client.has_voted(661200758510977084).await.unwrap();
  }

  delayed! {
    let _is_weekend = client.is_weekend().await.unwrap();
  }
}
