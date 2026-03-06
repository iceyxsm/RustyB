//! TLS certificate generation for MITM

use rcgen::{Certificate, CertificateParams, KeyPair};

/// Certificate generator for TLS interception
pub struct CertGenerator {
    ca_cert: Certificate,
    ca_key: KeyPair,
}

impl CertGenerator {
    pub fn generate_ca() -> anyhow::Result<(Certificate, KeyPair)> {
        let mut params = CertificateParams::new(vec!["Rusty Browser CA".to_string()]);
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        
        let key_pair = KeyPair::generate()?;
        let cert = params.self_signed(&key_pair)?;
        
        Ok((cert, key_pair))
    }

    pub fn generate_domain_cert(
        ca_cert: &Certificate,
        ca_key: &KeyPair,
        domain: &str,
    ) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        let mut params = CertificateParams::new(vec![domain.to_string()]);
        params.is_ca = rcgen::IsCa::No;
        
        let key_pair = KeyPair::generate()?;
        let cert = params.signed_by(&ca_cert, &ca_key)?;
        
        Ok((
            cert.pem().into_bytes(),
            key_pair.serialize_pem().into_bytes(),
        ))
    }
}
