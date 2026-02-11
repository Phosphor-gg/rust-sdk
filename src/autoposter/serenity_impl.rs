use crate::{
  autoposter::{Handler, SharedStats},
  InnerClient,
};
use serenity::{
  client::{Context, EventHandler, FullEvent},
  model::{
    gateway::Ready,
    guild::{Guild, UnavailableGuild},
    id::GuildId,
  },
};
use std::{
  sync::Arc,
  time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[cfg(not(feature = "serenity-cached"))]
use std::collections::HashSet;

/// A built-in [`Handler`] for the [serenity] library.
#[must_use]
pub struct Serenity {
  #[cfg(not(feature = "serenity-cached"))]
  cache: Mutex<HashSet<GuildId>>,
  stats: SharedStats,
  client: Arc<InnerClient>,
  min_interval: Duration,
  last_post: Mutex<Option<Instant>>,
}

#[allow(unused_variables)]
impl Serenity {
  #[inline(always)]
  pub(super) fn new(client: Arc<InnerClient>, min_interval: Duration) -> Self {
    Self {
      #[cfg(not(feature = "serenity-cached"))]
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

  /// Handles an entire [serenity] [`FullEvent`] enum. This can be used in [serenity] frameworks.
  pub async fn handle(&self, ctx: &Context, event: &FullEvent) {
    match event {
      FullEvent::Ready { data_about_bot } => self.on_ready(&data_about_bot.guilds).await,

      #[cfg(feature = "serenity-cached")]
      FullEvent::CacheReady { guilds } => self.on_cache_ready(guilds.len()).await,

      #[cfg(feature = "serenity-cached")]
      FullEvent::ShardsReady { total_shards } => self.on_shards_ready(*total_shards).await,

      FullEvent::GuildCreate { guild, is_new } => {
        cfg_if::cfg_if! {
          if #[cfg(feature = "serenity-cached")] {
            self.on_guild_create(ctx.cache.guilds().len(), is_new.unwrap_or(false)).await;
          } else {
            self.on_guild_create(guild.id).await;
          }
        }
      }

      FullEvent::GuildDelete { incomplete, .. } => {
        cfg_if::cfg_if! {
          if #[cfg(feature = "serenity-cached")] {
            self.on_guild_delete(ctx.cache.guilds().len()).await;
          } else {
            self.on_guild_delete(incomplete.id).await;
          }
        }
      }

      _ => {}
    }
  }

  async fn on_ready(&self, guilds: &[UnavailableGuild]) {
    {
      let mut stats = self.stats.write().await;
      stats.set_server_count(guilds.len());
    }

    #[cfg(not(feature = "serenity-cached"))]
    {
      let mut cache = self.cache.lock().await;
      *cache = guilds.iter().map(|g| g.id).collect();
    }

    self.try_post().await;
  }

  #[cfg(feature = "serenity-cached")]
  async fn on_cache_ready(&self, guild_count: usize) {
    {
      let mut stats = self.stats.write().await;
      stats.set_server_count(guild_count);
    }
    self.try_post().await;
  }

  #[cfg(feature = "serenity-cached")]
  async fn on_shards_ready(&self, shard_count: u32) {
    let mut stats = self.stats.write().await;
    stats.set_shard_count(shard_count as usize);
  }

  #[cfg(feature = "serenity-cached")]
  async fn on_guild_create(&self, guild_count: usize, is_new: bool) {
    if is_new {
      let mut stats = self.stats.write().await;
      stats.set_server_count(guild_count);
    }
    self.try_post().await;
  }

  #[cfg(not(feature = "serenity-cached"))]
  async fn on_guild_create(&self, guild_id: GuildId) {
    {
      let mut cache = self.cache.lock().await;
      if cache.insert(guild_id) {
        let mut stats = self.stats.write().await;
        stats.set_server_count(cache.len());
      }
    }
    self.try_post().await;
  }

  #[cfg(feature = "serenity-cached")]
  async fn on_guild_delete(&self, guild_count: usize) {
    {
      let mut stats = self.stats.write().await;
      stats.set_server_count(guild_count);
    }
    self.try_post().await;
  }

  #[cfg(not(feature = "serenity-cached"))]
  async fn on_guild_delete(&self, guild_id: GuildId) {
    {
      let mut cache = self.cache.lock().await;
      if cache.remove(&guild_id) {
        let mut stats = self.stats.write().await;
        stats.set_server_count(cache.len());
      }
    }
    self.try_post().await;
  }
}

#[serenity::async_trait]
#[allow(unused_variables)]
impl EventHandler for Serenity {
  async fn ready(&self, ctx: Context, data_about_bot: Ready) {
    self.on_ready(&data_about_bot.guilds).await;
  }

  #[cfg(feature = "serenity-cached")]
  async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
    self.on_cache_ready(guilds.len()).await;
  }

  #[cfg(feature = "serenity-cached")]
  async fn shards_ready(&self, ctx: Context, total_shards: u32) {
    self.on_shards_ready(total_shards).await;
  }

  async fn guild_create(&self, ctx: Context, guild: Guild, is_new: Option<bool>) {
    cfg_if::cfg_if! {
      if #[cfg(feature = "serenity-cached")] {
        self.on_guild_create(ctx.cache.guilds().len(), is_new.unwrap_or(false)).await;
      } else {
        self.on_guild_create(guild.id).await;
      }
    }
  }

  async fn guild_delete(&self, ctx: Context, incomplete: UnavailableGuild, full: Option<Guild>) {
    cfg_if::cfg_if! {
      if #[cfg(feature = "serenity-cached")] {
        self.on_guild_delete(ctx.cache.guilds().len()).await;
      } else {
        self.on_guild_delete(incomplete.id).await;
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
