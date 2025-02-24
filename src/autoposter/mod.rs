use crate::{Result, Stats};
use core::{
  ops::{Deref, DerefMut},
  time::Duration,
};
use std::sync::Arc;
use tokio::{
  sync::{mpsc, RwLock, RwLockWriteGuard, Semaphore},
  task::{spawn, JoinHandle},
  time::sleep,
};

mod client;

pub use client::AsClient;
pub(crate) use client::AsClientSealed;

cfg_if::cfg_if! {
  if #[cfg(feature = "serenity")] {
    mod serenity_impl;

    #[cfg_attr(docsrs, doc(cfg(feature = "serenity")))]
    pub use serenity_impl::Serenity;
  }
}

cfg_if::cfg_if! {
  if #[cfg(feature = "twilight")] {
    mod twilight_impl;

    #[cfg_attr(docsrs, doc(cfg(feature = "twilight")))]
    pub use twilight_impl::Twilight;
  }
}

/// A struct representing a thread-safe form of the [`Stats`] struct to be used in autoposter [`Handler`]s.
pub struct SharedStats {
  sem: Semaphore,
  stats: RwLock<Stats>,
}

/// A guard wrapping over tokio's [`RwLockWriteGuard`] that lets you freely feed new [`Stats`] data before being sent to the [`Autoposter`].
pub struct SharedStatsGuard<'a> {
  sem: &'a Semaphore,
  guard: RwLockWriteGuard<'a, Stats>,
}

impl SharedStatsGuard<'_> {
  /// Directly replaces the current [`Stats`] inside with the other.
  #[inline(always)]
  pub fn replace(&mut self, other: Stats) {
    let ref_mut = self.guard.deref_mut();
    *ref_mut = other;
  }

  /// Sets the current [`Stats`] server count.
  #[inline(always)]
  pub fn set_server_count(&mut self, server_count: usize) {
    self.guard.server_count = Some(server_count);
  }

  #[deprecated(
    since = "1.4.3",
    note = "No longer supported by Top.gg API v0. At the moment, this method has no effect."
  )]
  pub fn set_shard_count(&mut self, _shard_count: usize) {}
}

impl Deref for SharedStatsGuard<'_> {
  type Target = Stats;

  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    self.guard.deref()
  }
}

impl DerefMut for SharedStatsGuard<'_> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.guard.deref_mut()
  }
}

impl Drop for SharedStatsGuard<'_> {
  #[inline(always)]
  fn drop(&mut self) {
    if self.sem.available_permits() < 1 {
      self.sem.add_permits(1);
    }
  }
}

impl SharedStats {
  /// Creates a new [`SharedStats`] struct. Before any modifications, the [`Stats`] struct inside defaults to zero server count.
  #[inline(always)]
  pub fn new() -> Self {
    Self {
      sem: Semaphore::const_new(0),
      stats: RwLock::new(Stats::from(0)),
    }
  }

  /// Locks this [`SharedStats`] with exclusive write access, causing the current task to yield until the lock has been acquired. This is akin to [`RwLock::write`].
  #[inline(always)]
  pub async fn write<'a>(&'a self) -> SharedStatsGuard<'a> {
    SharedStatsGuard {
      sem: &self.sem,
      guard: self.stats.write().await,
    }
  }

  #[inline(always)]
  async fn wait(&self) {
    self.sem.acquire().await.unwrap().forget();
  }
}

/// A trait for handling events from third-party bot libraries.
///
/// The struct implementing this trait should own an [`SharedStats`] struct and update it accordingly whenever Discord updates them with new data regarding guild/shard count.
pub trait Handler: Send + Sync + 'static {
  /// The method that borrows [`SharedStats`] to the [`Autoposter`].
  fn stats(&self) -> &SharedStats;
}

/// A struct that lets you automate the process of posting bot statistics to [Top.gg](https://top.gg) in intervals.
///
/// **NOTE:** This struct owns the thread handle that executes the automatic posting. The autoposter thread will stop once this struct is dropped.
#[must_use]
pub struct Autoposter<H> {
  handler: Arc<H>,
  thread: JoinHandle<()>,
  receiver: Option<mpsc::UnboundedReceiver<Result<()>>>,
}

impl<H> Autoposter<H>
where
  H: Handler,
{
  /// Creates an [`Autoposter`] struct as well as immediately starting the thread. The thread will never stop until this struct gets dropped.
  ///
  /// - `client` can either be a reference to an existing [`Client`][crate::Client] or a [`&str`][std::str] representing a [Top.gg API](https://docs.top.gg) token.
  /// - `handler` is a struct that handles the *retrieving stats* part before being sent to the [`Autoposter`]. This datatype is essentially the bridge between an external third-party bot library between this library.
  ///
  /// # Panics
  ///
  /// Panics if the interval argument is shorter than 15 minutes (900 seconds).
  pub fn new<C>(client: &C, handler: H, interval: Duration) -> Self
  where
    C: AsClient,
  {
    assert!(
      interval.as_secs() >= 900,
      "The interval mustn't be shorter than 15 minutes."
    );

    let client = client.as_client();
    let handler = Arc::new(handler);
    let (sender, receiver) = mpsc::unbounded_channel();

    Self {
      handler: Arc::clone(&handler),
      thread: spawn(async move {
        loop {
          handler.stats().wait().await;

          {
            let stats = handler.stats().stats.read().await;

            if sender.send(client.post_stats(&stats).await).is_err() {
              break;
            }
          };

          sleep(interval).await;
        }
      }),
      receiver: Some(receiver),
    }
  }

  /// Retrieves the [`Handler`] inside in the form of a [cloned][Arc::clone] [`Arc<H>`][Arc].
  #[inline(always)]
  pub fn handler(&self) -> Arc<H> {
    Arc::clone(&self.handler)
  }

  /// Returns a future that resolves every time the [`Autoposter`] has attempted to post the bot's stats. If you want to use the receiver directly, call [`receiver`][Autoposter::receiver].
  #[inline(always)]
  pub async fn recv(&mut self) -> Option<Result<()>> {
    self.receiver.as_mut().expect("receiver is already taken from the receiver() method. please call recv() directly from the receiver.").recv().await
  }

  /// Takes the receiver responsible for [`recv`][Autoposter::recv]. Subsequent calls to this function and [`recv`][Autoposter::recv] after this call will panic.
  #[inline(always)]
  pub fn receiver(&mut self) -> mpsc::UnboundedReceiver<Result<()>> {
    self
      .receiver
      .take()
      .expect("receiver() can only be called once.")
  }
}

impl<H> Deref for Autoposter<H> {
  type Target = H;

  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    self.handler.deref()
  }
}

#[cfg(feature = "serenity")]
#[cfg_attr(docsrs, doc(cfg(feature = "serenity")))]
impl Autoposter<Serenity> {
  /// Creates an [`Autoposter`] struct from an existing built-in [serenity] [`Handler`] as well as immediately starting the thread. The thread will never stop until this struct gets dropped.
  ///
  /// - `client` can either be a reference to an existing [`Client`][crate::Client] or a [`&str`][std::str] representing a [Top.gg API](https://docs.top.gg) token.
  ///
  /// # Panics
  ///
  /// Panics if the interval argument is shorter than 15 minutes (900 seconds).
  #[inline(always)]
  pub fn serenity<C>(client: &C, interval: Duration) -> Self
  where
    C: AsClient,
  {
    Self::new(client, Serenity::new(), interval)
  }
}

#[cfg(feature = "twilight")]
#[cfg_attr(docsrs, doc(cfg(feature = "twilight")))]
impl Autoposter<Twilight> {
  /// Creates an [`Autoposter`] struct from an existing built-in [twilight](https://twilight.rs) [`Handler`] as well as immediately starting the thread. The thread will never stop until this struct gets dropped.
  ///
  /// - `client` can either be a reference to an existing [`Client`][crate::Client] or a [`&str`][std::str] representing a [Top.gg API](https://docs.top.gg) token.
  ///
  /// # Panics
  ///
  /// Panics if the interval argument is shorter than 15 minutes (900 seconds).
  #[inline(always)]
  pub fn twilight<C>(client: &C, interval: Duration) -> Self
  where
    C: AsClient,
  {
    Self::new(client, Twilight::new(), interval)
  }
}

impl<H> Drop for Autoposter<H> {
  #[inline(always)]
  fn drop(&mut self) {
    self.thread.abort();
  }
}
