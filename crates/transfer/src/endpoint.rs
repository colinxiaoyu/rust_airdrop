use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use quinn::{Endpoint, ServerConfig};
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

pub fn make_server_endpoint(bind_addr: SocketAddr) -> Result<Endpoint> {
    let certified_key = generate_simple_self_signed(vec!["airdrop".into()])?;
    let cert_der = certified_key.cert.der().to_vec();
    let key_der = certified_key.signing_key.serialize_der();

    let cert = CertificateDer::from(cert_der);
    let key = PrivateKeyDer::try_from(key_der)
        .map_err(|e| anyhow::anyhow!("无法解析私钥: {}", e))?;

    let server_config = ServerConfig::with_single_cert(vec![cert], key)?;

    let endpoint = Endpoint::server(server_config, bind_addr)
        .map_err(|e| anyhow::anyhow!("无法绑定端口 {}: {}", bind_addr, e))?;

    Ok(endpoint)
}

/// 创建客户端配置（跳过证书验证）
pub fn make_client_config() -> Result<quinn::ClientConfig> {
    tracing::info!("Creating client config...");

    // Create a custom certificate verifier that accepts all certificates
    let crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();
    tracing::info!("Created rustls ClientConfig");

    let quic_crypto = quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
        .map_err(|e| anyhow::anyhow!("Failed to create QuicClientConfig: {}", e))?;
    tracing::info!("Created QuicClientConfig");

    let client_config = quinn::ClientConfig::new(Arc::new(quic_crypto));
    tracing::info!("Created Quinn ClientConfig");

    Ok(client_config)
}

pub fn make_client_endpoint() -> Result<Endpoint> {
    tracing::info!("Creating client endpoint...");

    let client_config = make_client_config()?;

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)
        .map_err(|e| anyhow::anyhow!("Failed to create client endpoint: {}", e))?;
    tracing::info!("Created endpoint, local address: {:?}", endpoint.local_addr());

    endpoint.set_default_client_config(client_config);
    tracing::info!("Set default client config on endpoint");

    Ok(endpoint)
}

// Custom certificate verifier that skips validation (for testing with self-signed certs)
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
