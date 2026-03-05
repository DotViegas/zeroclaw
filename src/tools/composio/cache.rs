//! Caching layer for Composio integration.
//!
//! Provides generic caching for tool schemas, tool lists, and OAuth connections
//! with TTL support and LRU eviction.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Connection information for OAuth-authenticated toolkits
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionInfo {
    pub toolkit: String,
    pub connected_account_id: String,
    pub status: ConnectionStatus,
    pub created_at: DateTime<Utc>,
}

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    Active,
    Expired,
    Revoked,
}

/// Generic cache entry with TTL support
#[derive(Debug, Clone)]
struct CachedEntry<T> {
    value: T,
    cached_at: DateTime<Utc>,
    ttl: chrono::Duration,
}

impl<T> CachedEntry<T> {
    fn is_expired(&self) -> bool {
        Utc::now() >= self.cached_at + self.ttl
    }
}

/// Connection cache with TTL support and LRU eviction
///
/// Caches OAuth connections per (toolkit, user_id) pair with configurable TTL
/// and automatic eviction when max entries per user is exceeded.
pub struct ConnectionCache {
    cache: Arc<RwLock<HashMap<String, CachedEntry<ConnectionInfo>>>>,
    max_entries_per_user: usize,
}

impl ConnectionCache {
    /// Create a new connection cache
    ///
    /// # Arguments
    /// * `max_entries_per_user` - Maximum number of cached connections per user_id
    pub fn new(max_entries_per_user: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_entries_per_user,
        }
    }

    /// Get a cached connection if valid
    ///
    /// Returns None if:
    /// - Connection not in cache
    /// - Connection expired (TTL exceeded)
    pub async fn get(&self, toolkit: &str, user_id: &str) -> Option<ConnectionInfo> {
        let cache = self.cache.read().await;
        let key = Self::cache_key(toolkit, user_id);

        if let Some(cached) = cache.get(&key) {
            if !cached.is_expired() {
                tracing::debug!(
                    toolkit = toolkit,
                    user_id = user_id,
                    "Connection cache hit"
                );
                return Some(cached.value.clone());
            } else {
                tracing::debug!(
                    toolkit = toolkit,
                    user_id = user_id,
                    "Connection cache expired"
                );
            }
        }

        None
    }

    /// Insert a connection into the cache
    ///
    /// Implements LRU eviction when max_entries_per_user is exceeded.
    pub async fn insert(
        &self,
        toolkit: &str,
        user_id: &str,
        info: ConnectionInfo,
        ttl: chrono::Duration,
    ) {
        let mut cache = self.cache.write().await;
        let key = Self::cache_key(toolkit, user_id);

        // Check if we need to evict entries for this user
        let user_entries: Vec<_> = cache
            .keys()
            .filter(|k| k.ends_with(&format!("::{}", user_id)))
            .cloned()
            .collect();

        if user_entries.len() >= self.max_entries_per_user {
            // LRU eviction: remove oldest entry for this user
            if let Some(oldest_key) = user_entries
                .iter()
                .filter_map(|k| cache.get(k).map(|v| (k, v.cached_at)))
                .min_by_key(|(_, cached_at)| *cached_at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
                tracing::debug!(
                    user_id = user_id,
                    evicted_key = %oldest_key,
                    "Evicted oldest connection cache entry"
                );
            }
        }

        cache.insert(
            key.clone(),
            CachedEntry {
                value: info,
                cached_at: Utc::now(),
                ttl,
            },
        );

        tracing::debug!(
            toolkit = toolkit,
            user_id = user_id,
            "Connection cached"
        );
    }

    /// Remove a connection from the cache
    pub async fn remove(&self, toolkit: &str, user_id: &str) {
        let mut cache = self.cache.write().await;
        let key = Self::cache_key(toolkit, user_id);
        cache.remove(&key);

        tracing::debug!(
            toolkit = toolkit,
            user_id = user_id,
            "Connection removed from cache"
        );
    }

    /// Clear all cached connections for a user
    pub async fn clear_user(&self, user_id: &str) {
        let mut cache = self.cache.write().await;
        let keys_to_remove: Vec<_> = cache
            .keys()
            .filter(|k| k.ends_with(&format!("::{}", user_id)))
            .cloned()
            .collect();

        for key in keys_to_remove {
            cache.remove(&key);
        }

        tracing::debug!(
            user_id = user_id,
            "Cleared all connections for user"
        );
    }

    /// Generate cache key from toolkit and user_id
    fn cache_key(toolkit: &str, user_id: &str) -> String {
        format!("{}::{}", toolkit, user_id)
    }
}

/// Tool schema cache with TTL support
///
/// Caches tool schemas to minimize MCP round-trips.
pub struct ToolSchemaCache {
    cache: Arc<RwLock<HashMap<String, CachedEntry<serde_json::Value>>>>,
}

impl ToolSchemaCache {
    /// Create a new tool schema cache
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a cached tool schema if valid
    pub async fn get(&self, tool_name: &str) -> Option<serde_json::Value> {
        let cache = self.cache.read().await;

        if let Some(cached) = cache.get(tool_name) {
            if !cached.is_expired() {
                tracing::debug!(tool_name = tool_name, "Tool schema cache hit");
                return Some(cached.value.clone());
            } else {
                tracing::debug!(tool_name = tool_name, "Tool schema cache expired");
            }
        }

        None
    }

    /// Insert a tool schema into the cache
    pub async fn insert(&self, tool_name: &str, schema: serde_json::Value, ttl: chrono::Duration) {
        let mut cache = self.cache.write().await;

        cache.insert(
            tool_name.to_string(),
            CachedEntry {
                value: schema,
                cached_at: Utc::now(),
                ttl,
            },
        );

        tracing::debug!(tool_name = tool_name, "Tool schema cached");
    }

    /// Clear all cached schemas
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        tracing::debug!("Cleared all tool schemas");
    }
}

impl Default for ToolSchemaCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool list cache with TTL support
///
/// Caches the list of available tools to minimize MCP calls.
pub struct ToolListCache {
    cache: Arc<RwLock<Option<CachedEntry<Vec<String>>>>>,
}

impl ToolListCache {
    /// Create a new tool list cache
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the cached tool list if valid
    pub async fn get(&self) -> Option<Vec<String>> {
        let cache = self.cache.read().await;

        if let Some(cached) = cache.as_ref() {
            if !cached.is_expired() {
                tracing::debug!("Tool list cache hit");
                return Some(cached.value.clone());
            } else {
                tracing::debug!("Tool list cache expired");
            }
        }

        None
    }

    /// Insert a tool list into the cache
    pub async fn insert(&self, tools: Vec<String>, ttl: chrono::Duration) {
        let mut cache = self.cache.write().await;

        *cache = Some(CachedEntry {
            value: tools,
            cached_at: Utc::now(),
            ttl,
        });

        tracing::debug!("Tool list cached");
    }

    /// Clear the cached tool list
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        *cache = None;
        tracing::debug!("Cleared tool list cache");
    }
}

impl Default for ToolListCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_cache_basic() {
        let cache = ConnectionCache::new(100);

        let info = ConnectionInfo {
            toolkit: "gmail".to_string(),
            connected_account_id: "acc_123".to_string(),
            status: ConnectionStatus::Active,
            created_at: Utc::now(),
        };

        // Insert and retrieve
        cache
            .insert("gmail", "user1", info.clone(), chrono::Duration::hours(1))
            .await;

        let cached = cache.get("gmail", "user1").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().connected_account_id, "acc_123");
    }

    #[tokio::test]
    async fn test_connection_cache_expiry() {
        let cache = ConnectionCache::new(100);

        let info = ConnectionInfo {
            toolkit: "gmail".to_string(),
            connected_account_id: "acc_123".to_string(),
            status: ConnectionStatus::Active,
            created_at: Utc::now(),
        };

        // Insert with very short TTL
        cache
            .insert(
                "gmail",
                "user1",
                info.clone(),
                chrono::Duration::milliseconds(1),
            )
            .await;

        // Wait for expiry
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let cached = cache.get("gmail", "user1").await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_connection_cache_eviction() {
        let cache = ConnectionCache::new(3);

        // Insert 4 connections for same user
        for i in 0..4 {
            let info = ConnectionInfo {
                toolkit: format!("toolkit_{}", i),
                connected_account_id: format!("acc_{}", i),
                status: ConnectionStatus::Active,
                created_at: Utc::now(),
            };

            cache
                .insert(&format!("toolkit_{}", i), "user1", info, chrono::Duration::hours(1))
                .await;

            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }

        // First entry should be evicted
        let first = cache.get("toolkit_0", "user1").await;
        assert!(first.is_none());

        // Last entry should still be there
        let last = cache.get("toolkit_3", "user1").await;
        assert!(last.is_some());
    }

    #[tokio::test]
    async fn test_tool_schema_cache() {
        let cache = ToolSchemaCache::new();

        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"}
            }
        });

        cache
            .insert("GMAIL_SEND_EMAIL", schema.clone(), chrono::Duration::hours(1))
            .await;

        let cached = cache.get("GMAIL_SEND_EMAIL").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), schema);
    }

    #[tokio::test]
    async fn test_tool_list_cache() {
        let cache = ToolListCache::new();

        let tools = vec![
            "GMAIL_SEND_EMAIL".to_string(),
            "SLACK_SEND_MESSAGE".to_string(),
        ];

        cache.insert(tools.clone(), chrono::Duration::hours(1)).await;

        let cached = cache.get().await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), tools);
    }
}
