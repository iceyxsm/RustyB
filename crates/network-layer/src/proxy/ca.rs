//! Certificate Authority for TLS interception
//! 
//! This module handles:
//! - CA certificate generation and persistence
//! - Per-domain certificate signing
//! - Certificate caching with TTL
//! - PEM/DER encoding conversions
//! 
//! Uses rcgen 0.14 API with x509-parser support for loading existing CA certificates

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType, BasicConstraints as RcgenBasicConstraints, KeyUsagePurpose, ExtendedKeyUsagePurpose, IsCa, Issuer};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Default CA certificate validity period (10 years)
const CA_VALIDITY_YEARS: i32 = 10;
/// Default domain certificate validity period (1 year)
const DOMAIN_VALIDITY_YEARS: i32 = 1;
/// Certificate cache TTL (30 days)
const CACHE_TTL_DAYS: u64 = 30;

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
    /// Check if the cached certificate has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > Duration::from_secs(CACHE_TTL_DAYS * 24 * 60 * 60)
    }

    /// Get the private key as PrivateKeyDer
    pub fn key(&self) -> PrivateKeyDer<'static> {
        PrivatePkcs8KeyDer::from(self.key_der_bytes.clone()).into()
    }
}

/// Certificate Authority for generating interception certificates
/// 
/// This struct manages the CA certificate and key pair, and provides
/// methods to generate and cache per-domain certificates for TLS interception.
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
    /// Storage path for CA certificate
    storage_path: Option<PathBuf>,
}

impl CertificateAuthority {
    /// Create a new Certificate Authority, loading from disk if available
    /// 
    /// If a CA certificate exists at the storage path, it will be loaded.
    /// Otherwise, a new CA will be generated and saved to disk.
    pub async fn new(storage_path: Option<PathBuf>) -> anyhow::Result<Self> {
        if let Some(ref path) = storage_path {
            match Self::load_from_disk(path).await {
                Ok(ca) => {
                    info!("Loaded CA certificate from disk at {:?}", path);
                    return Ok(ca);
                }
                Err(e) => {
                    warn!("Failed to load CA from disk: {}. Generating new CA...", e);
                }
            }
        }

        info!("Generating new Certificate Authority...");
        let ca = Self::generate(storage_path.clone())?;
        
        if let Some(ref path) = storage_path {
            if let Err(e) = ca.save_to_disk(path).await {
                warn!("Failed to save CA certificate: {}", e);
            }
        }

        Ok(ca)
    }

    /// Generate a new CA certificate using rcgen 0.14 API
    fn generate(storage_path: Option<PathBuf>) -> anyhow::Result<Self> {
        let ca_key = KeyPair::generate()?;
        let ca_key_bytes = Arc::new(ca_key.serialize_der());
        
        let params = Self::create_ca_params()?;
        let ca_cert = params.self_signed(&ca_key)?;
        let ca_cert_pem = ca_cert.pem();
        let ca_cert_der = CertificateDer::from(ca_cert.der().clone());

        info!("Generated new CA certificate with 10-year validity");

        Ok(Self {
            ca_cert: ca_cert_der,
            ca_cert_pem,
            ca_key_bytes,
            ca_params: params,
            cert_cache: Arc::new(RwLock::new(HashMap::new())),
            storage_path,
        })
    }

    /// Create CA certificate parameters
    fn create_ca_params() -> anyhow::Result<CertificateParams> {
        let mut params = CertificateParams::new(vec!["Rusty Browser MITM CA".to_string()])?;
        
        params.is_ca = IsCa::Ca(RcgenBasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
        ];
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
            ExtendedKeyUsagePurpose::ClientAuth,
        ];
        
        // Use rcgen's date_time_ymd for proper certificate dates
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(2034, 12, 31);

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Rusty Browser MITM CA");
        dn.push(DnType::OrganizationName, "Rusty Browser");
        dn.push(DnType::CountryName, "US");
        params.distinguished_name = dn;

        Ok(params)
    }

    /// Load CA certificate and key from disk
    /// 
    /// Parses the existing CA certificate using x509-parser to extract
    /// the distinguished name, key usages, and other parameters needed
    /// to reconstruct the Issuer for signing domain certificates.
    async fn load_from_disk(path: &PathBuf) -> anyhow::Result<Self> {
        let cert_path = path.join("ca-cert.pem");
        let key_path = path.join("ca-key.pem");

        if !cert_path.exists() || !key_path.exists() {
            return Err(anyhow::anyhow!("CA certificate or key file not found"));
        }

        // Read certificate and key files
        let cert_pem = tokio::fs::read_to_string(&cert_path).await
            .map_err(|e| anyhow::anyhow!("Failed to read CA certificate: {}", e))?;
        let key_pem = tokio::fs::read_to_string(&key_path).await
            .map_err(|e| anyhow::anyhow!("Failed to read CA key: {}", e))?;

        // Parse the key pair from PEM
        let ca_key = KeyPair::from_pem(&key_pem)
            .map_err(|e| anyhow::anyhow!("Failed to parse CA key: {}", e))?;
        let ca_key_bytes = Arc::new(ca_key.serialize_der());

        // Parse the certificate to extract params using x509-parser
        let ca_params = Self::parse_ca_params_from_pem(&cert_pem)?;
        
        info!("Successfully loaded CA certificate from disk");

        Ok(Self {
            ca_cert: Self::parse_cert_pem_to_der(&cert_pem)?,
            ca_cert_pem: cert_pem,
            ca_key_bytes,
            ca_params,
            cert_cache: Arc::new(RwLock::new(HashMap::new())),
            storage_path: Some(path.clone()),
        })
    }

    /// Parse CA certificate PEM and extract CertificateParams using x509-parser
    fn parse_ca_params_from_pem(cert_pem: &str) -> anyhow::Result<CertificateParams> {
        use x509_parser::pem::parse_x509_pem;
        use x509_parser::prelude::*;

        let pem = parse_x509_pem(cert_pem.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to parse PEM: {:?}", e))?
            .1;
        
        let x509 = pem.parse_x509()
            .map_err(|e| anyhow::anyhow!("Failed to parse X509: {:?}", e))?;

        // Extract subject name for SANs
        let subject = x509.subject().to_string();
        let subject_alt_names = vec![subject.clone()];

        let mut params = CertificateParams::new(subject_alt_names)?;

        // Extract distinguished name from subject
        let mut dn = DistinguishedName::new();
        for rdn in x509.subject().iter() {
            for attr in rdn.iter() {
                let attr_str = attr.as_str()
                    .map_err(|_| anyhow::anyhow!("Non-UTF8 attribute value"))?;
                
                // Map OID to DnType
                let oid: Vec<u64> = attr.attr_type().iter().ok_or_else(|| anyhow::anyhow!("Invalid OID"))?.collect();
                let dn_type = match oid.as_slice() {
                    [2, 5, 4, 6] => DnType::CountryName,
                    [2, 5, 4, 10] => DnType::OrganizationName,
                    [2, 5, 4, 11] => DnType::OrganizationalUnitName,
                    [2, 5, 4, 3] => DnType::CommonName,
                    [2, 5, 4, 7] => DnType::LocalityName,
                    [2, 5, 4, 8] => DnType::StateOrProvinceName,
                    _ => continue, // Skip unknown OIDs
                };
                dn.push(dn_type, attr_str);
            }
        }
        params.distinguished_name = dn;

        // Extract key usages
        if let Ok(Some(key_usage_ext)) = x509.key_usage() {
            let mut key_usages = Vec::new();
            let ku = &key_usage_ext.value;
            if ku.digital_signature() { key_usages.push(KeyUsagePurpose::DigitalSignature); }
            if ku.non_repudiation() { key_usages.push(KeyUsagePurpose::ContentCommitment); }
            if ku.key_encipherment() { key_usages.push(KeyUsagePurpose::KeyEncipherment); }
            if ku.data_encipherment() { key_usages.push(KeyUsagePurpose::DataEncipherment); }
            if ku.key_agreement() { key_usages.push(KeyUsagePurpose::KeyAgreement); }
            if ku.key_cert_sign() { key_usages.push(KeyUsagePurpose::KeyCertSign); }
            if ku.crl_sign() { key_usages.push(KeyUsagePurpose::CrlSign); }
            if ku.encipher_only() { key_usages.push(KeyUsagePurpose::EncipherOnly); }
            if ku.decipher_only() { key_usages.push(KeyUsagePurpose::DecipherOnly); }
            params.key_usages = key_usages;
        }

        // Set CA constraints
        params.is_ca = IsCa::Ca(RcgenBasicConstraints::Unconstrained);
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
            ExtendedKeyUsagePurpose::ClientAuth,
        ];

        // Set validity period
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(2034, 12, 31);

        Ok(params)
    }

    /// Parse certificate PEM to DER format
    fn parse_cert_pem_to_der(cert_pem: &str) -> anyhow::Result<CertificateDer<'static>> {
        use x509_parser::pem::parse_x509_pem;
        
        let pem = parse_x509_pem(cert_pem.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to parse PEM: {:?}", e))?
            .1;
        
        Ok(CertificateDer::from(pem.contents.to_vec()))
    }

    /// Create an Issuer from stored params and key bytes
    fn create_issuer(&self) -> anyhow::Result<Issuer<'static, KeyPair>> {
        let ca_key: KeyPair = (*self.ca_key_bytes).clone().try_into()
            .map_err(|e| anyhow::anyhow!("Failed to reconstruct CA key: {:?}", e))?;
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

        let domain_key = KeyPair::generate()?;
        let mut params = CertificateParams::new(vec![domain.to_string()])?;
        
        params.subject_alt_names = vec![
            SanType::DnsName(domain.try_into()?),
        ];
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
        ];
        
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(2025, 12, 31);

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, domain.to_string());
        params.distinguished_name = dn;

        let issuer = self.create_issuer()?;
        let domain_cert = params.signed_by(&domain_key, &issuer)?;
        
        let cert_der = CertificateDer::from(domain_cert.der().clone());
        let key_der_bytes = domain_key.serialize_der();

        let cached = CachedCert {
            cert: cert_der,
            key_der_bytes,
            created_at: Instant::now(),
            domain: domain.to_string(),
        };

        {
            let mut cache = self.cert_cache.write().await;
            cache.insert(domain.to_string(), cached.clone());
        }

        Ok(cached)
    }

    /// Save CA certificate and key to disk
    pub async fn save_to_disk(&self, path: &PathBuf) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(path).await
            .map_err(|e| anyhow::anyhow!("Failed to create CA directory: {}", e))?;

        let cert_path = path.join("ca-cert.pem");
        let key_path = path.join("ca-key.pem");

        tokio::fs::write(&cert_path, &self.ca_cert_pem).await
            .map_err(|e| anyhow::anyhow!("Failed to write CA certificate: {}", e))?;
        tokio::fs::write(&key_path, self.serialize_key_pem()).await
            .map_err(|e| anyhow::anyhow!("Failed to write CA key: {}", e))?;

        info!("Saved CA certificate to {:?}", cert_path);
        Ok(())
    }

    /// Serialize the CA key to PEM
    fn serialize_key_pem(&self) -> String {
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
        let count = cache.len();
        cache.clear();
        info!("Cleared {} cached certificates", count);
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cert_cache.read().await;
        let total = cache.len();
        let expired = cache.values().filter(|e| e.is_expired()).count();
        (total, expired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ca_generation() {
        let ca = CertificateAuthority::new(None).await.unwrap();
        assert!(!ca.ca_cert_pem.is_empty());
    }

    #[tokio::test]
    async fn test_domain_cert_generation() {
        let ca = CertificateAuthority::new(None).await.unwrap();
        let cert = ca.generate_domain_cert("example.com").await.unwrap();
        assert_eq!(cert.domain, "example.com");
        assert!(!cert.key_der_bytes.is_empty());
    }

    #[tokio::test]
    async fn test_ca_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Generate and save CA
        let ca = CertificateAuthority::new(Some(path.clone())).await.unwrap();
        ca.save_to_disk(&path).await.unwrap();

        // Load CA
        let loaded_ca = CertificateAuthority::load_from_disk(&path).await.unwrap();
        assert_eq!(ca.ca_cert_pem, loaded_ca.ca_cert_pem);
    }

    #[tokio::test]
    async fn test_cert_caching() {
        let ca = CertificateAuthority::new(None).await.unwrap();
        
        // Generate first cert
        let cert1 = ca.generate_domain_cert("test.com").await.unwrap();
        
        // Generate again - should return cached
        let cert2 = ca.generate_domain_cert("test.com").await.unwrap();
        
        assert_eq!(cert1.created_at, cert2.created_at);
        
        // Check cache stats
        let (total, expired) = ca.cache_stats().await;
        assert_eq!(total, 1);
        assert_eq!(expired, 0);
    }
}
