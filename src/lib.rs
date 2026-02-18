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
    mod error;
    mod project;
    mod user;
    mod util;

    pub use client::Client;
    pub use error::{Error, PostBotCommandsError, PostBotCommandsResult, Result};
    pub use project::{GetCommands, Project, ProjectType, Platform};
    pub use snowflake::Snowflake; // for doc purposes
    pub use user::{UserSource, Vote};

    #[doc(hidden)]
    #[cfg(any(feature = "twilight", feature = "twilight-cached"))]
    pub use project::TwilightGetCommandsError;
  }
}

cfg_if::cfg_if! {
  if #[cfg(feature = "webhooks")] {
    mod webhooks;

    pub use webhooks::*;
  }
}
