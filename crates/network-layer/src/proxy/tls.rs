//! TLS interception implementation
//!
//! This module provides:
//! - TLS 1.3 server configuration for client connections
//! - TLS client configuration for upstream connections
//! - SNI parsing for certificate selection
//! - ALPN negotiation for HTTP/2 support

use crate::proxy::ca::CertificateAuthority;
use rustls::{
    ClientConfig, ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, ServerName},
};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, TlsConnector, TlsStream};
use tracing::{debug, error, warn};

/// TLS configuration for MITM proxy
pub struct TlsConfig {
    /// CA for generating certificates
    ca: CertificateAuthority,
    /// Root certificates for upstream connections
    root_certs: rustls::RootCertStore,
    /// Enable HTTP/2 ALPN
    http2_enabled: bool,
    /// Enable HTTP/3
    http3_enabled: bool,
}

impl TlsConfig {
    /// Create new TLS configuration
    pub fn new(
        ca: CertificateAuthority,
        http2_enabled: bool,
        http3_enabled: bool,
    ) -> anyhow::Result<Self> {
        // Load root certificates from system
        let root_certs = Self::load_root_certs()?;

        Ok(Self {
            ca,
            root_certs,
            http2_enabled,
            http3_enabled,
        })
    }

    /// Create TLS acceptor for incoming client connections
    pub async fn create_server_acceptor(
        &self,
        domain: &str,
    ) -> anyhow::Result<TlsAcceptor> {
        // Generate certificate for this domain
        let cached_cert = self.ca.generate_domain_cert(domain).await?;

        // Build server config
        let cert = cached_cert.cert.clone();
        let key = cached_cert.key();
        let config = self.build_server_config(cert, key)?;

        Ok(TlsAcceptor::from(Arc::new(config)))
    }

    /// Create TLS connector for upstream connections
    pub fn create_client_connector(&self) -> anyhow::Result<TlsConnector> {
        let config = self.build_client_config()?;
        Ok(TlsConnector::from(Arc::new(config)))
    }

    /// Accept TLS connection from client
    pub async fn accept_client(
        &self,
        stream: TcpStream,
        domain: &str,
    ) -> anyhow::Result<TlsStream<TcpStream>> {
        let acceptor = self.create_server_acceptor(domain).await?;
        
        match acceptor.accept(stream).await {
            Ok(tls_stream) => {
                debug!("TLS handshake successful with client for {}", domain);
                Ok(TlsStream::Server(tls_stream))
            }
            Err(e) => {
                error!("TLS handshake failed with client: {}", e);
                Err(anyhow::anyhow!("TLS client handshake failed: {}", e))
            }
        }
    }

    /// Connect to upstream server with TLS
    pub async fn connect_upstream(
        &self,
        stream: TcpStream,
        domain: &str,
    ) -> anyhow::Result<TlsStream<TcpStream>> {
        let connector = self.create_client_connector()?;
        let server_name = ServerName::try_from(domain.to_string())?;

        match connector.connect(server_name, stream).await {
            Ok(tls_stream) => {
                debug!("TLS handshake successful with upstream {}", domain);
                Ok(TlsStream::Client(tls_stream))
            }
            Err(e) => {
                // Check for certificate pinning
                if e.to_string().contains("certificate verify") {
                    warn!("Certificate verification failed for {} - possible pinning", domain);
                }
                error!("TLS handshake failed with upstream: {}", e);
                Err(anyhow::anyhow!("TLS upstream handshake failed: {}", e))
            }
        }
    }

    /// Build server configuration with domain certificate
    fn build_server_config(
        &self,
        cert: CertificateDer<'static>,
        key: PrivateKeyDer<'static>,
    ) -> anyhow::Result<ServerConfig> {
        let mut config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)?;

        // Configure ALPN protocols
        let alpn_protocols = if self.http2_enabled {
            vec![
                b"h2".to_vec(),
                b"http/1.1".to_vec(),
            ]
        } else {
            vec![b"http/1.1".to_vec()]
        };

        config.alpn_protocols = alpn_protocols;
        config.max_early_data_size = 0;

        Ok(config)
    }

    /// Build client configuration for upstream connections
    fn build_client_config(&self) -> anyhow::Result<ClientConfig> {
        let mut config = ClientConfig::builder()
            .with_root_certificates(self.root_certs.clone())
            .with_no_client_auth();

        // Configure ALPN protocols
        let alpn_protocols = if self.http2_enabled {
            vec![
                b"h2".to_vec(),
                b"http/1.1".to_vec(),
            ]
        } else {
            vec![b"http/1.1".to_vec()]
        };

        config.alpn_protocols = alpn_protocols;
        config.enable_early_data = false;

        Ok(config)
    }

    /// Load root certificates from system
    fn load_root_certs() -> anyhow::Result<rustls::RootCertStore> {
        let mut root_store = rustls::RootCertStore::empty();

        // Load native certificates
        let certs = rustls_native_certs::load_native_certs();
        
        for cert in certs.certs {
            if let Err(e) = root_store.add(cert) {
                warn!("Failed to add certificate to root store: {}", e);
            }
        }

        debug!("Loaded {} root certificates", root_store.len());
        Ok(root_store)
    }

    /// Get reference to CA
    pub fn ca(&self) -> &CertificateAuthority {
        &self.ca
    }

    /// Get CA certificate PEM for installation
    pub fn ca_cert_pem(&self) -> &str {
        self.ca.ca_cert_pem()
    }
}

/// Parse SNI from ClientHello
pub fn parse_sni(data: &[u8]) -> Option<String> {
    // Simple SNI parser for TLS ClientHello
    // This is a simplified version - production would use proper TLS parsing
    
    if data.len() < 43 {
        return None;
    }

    // Check TLS record type (handshake = 0x16)
    if data[0] != 0x16 {
        return None;
    }

    // Check TLS version
    let version = u16::from_be_bytes([data[1], data[2]]);
    if version < 0x0301 {
        return None;
    }

    // Skip record header (5 bytes) and handshake header (4 bytes)
    let mut pos = 9;

    // Skip client version (2 bytes) and random (32 bytes)
    pos += 34;

    if pos >= data.len() {
        return None;
    }

    // Skip session ID
    let session_id_len = data[pos] as usize;
    pos += 1 + session_id_len;

    if pos + 2 > data.len() {
        return None;
    }

    // Skip cipher suites
    let cipher_suites_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2 + cipher_suites_len;

    if pos + 1 > data.len() {
        return None;
    }

    // Skip compression methods
    let compression_len = data[pos] as usize;
    pos += 1 + compression_len;

    if pos + 2 > data.len() {
        return None;
    }

    // Extensions length
    let extensions_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;

    let extensions_end = pos + extensions_len;

    // Parse extensions
    while pos + 4 <= extensions_end.min(data.len()) {
        let ext_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let ext_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        // SNI extension type = 0
        if ext_type == 0 && pos + ext_len <= data.len() {
            return parse_sni_extension(&data[pos..pos + ext_len]);
        }

        pos += ext_len;
    }

    None
}

/// Parse SNI extension data
fn parse_sni_extension(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }

    let _list_len = u16::from_be_bytes([data[0], data[1]]) as usize;
    let mut pos = 2;

    while pos + 3 <= data.len() {
        let name_type = data[pos];
        let name_len = u16::from_be_bytes([data[pos + 1], data[pos + 2]]) as usize;
        pos += 3;

        // Host name type = 0
        if name_type == 0 && pos + name_len <= data.len() {
            return std::string::String::from_utf8(data[pos..pos + name_len].to_vec()).ok();
        }

        pos += name_len;
    }

    None
}

/// Check if data looks like a TLS ClientHello
pub fn is_tls_client_hello(data: &[u8]) -> bool {
    data.len() >= 6 
        && data[0] == 0x16 // Handshake
        && data[1] == 0x03 // TLS major version
        && data[5] == 0x01 // ClientHello
}

/// Get negotiated protocol from TLS stream
pub fn get_negotiated_protocol(stream: &TlsStream<TcpStream>) -> Option<String> {
    match stream {
        TlsStream::Server(s) => s.get_ref().1.alpn_protocol()
            .map(|p| std::string::String::from_utf8_lossy(p).to_string()),
        TlsStream::Client(c) => c.get_ref().1.alpn_protocol()
            .map(|p| std::string::String::from_utf8_lossy(p).to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_tls_client_hello() {
        // Valid TLS 1.2 ClientHello
        let valid = vec![0x16, 0x03, 0x03, 0x00, 0x05, 0x01];
        assert!(is_tls_client_hello(&valid));

        // Invalid - not handshake
        let invalid = vec![0x17, 0x03, 0x03, 0x00, 0x05, 0x01];
        assert!(!is_tls_client_hello(&invalid));

        // Too short
        let short = vec![0x16, 0x03];
        assert!(!is_tls_client_hello(&short));
    }

    #[test]
    fn test_parse_sni_extension() {
        // SNI extension with hostname "example.com"
        let hostname = b"example.com";
        let mut ext = vec![
            0x00, (hostname.len() + 3) as u8, // list length
            0x00, // name type (hostname)
            0x00, hostname.len() as u8, // name length
        ];
        ext.extend_from_slice(hostname);

        let result = parse_sni_extension(&ext);
        assert_eq!(result, Some("example.com".to_string()));
    }
}
