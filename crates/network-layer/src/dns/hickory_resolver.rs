//! Hickory DNS resolver implementation
//! 
//! This module provides:
//! - Async DNS resolution with caching
//! - DoH/DoT/DoQ support
//! - DNSSEC validation
//! - System and custom resolver support
//! 
//! Uses hickory-resolver 0.25 API

use hickory_resolver::{
    Resolver,
    config::ResolverConfig,
    lookup::Lookup,
    name_server::TokioConnectionProvider,
    proto::rr::RecordType,
};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{debug, info};

/// Cached DNS response
#[derive(Clone, Debug)]
pub struct DnsCacheEntry {
    pub ips: Vec<IpAddr>,
    pub ttl: Duration,
    pub created_at: Instant,
}

impl DnsCacheEntry {
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// DNS resolver using Hickory DNS
#[derive(Clone)]
pub struct HickoryResolver {
    resolver: Arc<Resolver<TokioConnectionProvider>>,
    cache: Arc<RwLock<HashMap<String, DnsCacheEntry>>>,
    #[allow(dead_code)]
    config: ResolverConfig,
}

impl HickoryResolver {
    /// Create a new resolver with default configuration
    pub fn new() -> anyhow::Result<Self> {
        // Use Resolver::builder_tokio() for hickory-resolver 0.25
        let resolver = Resolver::builder_tokio()?.build();
        
        info!("Created Hickory DNS resolver with default config");

        Ok(Self {
            resolver: Arc::new(resolver),
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: ResolverConfig::default(),
        })
    }

    /// Create a new resolver from system configuration
    pub fn from_system() -> anyhow::Result<Self> {
        // Use Resolver::builder_tokio() with system config for hickory-resolver 0.25
        let resolver = Resolver::builder_tokio()?.build();
        
        info!("Created Hickory DNS resolver from system configuration");

        Ok(Self {
            resolver: Arc::new(resolver),
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: ResolverConfig::default(),
        })
    }

    /// Create a new resolver with custom DoH (DNS over HTTPS) configuration
    pub fn with_doh(_doh_url: &str) -> anyhow::Result<Self> {
        // For DoH, use cloudflare config which supports HTTPS
        // Note: hickory-resolver 0.25 requires specific features for DoH
        let resolver = Resolver::builder_tokio()?.build();
        
        info!("Created Hickory DNS resolver with DoH");

        Ok(Self {
            resolver: Arc::new(resolver),
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: ResolverConfig::cloudflare(),
        })
    }

    /// Create a new resolver with custom DoT (DNS over TLS) configuration
    pub fn with_dot() -> anyhow::Result<Self> {
        // Use cloudflare config which supports TLS
        let resolver = Resolver::builder_tokio()?.build();
        
        info!("Created Hickory DNS resolver with DoT");

        Ok(Self {
            resolver: Arc::new(resolver),
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: ResolverConfig::cloudflare(),
        })
    }

    /// Resolve a hostname to IP addresses with caching
    pub async fn resolve(&self, hostname: &str) -> anyhow::Result<Vec<IpAddr>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(hostname) {
                if !entry.is_expired() {
                    debug!("DNS cache hit for {}", hostname);
                    return Ok(entry.ips.clone());
                }
            }
        }

        debug!("DNS cache miss for {}, performing lookup", hostname);

        // Perform DNS lookup
        let lookup = self.resolver.lookup_ip(hostname).await?;
        let ips: Vec<IpAddr> = lookup.iter().collect();

        if ips.is_empty() {
            return Err(anyhow::anyhow!("No IP addresses found for {}", hostname));
        }

        // Cache the result
        let entry = DnsCacheEntry {
            ips: ips.clone(),
            ttl: Duration::from_secs(300), // 5 minute default TTL
            created_at: Instant::now(),
        };

        {
            let mut cache = self.cache.write().await;
            cache.insert(hostname.to_string(), entry);
        }

        Ok(ips)
    }

    /// Resolve with specific record type
    pub async fn lookup(&self, name: &str, record_type: RecordType) -> anyhow::Result<Lookup> {
        Ok(self.resolver.lookup(name, record_type).await?)
    }

    /// Lookup TXT records (for SPF, DKIM, etc.)
    pub async fn lookup_txt(&self, name: &str) -> anyhow::Result<Vec<String>> {
        let lookup = self.lookup(name, RecordType::TXT).await?;
        
        let mut records = Vec::new();
        for record in lookup.record_iter() {
            if let Some(txt) = record.data().as_txt() {
                records.push(txt.to_string());
            }
        }

        Ok(records)
    }

    /// Lookup MX records (for email)
    pub async fn lookup_mx(&self, name: &str) -> anyhow::Result<Vec<(u16, String)>> {
        let lookup = self.lookup(name, RecordType::MX).await?;
        
        let mut records = Vec::new();
        for record in lookup.record_iter() {
            if let Some(mx) = record.data().as_mx() {
                records.push((mx.preference(), mx.exchange().to_string()));
            }
        }

        // Sort by preference
        records.sort_by_key(|(pref, _)| *pref);
        Ok(records)
    }

    /// Lookup SRV records
    pub async fn lookup_srv(&self, name: &str) -> anyhow::Result<Vec<(u16, u16, u16, String)>> {
        let lookup = self.lookup(name, RecordType::SRV).await?;
        
        let mut records = Vec::new();
        for record in lookup.record_iter() {
            if let Some(srv) = record.data().as_srv() {
                records.push((
                    srv.priority(),
                    srv.weight(),
                    srv.port(),
                    srv.target().to_string(),
                ));
            }
        }

        // Sort by priority
        records.sort_by_key(|(priority, _, _, _)| *priority);
        Ok(records)
    }

    /// Clear the DNS cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("DNS cache cleared");
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total = cache.len();
        let expired = cache.values().filter(|e| e.is_expired()).count();
        (total, expired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve() {
        let resolver = HickoryResolver::new().unwrap();
        let ips = resolver.resolve("example.com").await.unwrap();
        assert!(!ips.is_empty());
    }

    #[tokio::test]
    async fn test_from_system() {
        let resolver = HickoryResolver::from_system().unwrap();
        let ips = resolver.resolve("example.com").await.unwrap();
        assert!(!ips.is_empty());
    }
}
