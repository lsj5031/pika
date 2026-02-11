use axum::http::HeaderMap;
use ipnet::IpNet;
use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const DEFAULT_MAX_KEYS: usize = 10_000;
const DEFAULT_COMPACTION_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct RateLimitDecision {
    pub allowed: bool,
    pub retry_after_seconds: u64,
}

#[derive(Debug, Clone)]
struct RateLimitEntry {
    timestamps: VecDeque<Instant>,
    last_seen: Instant,
}

#[derive(Debug)]
struct RateLimitStore {
    entries: HashMap<String, RateLimitEntry>,
    last_compaction: Instant,
}

#[derive(Clone)]
pub struct FixedWindowRateLimiter {
    limit: usize,
    window: Duration,
    max_keys: usize,
    compaction_interval: Duration,
    store: Arc<Mutex<RateLimitStore>>,
}

impl FixedWindowRateLimiter {
    pub fn new(limit: usize, window: Duration) -> Self {
        Self {
            limit,
            window,
            max_keys: DEFAULT_MAX_KEYS,
            compaction_interval: DEFAULT_COMPACTION_INTERVAL,
            store: Arc::new(Mutex::new(RateLimitStore {
                entries: HashMap::new(),
                last_compaction: Instant::now(),
            })),
        }
    }

    #[cfg(test)]
    fn with_limits(
        limit: usize,
        window: Duration,
        max_keys: usize,
        compaction_interval: Duration,
    ) -> Self {
        Self {
            limit,
            window,
            max_keys,
            compaction_interval,
            store: Arc::new(Mutex::new(RateLimitStore {
                entries: HashMap::new(),
                last_compaction: Instant::now(),
            })),
        }
    }

    pub async fn check(&self, key: &str) -> RateLimitDecision {
        if self.limit == 0 {
            return RateLimitDecision {
                allowed: false,
                retry_after_seconds: self.window.as_secs().max(1),
            };
        }

        let now = Instant::now();
        let mut guard = self.store.lock().await;

        if now.duration_since(guard.last_compaction) >= self.compaction_interval {
            self.compact_locked(&mut guard, now);
            guard.last_compaction = now;
        }

        let should_remove_existing = if let Some(existing) = guard.entries.get_mut(key) {
            prune_old_timestamps(&mut existing.timestamps, now, self.window);
            existing.timestamps.is_empty()
        } else {
            false
        };
        if should_remove_existing {
            guard.entries.remove(key);
        }

        if !guard.entries.contains_key(key) && guard.entries.len() >= self.max_keys {
            self.evict_lru_locked(&mut guard);
        }

        let queue_len;
        let retry_after_seconds;
        {
            let entry = guard
                .entries
                .entry(key.to_string())
                .or_insert_with(|| RateLimitEntry {
                    timestamps: VecDeque::new(),
                    last_seen: now,
                });

            entry.last_seen = now;
            prune_old_timestamps(&mut entry.timestamps, now, self.window);
            queue_len = entry.timestamps.len();

            retry_after_seconds = entry
                .timestamps
                .front()
                .map(|first| {
                    self.window
                        .saturating_sub(now.saturating_duration_since(*first))
                        .as_secs()
                        .max(1)
                })
                .unwrap_or(1);

            if queue_len < self.limit {
                entry.timestamps.push_back(now);
            }
        }

        if queue_len >= self.limit {
            return RateLimitDecision {
                allowed: false,
                retry_after_seconds,
            };
        }

        RateLimitDecision {
            allowed: true,
            retry_after_seconds: 0,
        }
    }

    fn compact_locked(&self, store: &mut RateLimitStore, now: Instant) {
        store.entries.retain(|_, entry| {
            prune_old_timestamps(&mut entry.timestamps, now, self.window);
            !entry.timestamps.is_empty()
        });

        while store.entries.len() > self.max_keys {
            self.evict_lru_locked(store);
        }
    }

    fn evict_lru_locked(&self, store: &mut RateLimitStore) {
        if let Some(oldest_key) = store
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_seen)
            .map(|(key, _)| key.clone())
        {
            store.entries.remove(&oldest_key);
        }
    }
}

fn prune_old_timestamps(queue: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while let Some(oldest) = queue.front() {
        if now.duration_since(*oldest) > window {
            queue.pop_front();
        } else {
            break;
        }
    }
}

#[derive(Clone)]
pub struct RateLimitState {
    pub login: FixedWindowRateLimiter,
    pub websocket_connect: FixedWindowRateLimiter,
}

impl RateLimitState {
    pub fn new(login_limit_per_minute: u32, websocket_limit_per_minute: u32) -> Self {
        Self {
            login: FixedWindowRateLimiter::new(
                login_limit_per_minute as usize,
                Duration::from_secs(60),
            ),
            websocket_connect: FixedWindowRateLimiter::new(
                websocket_limit_per_minute as usize,
                Duration::from_secs(60),
            ),
        }
    }
}

pub fn parse_trusted_proxy_cidrs(cidrs: &[String]) -> Vec<IpNet> {
    cidrs
        .iter()
        .filter_map(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return None;
            }

            if let Ok(net) = IpNet::from_str(trimmed) {
                return Some(net);
            }

            if let Ok(ip) = IpAddr::from_str(trimmed) {
                let prefix = match ip {
                    IpAddr::V4(_) => 32,
                    IpAddr::V6(_) => 128,
                };
                return IpNet::new(ip, prefix).ok();
            }

            None
        })
        .collect()
}

/// Resolve the effective client IP for rate limiting.
///
/// The direct peer IP is always used unless that peer belongs to a configured trusted proxy CIDR,
/// in which case forwarded headers are honored.
pub fn extract_client_ip(
    headers: &HeaderMap,
    peer_addr: SocketAddr,
    trusted_proxy_cidrs: &[IpNet],
) -> IpAddr {
    let peer_ip = peer_addr.ip();

    if !trusted_proxy_cidrs
        .iter()
        .any(|cidr| cidr.contains(&peer_ip))
    {
        return peer_ip;
    }

    if let Some(forwarded_for) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok())
        && let Some(first) = forwarded_for.split(',').next()
        && let Ok(client_ip) = IpAddr::from_str(first.trim())
    {
        return client_ip;
    }

    if let Some(real_ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok())
        && let Ok(client_ip) = IpAddr::from_str(real_ip.trim())
    {
        return client_ip;
    }

    peer_ip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn evicts_oldest_key_when_capacity_is_reached() {
        let limiter = FixedWindowRateLimiter::with_limits(
            5,
            Duration::from_secs(60),
            2,
            Duration::from_secs(60),
        );

        assert!(limiter.check("a").await.allowed);
        assert!(limiter.check("b").await.allowed);
        assert!(limiter.check("c").await.allowed);

        let guard = limiter.store.lock().await;
        assert_eq!(guard.entries.len(), 2);
        assert!(!guard.entries.contains_key("a"));
    }

    #[test]
    fn forwarded_headers_are_only_used_for_trusted_proxies() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "198.51.100.77".parse().unwrap());

        let trusted = parse_trusted_proxy_cidrs(&["10.0.0.0/8".to_string()]);

        let untrusted_peer = SocketAddr::from(([203, 0, 113, 9], 12345));
        let trusted_peer = SocketAddr::from(([10, 1, 2, 3], 12345));

        assert_eq!(
            extract_client_ip(&headers, untrusted_peer, &trusted),
            IpAddr::from([203, 0, 113, 9])
        );
        assert_eq!(
            extract_client_ip(&headers, trusted_peer, &trusted),
            IpAddr::from([198, 51, 100, 77])
        );
    }
}
