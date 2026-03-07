//! TLS interception module
//!
//! This module provides TLS certificate generation and interception capabilities.
//! For the main MITM proxy implementation, see `crate::proxy`.

pub mod cert_generator;
pub mod interceptor;

pub use cert_generator::*;
pub use interceptor::*;

// Re-export proxy TLS types for backwards compatibility
pub use crate::proxy::tls::*;
