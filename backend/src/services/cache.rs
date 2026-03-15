//! Cache service with TTL support

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

impl<T> CacheEntry<T> {
    fn new(value: T, ttl_seconds: u64) -> Self {
        Self {
            value,
            expires_at: Instant::now() + Duration::from_secs(ttl_seconds),
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

#[derive(Clone)]
pub struct CacheService {
    data: Arc<DashMap<String, CacheEntry<Vec<u8>>>>,
    default_ttl: u64,
}

impl CacheService {
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
            default_ttl: 300, // 5 minutes
        }
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        if let Some(entry) = self.data.get(key) {
            if !entry.is_expired() {
                return serde_json::from_slice(&entry.value).ok();
            }
            drop(entry);
            self.data.remove(key);
        }
        None
    }

    pub fn set<T: serde::Serialize>(&self, key: &str, value: &T, ttl_seconds: u64) {
        if let Ok(bytes) = serde_json::to_vec(value) {
            let entry = CacheEntry::new(bytes, ttl_seconds);
            self.data.insert(key.to_string(), entry);
        }
    }

    pub fn delete(&self, key: &str) {
        self.data.remove(key);
    }

    pub fn clear(&self) {
        self.data.clear();
    }
}

impl Default for CacheService {
    fn default() -> Self {
        Self::new()
    }
}
