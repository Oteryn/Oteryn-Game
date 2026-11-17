use super::*;
pub(crate) async fn exchange_coins(
    desc: &ProducerDescriptor,
    method: &str,
    path: &str,
    body: &str,
    permit: &mut crate::QueuePermit<'_>,
) -> Result<(u16, Vec<u8>), SourceError> {
    permit.require_active()?;
    if !matches!(method, "GET" | "POST")
        || !path.starts_with("/internal/v1/coins/")
        || path.len() > 256
        || !path
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/-?=&".contains(&c))
        || body.len() > 1024
        || (method == "GET" && !body.is_empty())
    {
        return Err(SourceError::InvalidInput);
    }
    let work = async {
        let tcp = tokio::time::timeout(
            Duration::from_millis(CONNECT_DEADLINE_MS),
            tokio::net::TcpStream::connect(desc.connect_addr),
        )
        .await
        .map_err(|_| SourceError::Unavailable)??;
        let connector = tokio_rustls::TlsConnector::from(desc.tls.clone());
        let mut tls = tokio::time::timeout(
            Duration::from_millis(HANDSHAKE_DEADLINE_MS),
            connector.connect(desc.server_name.clone(), BoundedHandshakeIo::new(tcp)),
        )
        .await
        .map_err(|_| SourceError::Unavailable)??;
        tls.get_mut().0.finish_handshake();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            desc.host_header,
            body.len()
        );
        tls.write_all(request.as_bytes()).await?;
        let maximum = if path.contains("/history?") {
            65536
        } else {
            2048
        };
        read_response_impl(
            &mut tls,
            false,
            maximum,
            true,
            &[],
            false,
            Some(&mut None),
            false,
            Duration::from_secs(30),
            true,
            false,
        )
        .await
    };
    tokio::time::timeout(Duration::from_millis(EXCHANGE_DEADLINE_MS), work)
        .await
        .map_err(|_| SourceError::Unavailable)?
}
