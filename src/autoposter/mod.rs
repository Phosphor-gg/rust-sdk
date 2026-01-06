use crate::Stats;
use core::{
  ops::{Deref, DerefMut},
  time::Duration,
};
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockWriteGuard};

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
  stats: RwLock<Stats>,
}

/// A guard wrapping over tokio's [`RwLockWriteGuard`] that lets you freely feed new [`Stats`] data before being sent to the [`Autoposter`].
pub struct SharedStatsGuard<'a> {
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

  /// Sets the current [`Stats`] shard count.
  #[inline(always)]
  pub fn set_shard_count(&mut self, shard_count: usize) {
    self.guard.shard_count = Some(shard_count);
  }
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

impl SharedStats {
  /// Creates a new [`SharedStats`] struct. Before any modifications, the [`Stats`] struct inside defaults to zero server count.
  #[inline(always)]
  pub fn new() -> Self {
    Self {
      stats: RwLock::new(Stats::from(0)),
    }
  }

  /// Locks this [`SharedStats`] with exclusive write access, causing the current task to yield until the lock has been acquired. This is akin to [`RwLock::write`].
  #[inline(always)]
  pub async fn write<'a>(&'a self) -> SharedStatsGuard<'a> {
    SharedStatsGuard {
      guard: self.stats.write().await,
    }
  }
}

/// A trait for handling events from third-party Discord Bot libraries.
///
/// The struct implementing this trait should own an [`SharedStats`] struct and update it accordingly whenever Discord updates them with new data regarding guild/shard count.
pub trait Handler: Send + Sync + 'static {
  /// The method that borrows [`SharedStats`] to the [`Autoposter`].
  fn stats(&self) -> &SharedStats;
}

/// A struct that lets you automate the process of posting bot statistics to [Top.gg](https://top.gg) on guild events with a minimum interval.
///
/// **NOTE:** This struct provides a handler that posts statistics when the bot joins or leaves guilds, ensuring at least the minimum interval between posts.
#[must_use]
pub struct Autoposter<H> {
  handler: Arc<H>,
}

impl<H> Autoposter<H>
where
  H: Handler,
{
  /// Creates an [`Autoposter`] struct.
  ///
  /// - `handler` is a struct that handles the *retrieving stats* part and posting to the [`Autoposter`]. This datatype is essentially the bridge between an external third-party Discord Bot library between this library.
  ///
  /// # Panics
  ///
  /// Panics if the interval argument is shorter than 15 minutes (900 seconds).
  pub fn new(handler: H, interval: Duration) -> Self {
    assert!(
      interval.as_secs() >= 900,
      "The interval mustn't be shorter than 15 minutes."
    );

    let handler = Arc::new(handler);

    Self { handler }
  }

  /// Retrieves the [`Handler`] inside in the form of a [cloned][Arc::clone] [`Arc<H>`][Arc].
  #[inline(always)]
  pub fn handler(&self) -> Arc<H> {
    Arc::clone(&self.handler)
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
  /// Creates an [`Autoposter`] struct from an existing built-in [serenity] [`Handler`].
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
    let c = client.as_client();
    Self::new(Serenity::new(Arc::clone(&c), interval), interval)
  }
}

#[cfg(feature = "twilight")]
#[cfg_attr(docsrs, doc(cfg(feature = "twilight")))]
impl Autoposter<Twilight> {
  /// Creates an [`Autoposter`] struct from an existing built-in [twilight](https://twilight.rs) [`Handler`].
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
    let c = client.as_client();
    Self::new(Twilight::new(Arc::clone(&c), interval), interval)
  }
}
