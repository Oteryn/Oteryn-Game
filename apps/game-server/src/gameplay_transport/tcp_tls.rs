use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::io;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, rustls, server::TlsStream};

pub(super) fn tls_config(
    certificates: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<Arc<rustls::ServerConfig>, rustls::Error> {
    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(certificates, key)?;
    config.alpn_protocols = vec![crate::foundation::ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    config.max_early_data_size = 0;
    Ok(Arc::new(config))
}

pub(super) async fn accept_tls(
    stream: TcpStream,
    config: Arc<rustls::ServerConfig>,
) -> io::Result<TlsStream<TcpStream>> {
    let stream = TlsAcceptor::from(config).accept(stream).await?;
    let (_, negotiated) = stream.get_ref();
    if negotiated.protocol_version() != Some(rustls::ProtocolVersion::TLSv1_3)
        || negotiated.alpn_protocol() != Some(crate::foundation::ALPN_OTERYN_GAME_V1.as_bytes())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "gameplay TLS profile mismatch",
        ));
    }
    Ok(stream)
}

pub(super) async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Vec<u8>> {
    let mut prefix = [0u8; 4];
    reader.read_exact(&mut prefix).await?;
    let length = crate::foundation::FrameLength::from_prefix(&prefix)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let mut body = vec![0; length.get() as usize];
    reader.read_exact(&mut body).await?;
    Ok(body)
}

pub(super) async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    body: &[u8],
) -> io::Result<()> {
    let length = u32::try_from(body.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "gameplay frame length overflow",
        )
    })?;
    let length = crate::foundation::FrameLength::new(length)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    writer.write_all(&length.to_prefix()).await?;
    writer.write_all(body).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use tokio::io::AsyncWriteExt;
    use tokio::net::{TcpListener, TcpStream};

    #[test]
    fn transport_tls12_rejected_before_frame_handoff() -> Result<(), Box<dyn Error>> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(async {
                use rustls::pki_types::{PrivatePkcs8KeyDer, ServerName};
                use std::sync::Arc;
                use tokio_rustls::{TlsConnector, rustls};
                let material = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
                let cert = material.cert.der().clone();
                let key = PrivatePkcs8KeyDer::from(material.signing_key.serialize_der());
                let server = tls_config(vec![cert.clone()], key.into())?;
                for (version, alpn, valid) in [
                    (
                        &rustls::version::TLS13,
                        Some(b"oteryn-game/1".to_vec()),
                        true,
                    ),
                    (
                        &rustls::version::TLS12,
                        Some(b"oteryn-game/1".to_vec()),
                        false,
                    ),
                    (&rustls::version::TLS13, Some(b"wrong".to_vec()), false),
                    (&rustls::version::TLS13, None, false),
                ] {
                    let listener = TcpListener::bind("127.0.0.1:0").await?;
                    let address = listener.local_addr()?;
                    let server = server.clone();
                    let receiver = tokio::spawn(async move {
                        let (stream, _) = listener.accept().await?;
                        let mut authenticated = accept_tls(stream, server).await?;
                        read_frame(&mut authenticated).await
                    });
                    let mut roots = rustls::RootCertStore::empty();
                    roots.add(cert.clone())?;
                    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
                        rustls::crypto::ring::default_provider(),
                    ))
                    .with_protocol_versions(&[version])?
                    .with_root_certificates(roots)
                    .with_no_client_auth();
                    config.alpn_protocols = alpn.into_iter().collect();
                    let connector = TlsConnector::from(Arc::new(config));
                    let client = connector
                        .connect(
                            ServerName::try_from("localhost")?,
                            TcpStream::connect(address).await?,
                        )
                        .await;
                    if let Ok(mut client) = client {
                        // Invalid ALPN can be noticed by the server just after the
                        // client completes its handshake. Its frame must not escape.
                        let _ = client.write_all(&[0, 0, 0, 1, 7]).await;
                        let _ = client.shutdown().await;
                    } else {
                        assert!(!valid);
                    }
                    let received = receiver.await?;
                    assert_eq!(received.is_ok(), valid);
                    if valid {
                        assert_eq!(received?, vec![7]);
                    }
                }
                let listener = TcpListener::bind("127.0.0.1:0").await?;
                let mut client = TcpStream::connect(listener.local_addr()?).await?;
                let (stream, _) = listener.accept().await?;
                client.write_all(b"plaintext").await?;
                client.shutdown().await?;
                assert!(accept_tls(stream, server).await.is_err());
                Ok::<_, Box<dyn Error>>(())
            })
    }

    #[test]
    fn frame_writer_rejects_before_any_output() -> Result<(), Box<dyn Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let mut output = Vec::new();
                assert!(write_frame(&mut output, &[]).await.is_err());
                assert!(output.is_empty());
                assert!(write_frame(&mut output, &vec![0; 1_048_577]).await.is_err());
                assert!(output.is_empty());
                write_frame(&mut output, &[7]).await?;
                assert_eq!(output, vec![0, 0, 0, 1, 7]);
                Ok::<_, Box<dyn Error>>(())
            })
    }

    #[test]
    fn frame_boundaries_and_truncation() -> Result<(), Box<dyn Error>> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(async {
                for (length, body, valid) in [
                    (0u32, vec![], false),
                    (1_048_577, vec![], false),
                    (1, vec![7], true),
                    (1_048_576, vec![7; 1_048_576], true),
                    (8, vec![7], false),
                ] {
                    let listener = TcpListener::bind("127.0.0.1:0").await?;
                    let address = listener.local_addr()?;
                    let sender = tokio::spawn(async move {
                        let mut client = TcpStream::connect(address).await?;
                        client.write_all(&length.to_be_bytes()).await?;
                        client.write_all(&body).await?;
                        client.shutdown().await
                    });
                    let (mut stream, _) = listener.accept().await?;
                    let result = read_frame(&mut stream).await;
                    assert_eq!(result.is_ok(), valid, "length {length}");
                    if valid {
                        assert_eq!(result?.len(), length as usize);
                    }
                    sender.await??;
                }
                let listener = TcpListener::bind("127.0.0.1:0").await?;
                let mut client = TcpStream::connect(listener.local_addr()?).await?;
                let (mut stream, _) = listener.accept().await?;
                client.write_all(&[0, 1]).await?;
                client.shutdown().await?;
                assert!(read_frame(&mut stream).await.is_err());
                Ok::<_, Box<dyn Error>>(())
            })
    }
}
