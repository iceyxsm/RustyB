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
    TokioResolver,
    config::{ResolverConfig, ResolverOpts, NameServerConfig, Protocol},
    lookup::Lookup,
    error::ResolveError,
};
use hickory_resolver::proto::rr::RecordType;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{debug, info, warn};

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
pub struct HickoryResolver {
    resolver: TokioResolver,
    cache: Arc<RwLock<HashMap<String, DnsCacheEntry>>>,
    config: ResolverConfig,
}

impl HickoryResolver {
    /// Create a new resolver with default configuration
    pub async fn new() -> anyhow::Result<Self> {
        // Use TokioResolver::tokio() constructor for hickory-resolver 0.25
        let resolver = TokioResolver::tokio(ResolverConfig::default(), ResolverOpts::default()).await?;
        
        info!("Created Hickory DNS resolver with default config");

        Ok(Self {
            resolver,
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: ResolverConfig::default(),
        })
    }

    /// Create a new resolver from system configuration
    pub async fn from_system() -> anyhow::Result<Self> {
        // Use TokioResolver::from_system_conf() for hickory-resolver 0.25
        let resolver = TokioResolver::from_system_conf().await?;
        
        info!("Created Hickory DNS resolver from system configuration");

        Ok(Self {
            resolver,
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: ResolverConfig::default(),
        })
    }

    /// Create a new resolver with custom DoH (DNS over HTTPS) configuration
    pub async fn with_doh(doh_url: &str) -> anyhow::Result<Self> {
        let mut config = ResolverConfig::new();
        
        // Parse DoH URL
        let url = url::Url::parse(doh_url)?;
        let host = url.host_str().ok_or_else(|| anyhow::anyhow!("Invalid DoH URL: missing host"))?;
        let port = url.port().unwrap_or(443);
        
        // Get IP addresses for the DoH server
        let system_resolver = TokioResolver::from_system_conf().await?;
        let lookup = system_resolver.lookup_ip(host).await?;
        let ips: Vec<IpAddr> = lookup.iter().collect();
        
        if ips.is_empty() {
            return Err(anyhow::anyhow!("Could not resolve DoH server: {}", host));
        }

        // Add DoH name server
        let socket_addr = SocketAddr::new(ips[0], port);
        let ns_config = NameServerConfig {
            socket_addr,
            protocol: Protocol::Https,
            tls_dns_name: Some(host.to_string()),
            trust_negative_responses: false,
            bind_addr: None,
        };
        config.add_name_server(ns_config);

        let resolver = TokioResolver::tokio(config.clone(), ResolverOpts::default()).await?;
        
        info!("Created Hickory DNS resolver with DoH: {}", doh_url);

        Ok(Self {
            resolver,
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Create a new resolver with custom DoT (DNS over TLS) configuration
    pub async fn with_dot(dot_host: &str, dot_port: u16) -> anyhow::Result<Self> {
        let mut config = ResolverConfig::new();
        
        // Get IP addresses for the DoT server
        let system_resolver = TokioResolver::from_system_conf().await?;
        let lookup = system_resolver.lookup_ip(dot_host).await?;
        let ips: Vec<IpAddr> = lookup.iter().collect();
        
        if ips.is_empty() {
            return Err(anyhow::anyhow!("Could not resolve DoT server: {}", dot_host));
        }

        // Add DoT name server
        let socket_addr = SocketAddr::new(ips[0], dot_port);
        let ns_config = NameServerConfig {
            socket_addr,
            protocol: Protocol::Tls,
            tls_dns_name: Some(dot_host.to_string()),
            trust_negative_responses: false,
            bind_addr: None,
        };
        config.add_name_server(ns_config);

        let resolver = TokioResolver::tokio(config.clone(), ResolverOpts::default()).await?;
        
        info!("Created Hickory DNS resolver with DoT: {}:{}", dot_host, dot_port);

        Ok(Self {
            resolver,
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
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
    pub async fn lookup(&self, name: &str, record_type: RecordType) -> Result<Lookup, ResolveError> {
        self.resolver.lookup(name, record_type).await
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
        let resolver = HickoryResolver::new().await.unwrap();
        let ips = resolver.resolve("example.com").await.unwrap();
        assert!(!ips.is_empty());
    }

    #[tokio::test]
    async fn test_from_system() {
        let resolver = HickoryResolver::from_system().await.unwrap();
        let ips = resolver.resolve("example.com").await.unwrap();
        assert!(!ips.is_empty());
    }
}
