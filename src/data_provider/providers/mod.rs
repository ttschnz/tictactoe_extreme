mod cache_provider;
mod redis_provider;

pub use cache_provider::{CacheProvider, CacheProviderArgs};
pub use redis_provider::{RedisProvider, RedisProviderArgs};

pub enum Provider {
    Redis(RedisProvider),
    Cache(CacheProvider),
}
