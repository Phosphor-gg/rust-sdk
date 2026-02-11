use crate::{
  autoposter::{Handler, SharedStats},
  InnerClient,
};
use std::{
  collections::HashSet,
  sync::Arc,
  time::{Duration, Instant},
};
use tokio::sync::Mutex;
use twilight_model::gateway::event::Event;

/// A built-in [`Handler`] for the [twilight](https://twilight.rs) library.
pub struct Twilight {
  cache: Mutex<HashSet<u64>>,
  stats: SharedStats,
  client: Arc<InnerClient>,
  min_interval: Duration,
  last_post: Mutex<Option<Instant>>,
}

impl Twilight {
  #[inline(always)]
  pub(super) fn new(client: Arc<InnerClient>, min_interval: Duration) -> Self {
    Self {
      cache: Mutex::const_new(HashSet::new()),
      stats: SharedStats::new(),
      client,
      min_interval,
      last_post: Mutex::const_new(None),
    }
  }

  /// Attempts to post stats if the minimum interval has passed since the last post.
  async fn try_post(&self) {
    let now = Instant::now();
    let mut last = self.last_post.lock().await;
    if last.map_or(true, |l| now.duration_since(l) >= self.min_interval) {
      *last = Some(now);
      drop(last);

      let stats = self.stats.stats.read().await;
      if let Err(e) = self.client.post_stats(&stats).await {
        eprintln!("Failed to post bot stats: {}", e);
      }
    }
  }

  /// Handles an entire [twilight](https://twilight.rs) [`Event`] enum.
  pub async fn handle(&self, event: &Event) {
    match event {
      Event::Ready(ready) => {
        {
          let mut cache = self.cache.lock().await;
          *cache = ready.guilds.iter().map(|guild| guild.id.get()).collect();

          let mut stats = self.stats.write().await;
          stats.set_server_count(cache.len());
        }
        self.try_post().await;
      }

      Event::GuildCreate(guild_create) => {
        {
          let mut cache = self.cache.lock().await;
          if cache.insert(guild_create.0.id.get()) {
            let mut stats = self.stats.write().await;
            stats.set_server_count(cache.len());
          }
        }
        self.try_post().await;
      }

      Event::GuildDelete(guild_delete) => {
        {
          let mut cache = self.cache.lock().await;
          if cache.remove(&guild_delete.id.get()) {
            let mut stats = self.stats.write().await;
            stats.set_server_count(cache.len());
          }
        }
        self.try_post().await;
      }

      _ => {}
    }
  }
}

impl Handler for Twilight {
  #[inline(always)]
  fn stats(&self) -> &SharedStats {
    &self.stats
  }
}
