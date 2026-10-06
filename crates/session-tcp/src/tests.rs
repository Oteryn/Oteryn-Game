use super::*;
use rustls::pki_types::PrivatePkcs8KeyDer;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

type TestResult = Result<(), Box<dyn StdError>>;

#[test]
fn root_loads_from_pem_or_der_and_rejects_bundles() -> TestResult {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let der = generated.cert.der().clone();
    let pem = pem_certificate(der.as_ref());
    assert_eq!(parse_root_certificate(pem.as_bytes())?, der);
    assert_eq!(parse_root_certificate(der.as_ref())?, der);
    let bundle = format!("{pem}{pem}");
    assert!(matches!(
        parse_root_certificate(bundle.as_bytes()),
        Err(RootCertificateError::Invalid)
    ));
    assert!(matches!(
        parse_root_certificate(b"not a certificate"),
        Err(RootCertificateError::Invalid)
    ));
    Ok(())
}

#[test]
fn configured_root_verifies_the_named_server_only() -> TestResult {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let root = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let mut server = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(vec![root.clone()], key.into())?;
    server.alpn_protocols = vec![ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(server));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0"))?;
    let address = listener.local_addr()?;
    runtime.spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let _ = acceptor.accept(tcp).await;
        }
    });
    let connect_as = |server_name: &'static str| {
        let root = root.clone();
        runtime.block_on(async move {
            connect(TcpConnect {
                address,
                server_name,
                root_certificate: &root,
                deadline: Duration::from_secs(5),
            })
            .await
            .map(|_stream| ())
        })
    };
    connect_as("localhost")?;
    assert!(matches!(
        connect_as("game.example.test"),
        Err(TcpAdapterError::Tls(_) | TcpAdapterError::Io(_))
    ));
    Ok(())
}

fn pem_certificate(der: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::new();
    for chunk in der.chunks(3) {
        let bytes = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let value = (u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]);
        for index in 0..4 {
            if index <= chunk.len() {
                encoded.push(char::from(
                    TABLE[((value >> (18 - 6 * index)) & 0x3f) as usize],
                ));
            } else {
                encoded.push('=');
            }
        }
    }
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for line in encoded.as_bytes().chunks(64) {
        pem.push_str(&String::from_utf8_lossy(line));
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem
}
