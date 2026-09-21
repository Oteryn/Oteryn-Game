use super::{
    CONNECT_DEADLINE_MS, EXCHANGE_DEADLINE_MS, HANDSHAKE_DEADLINE_MS, HANDSHAKE_INBOUND_BYTES,
    SourceError,
    descriptor::{Operation, ProducerDescriptor},
};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
pub struct BoundedHandshakeIo<T> {
    inner: T,
    seen: usize,
    active: bool,
}
impl<T> BoundedHandshakeIo<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            seen: 0,
            active: true,
        }
    }
    pub fn finish_handshake(&mut self) {
        self.active = false
    }
    pub const fn seen(&self) -> usize {
        self.seen
    }
}
impl<T: AsyncRead + Unpin> AsyncRead for BoundedHandshakeIo<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        dst: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if !self.active {
            return Pin::new(&mut self.inner).poll_read(cx, dst);
        }
        if dst.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        let remaining = HANDSHAKE_INBOUND_BYTES.saturating_sub(self.seen);
        let max = dst.remaining().min(16_384).min(remaining.saturating_add(1));
        let mut scratch = [0u8; 16_384];
        let mut temp = ReadBuf::new(&mut scratch[..max]);
        match Pin::new(&mut self.inner).poll_read(cx, &mut temp) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => {
                let n = temp.filled().len();
                let next = match self.seen.checked_add(n) {
                    Some(v) => v,
                    None => return Poll::Ready(Err(limit_error())),
                };
                if next > HANDSHAKE_INBOUND_BYTES {
                    return Poll::Ready(Err(limit_error()));
                }
                dst.put_slice(temp.filled());
                self.seen = next;
                Poll::Ready(Ok(()))
            }
        }
    }
}
impl<T: AsyncWrite + Unpin> AsyncWrite for BoundedHandshakeIo<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        b: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, b)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}
fn limit_error() -> io::Error {
    io::Error::other("native source handshake ingress exceeded")
}
pub(crate) async fn exchange(
    desc: &ProducerDescriptor,
    operation: Operation,
    body: &str,
    permit: &mut super::QueuePermit<'_>,
) -> Result<Vec<u8>, SourceError> {
    permit.require_active()?;
    if body.len() > 1024 {
        return Err(SourceError::CapacityExceeded);
    }
    let future = async {
        let tcp = tokio::time::timeout(
            Duration::from_millis(CONNECT_DEADLINE_MS),
            tokio::net::TcpStream::connect(desc.connect_addr),
        )
        .await
        .map_err(|_| SourceError::Unavailable)??;
        let io = BoundedHandshakeIo::new(tcp);
        let connector = tokio_rustls::TlsConnector::from(desc.tls.clone());
        let mut tls = tokio::time::timeout(
            Duration::from_millis(HANDSHAKE_DEADLINE_MS),
            connector.connect(desc.server_name.clone(), io),
        )
        .await
        .map_err(|_| SourceError::Unavailable)??;
        tls.get_mut().0.finish_handshake();
        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            operation.path(),
            desc.host_header,
            body.len(),
            body
        );
        if request.len() > 10_240 {
            return Err(SourceError::CapacityExceeded);
        }
        tls.write_all(request.as_bytes()).await?;
        read_response(&mut tls).await
    };
    tokio::time::timeout(Duration::from_millis(EXCHANGE_DEADLINE_MS), future)
        .await
        .map_err(|_| SourceError::Unavailable)?
}
pub(crate) async fn read_response<S: AsyncRead + Unpin>(
    stream: &mut S,
) -> Result<Vec<u8>, SourceError> {
    let mut head = Vec::with_capacity(1024);
    let mut one = [0_u8; 1];
    let mut line_bytes = 0usize;
    let mut first_line = true;
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() == 8192 {
            return Err(SourceError::CapacityExceeded);
        }
        if stream.read(&mut one).await? != 1 {
            return Err(SourceError::InvalidInput);
        }
        line_bytes += 1;
        if line_bytes > if first_line { 256 } else { 2048 } {
            return Err(SourceError::CapacityExceeded);
        }
        head.push(one[0]);
        if head.ends_with(b"\r\n") {
            line_bytes = 0;
            first_line = false;
        }
    }
    let text = std::str::from_utf8(&head).map_err(|_| SourceError::InvalidInput)?;
    let mut lines = text.split("\r\n");
    let status = lines.next().ok_or(SourceError::InvalidInput)?;
    if status.len() > 256 || status != "HTTP/1.1 200 OK" {
        return Err(SourceError::Unavailable);
    }
    let mut fields = 0_usize;
    let mut length = None;
    let mut chunked = false;
    let mut identity_encoding = false;
    for line in lines.filter(|line| !line.is_empty()) {
        fields = fields.checked_add(1).ok_or(SourceError::CapacityExceeded)?;
        if fields > 32 || line.len() + 2 > 2048 {
            return Err(SourceError::CapacityExceeded);
        }
        let (name, value) = line.split_once(':').ok_or(SourceError::InvalidInput)?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_|~".contains(&b) || b == 96)
            || value.bytes().any(|b| b.is_ascii_control() && b != b'\t')
        {
            return Err(SourceError::InvalidInput);
        }
        let value = value.trim_matches([' ', '\t']);
        if name.eq_ignore_ascii_case("content-length") {
            if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err(SourceError::InvalidInput);
            }
            if length.is_some() || chunked {
                return Err(SourceError::InvalidInput);
            }
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| SourceError::InvalidInput)?,
            );
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            if chunked || length.is_some() || !value.trim().eq_ignore_ascii_case("chunked") {
                return Err(SourceError::InvalidInput);
            }
            chunked = true;
        } else if name.eq_ignore_ascii_case("content-encoding") {
            if identity_encoding || !value.eq_ignore_ascii_case("identity") {
                return Err(SourceError::InvalidInput);
            }
            identity_encoding = true;
        } else if name.eq_ignore_ascii_case("location") {
            return Err(SourceError::InvalidInput);
        }
    }
    if chunked {
        return read_chunked(stream).await;
    }
    let Some(length) = length else {
        let mut body = Vec::with_capacity(8192);
        let mut scratch = [0u8; 1024];
        loop {
            // At the cap, probe one byte without retaining any excess body.
            let available = (8192 - body.len()).clamp(1, scratch.len());
            let n = stream.read(&mut scratch[..available]).await?;
            if n == 0 {
                return Ok(body);
            }
            if n > 8192 - body.len() {
                return Err(SourceError::CapacityExceeded);
            }
            body.extend_from_slice(&scratch[..n]);
        }
    };
    if length > 8192 {
        return Err(SourceError::CapacityExceeded);
    }
    let mut body = vec![0; length];
    stream.read_exact(&mut body).await?;
    Ok(body)
}
async fn read_chunked<S: AsyncRead + Unpin>(stream: &mut S) -> Result<Vec<u8>, SourceError> {
    let mut body = Vec::new();
    let mut chunks = 0_usize;
    let mut framing = 0_usize;
    loop {
        let line = read_crlf_line(stream, 64).await?;
        framing = framing
            .checked_add(line.len() + 2)
            .ok_or(SourceError::CapacityExceeded)?;
        if framing > 4096 || line.is_empty() || line.contains(&b';') {
            return Err(SourceError::InvalidInput);
        }
        let text = std::str::from_utf8(&line).map_err(|_| SourceError::InvalidInput)?;
        if !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(SourceError::InvalidInput);
        }
        let size = usize::from_str_radix(text, 16).map_err(|_| SourceError::InvalidInput)?;
        if size == 0 {
            if framing
                .checked_add(2)
                .ok_or(SourceError::CapacityExceeded)?
                > 4096
            {
                return Err(SourceError::CapacityExceeded);
            }
            let trailer = read_crlf_line(stream, 2).await?;
            if !trailer.is_empty() {
                return Err(SourceError::InvalidInput);
            }
            return Ok(body);
        }
        chunks = chunks.checked_add(1).ok_or(SourceError::CapacityExceeded)?;
        let next = body
            .len()
            .checked_add(size)
            .ok_or(SourceError::CapacityExceeded)?;
        if chunks > 64 || next > 8192 {
            return Err(SourceError::CapacityExceeded);
        }
        body.resize(next, 0);
        stream.read_exact(&mut body[next - size..]).await?;
        let mut delimiter = [0_u8; 2];
        stream.read_exact(&mut delimiter).await?;
        if delimiter != *b"\r\n" {
            return Err(SourceError::InvalidInput);
        }
        framing = framing
            .checked_add(2)
            .ok_or(SourceError::CapacityExceeded)?;
        if framing > 4096 {
            return Err(SourceError::CapacityExceeded);
        }
    }
}
async fn read_crlf_line<S: AsyncRead + Unpin>(
    stream: &mut S,
    maximum: usize,
) -> Result<Vec<u8>, SourceError> {
    let mut line = Vec::new();
    let mut one = [0_u8; 1];
    loop {
        if line.len() >= maximum {
            return Err(SourceError::CapacityExceeded);
        }
        if stream.read(&mut one).await? != 1 {
            return Err(SourceError::InvalidInput);
        }
        line.push(one[0]);
        if line.ends_with(b"\r\n") {
            line.truncate(line.len() - 2);
            return Ok(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ingress_counter_overflow_rejects_before_copy() -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let (mut sender, receiver) = tokio::io::duplex(1);
                sender.write_all(&[1]).await?;
                let mut bounded = BoundedHandshakeIo::new(receiver);
                bounded.seen = usize::MAX;
                let mut out = [9u8];
                assert!(bounded.read(&mut out).await.is_err());
                assert_eq!(out, [9]);
                assert_eq!(bounded.seen, usize::MAX);
                Ok::<(), io::Error>(())
            })?;
        Ok(())
    }
}
