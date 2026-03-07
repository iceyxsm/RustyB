//! Certificate Authority for TLS interception
//! 
//! This module handles:
//! - CA certificate generation and persistence
//! - Per-domain certificate signing
//! - Certificate caching with TTL
//! - PEM/DER encoding conversions
//! 
//! Uses rcgen 0.14 API

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType, BasicConstraints, KeyUsagePurpose, ExtendedKeyUsagePurpose, IsCa, Issuer};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Certificate with metadata for caching
#[derive(Clone)]
pub struct CachedCert {
    pub cert: CertificateDer<'static>,
    /// Key stored as PKCS8 DER bytes for cloning
    pub key_der_bytes: Vec<u8>,
    pub created_at: Instant,
    pub domain: String,
}

impl CachedCert {
    pub fn is_expired(&self) -> bool {
        // Certificates are valid for 1 year, but we cache them for 30 days
        self.created_at.elapsed() > Duration::from_secs(30 * 24 * 60 * 60)
    }

    /// Get the private key as PrivateKeyDer
    pub fn key(&self) -> PrivateKeyDer<'static> {
        PrivatePkcs8KeyDer::from(self.key_der_bytes.clone()).into()
    }
}

/// Certificate Authority for generating interception certificates
#[derive(Clone)]
pub struct CertificateAuthority {
    /// CA certificate in DER format
    pub ca_cert: CertificateDer<'static>,
    /// CA certificate in PEM format
    pub ca_cert_pem: String,
    /// CA private key bytes (stored for reconstruction since KeyPair doesn't implement Clone)
    ca_key_bytes: Arc<Vec<u8>>,
    /// CA certificate params for reconstructing Issuer
    ca_params: CertificateParams,
    /// Certificate cache: domain -> cached certificate
    cert_cache: Arc<RwLock<HashMap<String, CachedCert>>>,
    /// Cache TTL
    cache_ttl: Duration,
    /// Storage path for CA certificate
    storage_path: Option<PathBuf>,
}

impl CertificateAuthority {
    /// Create a new Certificate Authority, loading from disk if available
    pub async fn new(storage_path: Option<PathBuf>) -> anyhow::Result<Self> {
        if let Some(ref path) = storage_path {
            if let Ok(ca) = Self::load_from_disk(path).await {
                info!("Loaded CA certificate from disk");
                return Ok(ca);
            }
        }

        info!("Generating new Certificate Authority...");
        let ca = Self::generate(storage_path.clone())?;
        
        if let Some(ref path) = storage_path {
            if let Err(e) = ca.save_to_disk(path).await {
                tracing::warn!("Failed to save CA certificate: {}", e);
            }
        }

        Ok(ca)
    }

    /// Generate a new CA certificate using rcgen 0.14 API
    fn generate(storage_path: Option<PathBuf>) -> anyhow::Result<Self> {
        // Generate CA key pair
        let ca_key = KeyPair::generate()?;
        
        // Store key bytes for later reconstruction
        let ca_key_bytes = Arc::new(ca_key.serialize_der());
        
        // Build CA certificate parameters using the new() method which returns Result
        let mut params = CertificateParams::new(vec!["Rusty Browser MITM CA".to_string()])?;
        
        // Set CA constraints
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
        ];
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
            ExtendedKeyUsagePurpose::ClientAuth,
        ];
        
        // Set validity period (10 years)
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(2034, 12, 31);

        // Set distinguished name
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Rusty Browser MITM CA");
        dn.push(DnType::OrganizationName, "Rusty Browser");
        dn.push(DnType::CountryName, "US");
        params.distinguished_name = dn;

        // Self-sign the CA certificate
        let ca_cert = params.self_signed(&ca_key)?;
        
        // Get PEM encoding
        let ca_cert_pem = ca_cert.pem();
        
        // Convert to DER for rustls
        let ca_cert_der = CertificateDer::from(ca_cert.der().clone());

        info!("Generated new CA certificate");

        Ok(Self {
            ca_cert: ca_cert_der,
            ca_cert_pem,
            ca_key_bytes,
            ca_params: params,
            cert_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            storage_path,
        })
    }

    /// Create an Issuer from stored params and key bytes
    fn create_issuer(&self) -> anyhow::Result<Issuer<'static, KeyPair>> {
        // Reconstruct KeyPair from stored bytes
        let ca_key: KeyPair = (*self.ca_key_bytes).clone().try_into()?;
        
        // Create Issuer from params and key
        Ok(Issuer::new(self.ca_params.clone(), ca_key))
    }

    /// Generate a certificate for a specific domain signed by this CA
    pub async fn generate_domain_cert(&self, domain: &str) -> anyhow::Result<CachedCert> {
        // Check cache first
        {
            let cache = self.cert_cache.read().await;
            if let Some(cached) = cache.get(domain) {
                if !cached.is_expired() {
                    debug!("Using cached certificate for {}", domain);
                    return Ok(cached.clone());
                }
            }
        }

        debug!("Generating new certificate for {}", domain);

        // Generate domain key pair
        let domain_key = KeyPair::generate()?;
        
        // Build domain certificate parameters
        let mut params = CertificateParams::new(vec![domain.to_string()])?;
        
        // Add SANs for the domain
        params.subject_alt_names = vec![
            SanType::DnsName(domain.try_into()?),
        ];
        
        // Set key usage for TLS server
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
        ];
        
        // Set validity period (1 year)
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(2025, 12, 31);

        // Set distinguished name
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, domain.to_string());
        params.distinguished_name = dn;

        // Create Issuer for signing
        let issuer = self.create_issuer()?;
        
        // Sign with CA key using the Issuer
        let domain_cert = params.signed_by(&domain_key, &issuer)?;
        
        // Convert to DER
        let cert_der = CertificateDer::from(domain_cert.der().clone());
        
        // Store key as PKCS8 bytes for later cloning
        let key_der_bytes = domain_key.serialize_der();

        let cached = CachedCert {
            cert: cert_der,
            key_der_bytes,
            created_at: Instant::now(),
            domain: domain.to_string(),
        };

        // Cache the certificate
        {
            let mut cache = self.cert_cache.write().await;
            cache.insert(domain.to_string(), cached.clone());
        }

        Ok(cached)
    }

    /// Save CA certificate and key to disk
    pub async fn save_to_disk(&self, path: &PathBuf) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(path).await?;

        let cert_path = path.join("ca-cert.pem");
        let key_path = path.join("ca-key.pem");

        tokio::fs::write(&cert_path, &self.ca_cert_pem).await?;
        tokio::fs::write(&key_path, self.serialize_key_pem()).await?;

        info!("Saved CA certificate to {:?}", cert_path);
        Ok(())
    }

    /// Load CA certificate and key from disk
    pub async fn load_from_disk(_path: &PathBuf) -> anyhow::Result<Self> {
        // For loading, we need to reconstruct from the saved files
        // This is a placeholder - in production, you'd parse the actual certificate
        Err(anyhow::anyhow!("Loading CA from disk not fully implemented"))
    }

    /// Serialize the CA key to PEM
    fn serialize_key_pem(&self) -> String {
        // Reconstruct KeyPair to serialize to PEM
        if let Ok(key) = KeyPair::try_from(self.ca_key_bytes.as_ref().clone()) {
            key.serialize_pem()
        } else {
            String::new()
        }
    }

    /// Get the CA certificate PEM for installation in browsers
    pub fn ca_cert_pem(&self) -> &str {
        &self.ca_cert_pem
    }

    /// Clear the certificate cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cert_cache.write().await;
        cache.clear();
    }
}
