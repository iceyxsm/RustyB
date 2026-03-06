//! TLS interception implementation

use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;

/// TLS connection interceptor
pub struct TlsInterceptor {
    acceptor: TlsAcceptor,
}

impl TlsInterceptor {
    pub fn new(acceptor: TlsAcceptor) -> Self {
        Self { acceptor }
    }

    pub async fn intercept(&self, stream: TcpStream) -> anyhow::Result<()> {
        let _tls_stream = self.acceptor.accept(stream).await?;
        tracing::debug!("TLS connection intercepted");
        Ok(())
    }
}
