#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(feature = "webhooks", allow(unreachable_patterns))]
#![allow(clippy::needless_pass_by_value)]

mod snowflake;
#[cfg(test)]
mod test;

cfg_if::cfg_if! {
  if #[cfg(feature = "api")] {
    pub(crate) mod client;
    mod bot;
    mod error;
    mod util;
    mod vote;

    #[cfg(feature = "bot-autoposter")]
    pub(crate) use client::InnerClient;

    #[doc(inline)]
    pub use bot::{Bot, BotQuery};
    pub use client::Client;
    pub use error::{Error, Result};
    pub use snowflake::Snowflake; // for doc purposes
    pub use vote::Voter;

    #[doc(hidden)]
    #[cfg(any(feature = "twilight", feature = "twilight-cached"))]
    pub use project::TwilightGetCommandsError;
  }
}

cfg_if::cfg_if! {
  if #[cfg(feature = "bot-autoposter")] {
    mod bot_autoposter;

    #[doc(inline)]
    #[cfg_attr(docsrs, doc(cfg(feature = "bot-autoposter")))]
    pub use bot_autoposter::{BotAutoposter, BotAutoposterHandler};

    #[cfg(any(feature = "serenity", feature = "serenity-cached"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "bot-autoposter", any(feature = "serenity", feature = "serenity-cached")))))]
    pub use bot_autoposter::Serenity as SerenityBotAutoposter;

    #[cfg(any(feature = "twilight", feature = "twilight-cached"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "bot-autoposter", any(feature = "twilight", feature = "twilight-cached")))))]
    pub use bot_autoposter::Twilight as TwilightBotAutoposter;
  }
}

cfg_if::cfg_if! {
  if #[cfg(feature = "webhooks")] {
    mod webhooks;

    pub use webhooks::*;
  }
}
