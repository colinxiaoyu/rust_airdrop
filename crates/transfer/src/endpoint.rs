use std::net::SocketAddr;

use quinn::{Endpoint, ServerConfig};
use rcgen::generate_simple_self_signed;

pub fn make_server_endpoint(bind_addr: SocketAddr) -> Endpoint {
    let certified_key = generate_simple_self_signed(vec!["airdrop".into()]).unwrap();
    let cert_der = certified_key.cert.der().to_vec();
    let key_der = certified_key.key_pair.serialize_der();

    let server_config = ServerConfig::with_single_cert(
        vec![quinn::Certificate::from_der(&cert_der).unwrap()],
        quinn::PrivateKey::from_der(&key_der).unwrap(),
    )
    .unwrap();

    Endpoint::server(server_config, bind_addr).unwrap()
}

pub fn make_client_endpoint() -> Endpoint {
    let mut endpoint = Endpoint::client("0.0.0.0".parse().unwrap()).unwrap();
    endpoint.set_default_client_config(quinn::ClientConfig::with_native_roots());
    endpoint
}
