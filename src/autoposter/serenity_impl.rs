use crate::autoposter::{Handler, SharedStats};
use paste::paste;
use serenity::{
  client::{Context, EventHandler, FullEvent},
  model::{
    gateway::Ready,
    guild::{Guild, UnavailableGuild},
    id::GuildId,
  },
};

cfg_if::cfg_if! {
  if #[cfg(not(feature = "serenity-cached"))] {
    use std::collections::HashSet;
    use tokio::sync::Mutex;

    struct Cache {
      guilds: HashSet<GuildId>,
    }
  } else {
    use std::ops::Add;
  }
}

/// A built-in [`Handler`] for the [serenity] library.
#[must_use]
pub struct Serenity {
  #[cfg(not(feature = "serenity-cached"))]
  cache: Mutex<Cache>,
  stats: SharedStats,
}

macro_rules! serenity_handler {
  (
    ($self:ident, $context: ident) => {$(
      $(#[$attr:meta])?
      $handler_name:ident {
        map($($map_arg_name:ident: $map_arg_type:ty),*) $map_expr:tt
        handle($($(#[$handle_arg_attr:meta])?$handle_arg_name:ident: $handle_arg_type:ty),*) $handle_expr:tt
      }
    )*}
  ) => {
    paste! {
      #[allow(unused_variables)]
      impl Serenity {
        #[inline(always)]
        pub(super) fn new() -> Self {
          Self {
            #[cfg(not(feature = "serenity-cached"))]
            cache: Mutex::const_new(Cache {
              guilds: HashSet::new(),
            }),
            stats: SharedStats::new(),
          }
        }

        /// Handles an entire [serenity] [`FullEvent`] enum. This can be used in [serenity] frameworks.
        pub async fn handle(&$self, $context: &Context, event: &FullEvent) {
          match event {
            $(
              $(#[$attr])?
              FullEvent::[<$handler_name:camel>] { $($map_arg_name),* } => $map_expr,
            )*

            _ => {}
          }
        }

        $(
          $(#[$attr])?
          async fn [<handle_ $handler_name>](
            &$self,
            $(
              $(#[$handle_arg_attr])? $handle_arg_name: $handle_arg_type,
            )*
          ) $handle_expr
        )*
      }

      #[serenity::async_trait]
      #[allow(unused_variables)]
      impl EventHandler for Serenity {
        $(
          #[inline(always)]
          $(#[$attr])?
          async fn $handler_name(&$self, $context: Context, $($map_arg_name: $map_arg_type),*) $map_expr
        )*
      }
    }
  };
}

serenity_handler! {
  (self, context) => {
    ready {
      map(data_about_bot: Ready) {
        self.handle_ready(&data_about_bot.guilds).await
      }

      handle(guilds: &[UnavailableGuild]) {
        let mut stats = self.stats.write().await;

        stats.set_server_count(guilds.len());

        cfg_if::cfg_if! {
          if #[cfg(not(feature = "serenity-cached"))] {
            let mut cache = self.cache.lock().await;

            cache.guilds = guilds.iter().map(|x| x.id).collect();
          }
        }
      }
    }

    #[cfg(feature = "serenity-cached")]
    cache_ready {
      map(guilds: Vec<GuildId>) {
        self.handle_cache_ready(guilds.len()).await
      }

      handle(guild_count: usize) {
        let mut stats = self.stats.write().await;

        stats.set_server_count(guild_count);
      }
    }

    guild_create {
      map(guild: Guild, is_new: Option<bool>) {
        self.handle_guild_create(
          #[cfg(not(feature = "serenity-cached"))] guild.id,
          #[cfg(feature = "serenity-cached")] context.cache.guilds().len(),
          #[cfg(feature = "serenity-cached")] is_new.expect("serenity-cached feature is enabled but the bot doesn't cache guilds"),
        ).await
      }

      handle(
        #[cfg(not(feature = "serenity-cached"))] guild_id: GuildId,
        #[cfg(feature = "serenity-cached")] guild_count: usize,
        #[cfg(feature = "serenity-cached")] is_new: bool) {
        cfg_if::cfg_if! {
          if #[cfg(feature = "serenity-cached")] {
            if is_new {
              let mut stats = self.stats.write().await;

              stats.set_server_count(guild_count);
            }
          } else {
            let mut cache = self.cache.lock().await;

            if cache.guilds.insert(guild_id) {
              let mut stats = self.stats.write().await;

              stats.set_server_count(cache.guilds.len());
            }
          }
        }
      }
    }

    guild_delete {
      map(incomplete: UnavailableGuild, full: Option<Guild>) {
        self.handle_guild_delete(
          #[cfg(feature = "serenity-cached")] context.cache.guilds().len(),
          #[cfg(not(feature = "serenity-cached"))] incomplete.id
        ).await
      }

      handle(
        #[cfg(feature = "serenity-cached")] guild_count: usize,
        #[cfg(not(feature = "serenity-cached"))] guild_id: GuildId) {
        cfg_if::cfg_if! {
          if #[cfg(feature = "serenity-cached")] {
            let mut stats = self.stats.write().await;

            stats.set_server_count(guild_count);
          } else {
            let mut cache = self.cache.lock().await;

            if cache.guilds.remove(&guild_id) {
              let mut stats = self.stats.write().await;

              stats.set_server_count(cache.guilds.len());
            }
          }
        }
      }
    }
  }
}

impl Handler for Serenity {
  #[inline(always)]
  fn stats(&self) -> &SharedStats {
    &self.stats
  }
}
