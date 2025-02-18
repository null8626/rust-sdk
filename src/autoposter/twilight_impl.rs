use crate::autoposter::Handler;
use std::{collections::HashSet, ops::DerefMut};
use tokio::sync::{Mutex, RwLock};
use twilight_model::gateway::event::Event;

/// A built-in [`Handler`] for the [twilight](https://twilight.rs) library.
pub struct Twilight {
  cache: Mutex<HashSet<u64>>,
  server_count: RwLock<usize>,
}

impl Twilight {
  #[inline(always)]
  pub(super) fn new() -> Self {
    Self {
      cache: Mutex::const_new(HashSet::new()),
      server_count: RwLock::new(0),
    }
  }

  /// Handles an entire [twilight](https://twilight.rs) [`Event`] enum.
  pub async fn handle(&self, event: &Event) {
    match event {
      Event::Ready(ready) => {
        let mut cache: tokio::sync::MutexGuard<'_, HashSet<u64>> = self.cache.lock().await;
        let mut server_count = self.server_count.write().await;
        let cache_ref = cache.deref_mut();

        *cache_ref = ready.guilds.iter().map(|guild| guild.id.get()).collect();
        *server_count = cache.len();
      }

      Event::GuildCreate(guild_create) => {
        let mut cache = self.cache.lock().await;

        if cache.insert(guild_create.0.id.get()) {
          let mut server_count = self.server_count.write().await;

          *server_count = cache.len();
        }
      }

      Event::GuildDelete(guild_delete) => {
        let mut cache = self.cache.lock().await;

        if cache.remove(&guild_delete.id.get()) {
          let mut server_count = self.server_count.write().await;

          *server_count = cache.len();
        }
      }

      _ => {}
    }
  }
}

impl Handler for Twilight {
  #[inline(always)]
  fn server_count(&self) -> &RwLock<usize> {
    &self.server_count
  }
}
