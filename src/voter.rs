use crate::{snowflake, util};
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Voted {
  pub(crate) voted: u8,
}

util::debug_struct! {
  /// A struct representing a user who has voted on a bot listed on [Top.gg](https://top.gg). (See [`Client::get_voters`][crate::Client::get_voters])
  #[must_use]
  #[derive(Clone, Deserialize)]
  Voter {
    public {
      /// The Discord ID of this user.
      #[serde(deserialize_with = "snowflake::deserialize")]
      id: u64,

      /// The username of this user.
      username: String,
    }

    private {
      avatar: Option<String>,
    }

    getters(self) {
      /// Retrieves the creation date of this user.
      #[must_use]
      #[inline(always)]
      created_at: DateTime<Utc> => {
        util::get_creation_date(self.id)
      }

      /// Retrieves the Discord avatar URL of this user.
      ///
      /// Its format will either be PNG or GIF if animated.
      #[must_use]
      #[inline(always)]
      avatar: String => {
        util::get_avatar(&self.avatar, self.id)
      }
    }
  }
}
