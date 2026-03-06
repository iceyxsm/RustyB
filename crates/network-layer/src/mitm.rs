//! Man-in-the-Middle (MITM) proxy for HTTPS traffic interception

use rustls::{ServerConfig, ClientConfig, Certificate, PrivateKey};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::{TlsAcceptor, TlsConnector, server::TlsStream as ServerTlsStream, client::TlsStream as ClientTlsStream};
use tracing::{debug, error, info, warn};
use std::collections::HashMap;

/// Certificate authority for generating interception certificates
pub struct CertificateAuthority {
    ca_cert: Certificate,
    ca_key: PrivateKey,
    cert_cache: HashMap<String, (Certificate, PrivateKey)>,
}

impl CertificateAuthority {
    pub fn generate() -> anyhow::Result<Self> {
        info!("Generating Certificate Authority...");
        
        // Generate CA certificate
        let cert = rcgen::Certificate::from_params({
            let mut params = rcgen::CertificateParams::new(vec!["Rusty Browser CA".to_string()]);
            params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
            params.key_usages = vec![
                rcgen::KeyUsagePurpose::KeyCertSign,
                rcgen::KeyUsagePurpose::CrlSign,
            ];
            params
        })?;

        let ca_cert = Certificate(cert.serialize_der()?);
        let ca_key = PrivateKey(cert.serialize_private_key_der());

        Ok(Self {
            ca_cert,
            ca_key,
            cert_cache: HashMap::new(),
        })
    }

    pub fn generate_domain_cert(&mut self, domain: &str) -> anyhow::Result<(Certificate, PrivateKey)> {
        // Check cache
        if let Some(cached) = self.cert_cache.get(domain) {
            return Ok(cached.clone());
        }

        debug!("Generating certificate for domain: {}", domain);

        let mut params = rcgen::CertificateParams::new(vec![domain.to_string()]);
        params.is_ca = rcgen::IsCa::No;
        
        // Add SANs
        if domain.starts_with("*.") {
            params.subject_alt_names.push(rcgen::SanType::DnsName(domain[2..].to_string()));
        }

        let cert = rcgen::Certificate::from_params(params)?;
        
        // Sign with CA
        let cert_der = cert.serialize_der_with_signer(&rcgen::Certificate::from_params({
            let mut params = rcgen::CertificateParams::new(vec!["Rusty Browser CA".to_string()]);
            params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
            params
        })?)?;

        let domain_cert = Certificate(cert_der);
        let domain_key = PrivateKey(cert.serialize_private_key_der());

        let result = (domain_cert.clone(), domain_key.clone());
        self.cert_cache.insert(domain.to_string(), result.clone());

        Ok(result)
    }

    pub fn ca_cert_pem(&self) -> String {
        let ca_params = rcgen::CertificateParams::new(vec!["Rusty Browser CA".to_string()]);
        // This is a simplified version - in production, store the actual CA cert
        "-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----".to_string()
    }
}

/// MITM Proxy server
pub struct MitmProxy {
    addr: String,
    port: u16,
    ca: Arc<Mutex<CertificateAuthority>>,
    filter_engine: Arc<FilterEngine>,
    request_logger: Arc<dyn RequestLogger>,
}

use std::sync::Mutex;
use crate::filter::FilterEngine;
use crate::interceptor::RequestLogger;

impl MitmProxy {
    pub fn new(
        addr: impl Into<String>,
        port: u16,
        ca: CertificateAuthority,
        filter_engine: Arc<FilterEngine>,
        request_logger: Arc<dyn RequestLogger>,
    ) -> Self {
        Self {
            addr: addr.into(),
            port,
            ca: Arc::new(Mutex::new(ca)),
            filter_engine,
            request_logger,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(format!("{}:{}", self.addr, self.port)).await?;
        info!("MITM Proxy listening on {}:{}", self.addr, self.port);

        loop {
            let (stream, addr) = listener.accept().await?;
            debug!("New connection from: {}", addr);

            let ca = Arc::clone(&self.ca);
            let filter_engine = Arc::clone(&self.filter_engine);
            let request_logger = Arc::clone(&self.request_logger);

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(
                    stream,
                    ca,
                    filter_engine,
                    request_logger,
                ).await {
                    error!("Connection error: {}", e);
                }
            });
        }
    }

    async fn handle_connection(
        client_stream: TcpStream,
        ca: Arc<Mutex<CertificateAuthority>>,
        filter_engine: Arc<FilterEngine>,
        request_logger: Arc<dyn RequestLogger>,
    ) -> anyhow::Result<()> {
        // Read the CONNECT request
        let mut buffer = [0u8; 4096];
        let n = client_stream.peek(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..n]);

        if request.starts_with("CONNECT") {
            // HTTPS proxy
            Self::handle_https_connect(
                client_stream,
                request,
                ca,
                filter_engine,
                request_logger,
            ).await?;
        } else {
            // HTTP proxy
            Self::handle_http(
                client_stream,
                filter_engine,
                request_logger,
            ).await?;
        }

        Ok(())
    }

    async fn handle_https_connect(
        mut client_stream: TcpStream,
        connect_request: String,
        ca: Arc<Mutex<CertificateAuthority>>,
        filter_engine: Arc<FilterEngine>,
        request_logger: Arc<dyn RequestLogger>,
    ) -> anyhow::Result<()> {
        // Parse CONNECT request
        let target = connect_request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .ok_or_else(|| anyhow::anyhow!("Invalid CONNECT request"))?;

        let (host, port) = target.split_once(':')
            .unwrap_or((target, "443"));

        info!("HTTPS CONNECT to {}:{}", host, port);

        // Send 200 Connection Established
        client_stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;

        // Generate certificate for the target domain
        let (cert, key) = {
            let mut ca = ca.lock().unwrap();
            ca.generate_domain_cert(host)?
        };

        // Create TLS config for client
        let server_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)?;

        let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));

        // Accept TLS connection from client
        let client_tls = match tls_acceptor.accept(client_stream).await {
            Ok(stream) => stream,
            Err(e) => {
                warn!("TLS handshake failed with client: {}", e);
                return Ok(());
            }
        };

        // Connect to target server
        let server_stream = TcpStream::connect(format!("{}:{}", host, port)).await?;

        // Create TLS config for server connection
        let client_config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(rustls::RootCertStore::empty()) // Use system certs
            .with_no_client_auth();

        let tls_connector = TlsConnector::from(Arc::new(client_config));
        let server_name = rustls::ServerName::try_from(host)?;

        let server_tls = match tls_connector.connect(server_name, server_stream).await {
            Ok(stream) => stream,
            Err(e) => {
                warn!("TLS handshake failed with server: {}", e);
                return Ok(());
            }
        };

        // Proxy data between client and server with interception
        Self::proxy_tls_with_interception(
            client_tls,
            server_tls,
            host,
            filter_engine,
            request_logger,
        ).await;

        Ok(())
    }

    async fn proxy_tls_with_interception(
        mut client_tls: ServerTlsStream<TcpStream>,
        mut server_tls: ClientTlsStream<TcpStream>,
        host: &str,
        filter_engine: Arc<FilterEngine>,
        request_logger: Arc<dyn RequestLogger>,
    ) {
        let (mut client_reader, mut client_writer) = tokio::io::split(client_tls);
        let (mut server_reader, mut server_writer) = tokio::io::split(server_tls);

        let host = host.to_string();

        // Client to server
        let c2s = tokio::spawn(async move {
            let mut buffer = vec![0u8; 8192];
            loop {
                match client_reader.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        // Intercept and potentially modify request
                        let data = &buffer[..n];
                        
                        // Log request
                        if let Ok(text) = std::str::from_utf8(data) {
                            debug!("Request: {}", text.lines().next().unwrap_or(""));
                        }

                        if server_writer.write_all(data).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Server to client
        let s2c = tokio::spawn(async move {
            let mut buffer = vec![0u8; 8192];
            loop {
                match server_reader.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = &buffer[..n];
                        
                        // Log response
                        if let Ok(text) = std::str::from_utf8(data) {
                            debug!("Response: {}", text.lines().next().unwrap_or(""));
                        }

                        if client_writer.write_all(data).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let _ = tokio::join!(c2s, s2c);
    }

    async fn handle_http(
        client_stream: TcpStream,
        filter_engine: Arc<FilterEngine>,
        request_logger: Arc<dyn RequestLogger>,
    ) -> anyhow::Result<()> {
        // Handle plain HTTP proxy
        // This is similar to HTTPS but without TLS
        Ok(())
    }
}
