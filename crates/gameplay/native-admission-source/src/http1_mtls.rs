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
pub async fn exchange(
    desc: &ProducerDescriptor,
    operation: Operation,
    body: &str,
    permit: &mut super::QueuePermit<'_>,
) -> Result<Vec<u8>, SourceError> {
    exchange_status(desc, operation, body, permit, true)
        .await
        .map(|(_, body, _)| body)
}
/// `exchange` for an operation whose contract distinguishes failure statuses:
/// a final non-200 status is returned with its bounded body instead of
/// `Unavailable`. Framing, size and deadline rules are unchanged.
pub(crate) async fn exchange_with_status(
    desc: &ProducerDescriptor,
    operation: Operation,
    body: &str,
    permit: &mut super::QueuePermit<'_>,
) -> Result<(u16, Vec<u8>), SourceError> {
    exchange_status(desc, operation, body, permit, false)
        .await
        .map(|(status, body, _)| (status, body))
}
pub(crate) async fn exchange_store(
    desc: &ProducerDescriptor,
    operation: Operation,
    body: &str,
    permit: &mut super::QueuePermit<'_>,
) -> Result<(u16, Vec<u8>, Option<Duration>), SourceError> {
    exchange_status(desc, operation, body, permit, false).await
}
/// Request head bound: the fixed request line and fields, the longest
/// operation path, a 253-byte host and a 20-digit `Content-Length`.
const REQUEST_HEAD_BYTES: usize = 512;
/// Request envelope (head and body) for every other operation.
const REQUEST_ENVELOPE_BYTES: usize = 10_240;
/// Per-operation envelope cap: the projection snapshot (`LCA-REQUEST-BYTES`)
/// gets its own head allowance; every other operation keeps the shared cap.
const fn request_envelope_max(operation: Operation) -> usize {
    match operation {
        Operation::PublishGameJournalRecordV1 => 65536 + REQUEST_HEAD_BYTES,
        Operation::PublishAccountCharactersV1 => {
            super::account_characters::SNAPSHOT_BYTES + REQUEST_HEAD_BYTES
        }
        // `PUBPROJ-HS-REQUEST-BYTES` and `PUBPROJ-ONLINE-REQUEST-BYTES`.
        Operation::PublishWorldHighscoresV1 => {
            super::public_projections::HIGHSCORES_BYTES + REQUEST_HEAD_BYTES
        }
        Operation::PublishWorldOnlineV1 => {
            super::public_projections::ONLINE_BYTES + REQUEST_HEAD_BYTES
        }
        Operation::PublishBazaarTransferabilityV1
        | Operation::PublishBazaarCharacterCommandReceiptV2 => {
            operation.request_bytes_max() + REQUEST_HEAD_BYTES
        }
        Operation::PublishWorldEventOutcomeV1 | Operation::PublishGuildSeasonV1 => {
            operation.request_bytes_max() + REQUEST_HEAD_BYTES
        }
        _ => REQUEST_ENVELOPE_BYTES,
    }
}
const _: () = assert!(
    super::account_characters::WATERMARK_BYTES + REQUEST_HEAD_BYTES <= REQUEST_ENVELOPE_BYTES
);
const _: () = assert!(
    super::public_projections::PROFILE_BYTES + REQUEST_HEAD_BYTES <= REQUEST_ENVELOPE_BYTES
        && super::public_projections::WATERMARK_BYTES + REQUEST_HEAD_BYTES
            <= REQUEST_ENVELOPE_BYTES
);
fn request_head(operation: Operation, host_header: &str, body_len: usize) -> String {
    format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        operation.path(),
        host_header,
        body_len
    )
}
async fn exchange_status(
    desc: &ProducerDescriptor,
    operation: Operation,
    body: &str,
    permit: &mut super::QueuePermit<'_>,
    require_ok: bool,
) -> Result<(u16, Vec<u8>, Option<Duration>), SourceError> {
    permit.require_active()?;
    if body.len() > operation.request_bytes_max() {
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
        let mut request = request_head(operation, &desc.host_header, body.len());
        request.push_str(body);
        if request.len() > request_envelope_max(operation) {
            return Err(SourceError::CapacityExceeded);
        }
        tls.write_all(request.as_bytes()).await?;
        if let Some(max) = super::bazaar::transport::response_bytes(operation) {
            let mut retry_after = None;
            let (status, raw) = read_response_inner(
                &mut tls,
                false,
                max,
                true,
                &[],
                operation == Operation::ReadBazaarCommandIntentV1,
                Some(&mut retry_after),
                false,
                crate::store::MAX_BACKOFF,
            )
            .await?;
            return Ok((status, raw, retry_after));
        }
        if super::world_event_outcomes::is_operation(operation) {
            return read_t94_response(&mut tls).await;
        }
        if matches!(
            operation,
            Operation::PublishGameJournalRecordV1 | Operation::ReadGameJournalCursorV1
        ) {
            let mut retry_after = None;
            let (status, raw) = read_response_inner(
                &mut tls,
                false,
                1024,
                true,
                &[],
                false,
                Some(&mut retry_after),
                false,
                crate::store::MAX_BACKOFF,
            )
            .await?;
            return Ok((status, raw, retry_after));
        }
        if let Some(max) = super::store::response_bytes_max(operation) {
            let mut retry_after = None;
            let (status, raw) = read_response_inner(
                &mut tls,
                false,
                max,
                true,
                &[],
                false,
                Some(&mut retry_after),
                true,
                crate::store::MAX_BACKOFF,
            )
            .await?;
            return Ok((status, raw, retry_after));
        }
        let (status, raw) = if operation == Operation::ReadPremiumSnapshotV1 {
            read_response_bounded(&mut tls, require_ok, 1024, true, &[]).await
        } else if matches!(
            operation,
            Operation::ReportScopeAssignmentV1 | Operation::ReportScopeRevocationV1
        ) {
            // `NRS-RESPONSE-BYTES` bounds the body of every status, not only 200.
            // A status outside the §4 list is final without its body.
            read_response_bounded(
                &mut tls,
                require_ok,
                super::scope_assignment::RESPONSE_BYTES,
                false,
                &super::scope_assignment::RESPONSE_STATUSES,
            )
            .await
        } else if matches!(
            operation,
            Operation::PublishAccountCharactersV1
                | Operation::PublishProjectionWatermarkV1
                | Operation::PublishWorldHighscoresV1
                | Operation::PublishCharacterProfileV1
                | Operation::PublishWorldOnlineV1
                | Operation::PublishWorldTotalsV1
                | Operation::PublishPublicProjectionWatermarkV1
        ) {
            // §3 (LCFA and public projections): every projection response, success
            // or empty failure, fits 256 bytes.
            read_response_bounded(
                &mut tls,
                require_ok,
                super::account_characters::RESPONSE_BYTES,
                false,
                &[],
            )
            .await
        } else if let Some(body_max) = super::character_commands::response_bytes(operation) {
            // Character commands (§3): every response, success or empty failure,
            // fits the operation's bound. An over-bound `200` intent is reported
            // apart so it is decided invalid rather than retried as unavailable.
            let oversized_ok = operation == Operation::ReadCharacterCommandIntentV1;
            read_response_framed(&mut tls, require_ok, body_max, false, &[], oversized_ok).await
        } else if operation == Operation::PublishGameRestoreNoticeV1 {
            read_response_bounded(
                &mut tls,
                require_ok,
                super::restore_notice::RESPONSE_BYTES,
                false,
                &[],
            )
            .await
        } else if require_ok {
            read_response(&mut tls).await.map(|body| (200, body))
        } else {
            read_response_status(&mut tls, false).await
        }?;
        Ok((status, raw, None))
    };
    let exchange_millis = if matches!(
        operation,
        Operation::PublishBazaarTransferabilityV1
            | Operation::PublishBazaarListingStatusV1
            | Operation::PublishBazaarWatermarkV1
            | Operation::PublishBazaarCharacterCommandReceiptV2
    ) || super::world_event_outcomes::is_operation(operation)
    {
        5000
    } else {
        EXCHANGE_DEADLINE_MS
    };
    tokio::time::timeout(Duration::from_millis(exchange_millis), future)
        .await
        .map_err(|_| SourceError::Unavailable)?
}
pub async fn read_response<S: AsyncRead + Unpin>(stream: &mut S) -> Result<Vec<u8>, SourceError> {
    read_response_status(stream, true)
        .await
        .map(|(_, body)| body)
}
async fn read_t94_response<S: AsyncRead + Unpin>(
    stream: &mut S,
) -> Result<(u16, Vec<u8>, Option<Duration>), SourceError> {
    let mut retry_after = None;
    let (status, raw) = read_response_inner(
        stream,
        false,
        256,
        true,
        &[],
        false,
        Some(&mut retry_after),
        false,
        Duration::from_secs(10),
    )
    .await?;
    if !matches!(status, 200 | 400 | 401 | 409 | 429 | 503) || (status != 200 && !raw.is_empty()) {
        return Err(SourceError::InvalidInput);
    }
    Ok((status, raw, retry_after))
}
/// A final status: `HTTP/1.1 <200-599>[ <reason>]`; informational and
/// malformed status lines are invalid.
fn final_status(line: &str) -> Result<u16, SourceError> {
    let rest = line
        .strip_prefix("HTTP/1.1 ")
        .ok_or(SourceError::InvalidInput)?;
    let (digits, reason) = rest.split_at_checked(3).ok_or(SourceError::InvalidInput)?;
    let code = digits
        .bytes()
        .all(|b| b.is_ascii_digit())
        .then(|| digits.parse::<u16>().ok())
        .flatten()
        .ok_or(SourceError::InvalidInput)?;
    if !(200..=599).contains(&code)
        || !(reason.is_empty() || reason.starts_with(' '))
        || reason.bytes().any(|b| b.is_ascii_control())
    {
        return Err(SourceError::InvalidInput);
    }
    Ok(code)
}
async fn read_response_status<S: AsyncRead + Unpin>(
    stream: &mut S,
    require_ok: bool,
) -> Result<(u16, Vec<u8>), SourceError> {
    read_response_bounded(stream, require_ok, 8192, false, &[]).await
}
pub(crate) async fn read_response_bounded<S: AsyncRead + Unpin>(
    stream: &mut S,
    require_ok: bool,
    body_max: usize,
    require_json: bool,
    body_statuses: &[u16],
) -> Result<(u16, Vec<u8>), SourceError> {
    read_response_framed(
        stream,
        require_ok,
        body_max,
        require_json,
        body_statuses,
        false,
    )
    .await
}
/// With `oversized_ok`, a `200` body over `body_max` is `OversizedOk`; every
/// other over-capacity response stays `CapacityExceeded`.
pub(crate) async fn read_response_framed<S: AsyncRead + Unpin>(
    stream: &mut S,
    require_ok: bool,
    body_max: usize,
    require_json: bool,
    body_statuses: &[u16],
    oversized_ok: bool,
) -> Result<(u16, Vec<u8>), SourceError> {
    read_response_inner(
        stream,
        require_ok,
        body_max,
        require_json,
        body_statuses,
        oversized_ok,
        None,
        false,
        crate::store::MAX_BACKOFF,
    )
    .await
}
#[allow(
    clippy::too_many_arguments,
    reason = "Store and T94 share framing but have distinct Retry-After requirements"
)]
async fn read_response_inner<S: AsyncRead + Unpin>(
    stream: &mut S,
    require_ok: bool,
    body_max: usize,
    require_json: bool,
    body_statuses: &[u16],
    oversized_ok: bool,
    store_retry: Option<&mut Option<Duration>>,
    require_retry_after: bool,
    retry_after_max: Duration,
) -> Result<(u16, Vec<u8>), SourceError> {
    read_response_impl(
        stream,
        require_ok,
        body_max,
        require_json,
        body_statuses,
        oversized_ok,
        store_retry,
        require_retry_after,
        retry_after_max,
        false,
    )
    .await
}
#[allow(
    clippy::too_many_arguments,
    reason = "Coin framing has an independently bounded history page and strict error JSON"
)]
async fn read_response_impl<S: AsyncRead + Unpin>(
    stream: &mut S,
    require_ok: bool,
    body_max: usize,
    require_json: bool,
    body_statuses: &[u16],
    oversized_ok: bool,
    mut store_retry: Option<&mut Option<Duration>>,
    require_retry_after: bool,
    retry_after_max: Duration,
    coins: bool,
) -> Result<(u16, Vec<u8>), SourceError> {
    if body_max == 0
        || body_max
            > if coins {
                65536
            } else if store_retry.is_some() {
                12288
            } else {
                8192
            }
    {
        return Err(SourceError::InvalidInput);
    }
    let mut head = Vec::with_capacity(1024);
    let mut one = [0_u8; 1];
    let mut line_bytes = 0usize;
    let mut code = None;
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() == 8192 {
            return Err(SourceError::CapacityExceeded);
        }
        if stream.read(&mut one).await? != 1 {
            return Err(SourceError::InvalidInput);
        }
        line_bytes += 1;
        if line_bytes > if code.is_none() { 256 } else { 2048 } {
            return Err(SourceError::CapacityExceeded);
        }
        head.push(one[0]);
        if head.ends_with(b"\r\n") {
            line_bytes = 0;
            if code.is_none() {
                // The status is classified before any field is buffered.
                let status = std::str::from_utf8(&head[..head.len() - 2])
                    .map_err(|_| SourceError::InvalidInput)?;
                let status = if status == "HTTP/1.1 200 OK" {
                    200
                } else if require_ok {
                    return Err(SourceError::Unavailable);
                } else {
                    final_status(status)?
                };
                // With a status list, an unlisted status is returned without
                // its fields or body, so no head or body bound can mask it.
                if !body_statuses.is_empty() && !body_statuses.contains(&status) {
                    return Ok((status, Vec::new()));
                }
                code = Some(status);
            }
        }
    }
    let code = code.ok_or(SourceError::InvalidInput)?;
    let text = std::str::from_utf8(&head).map_err(|_| SourceError::InvalidInput)?;
    let mut lines = text.split("\r\n");
    lines.next();
    let mut fields = 0_usize;
    let mut length = None;
    let mut chunked = false;
    let mut identity_encoding = false;
    let mut content_type = None;
    let mut retry_after = None;
    let mut no_store = false;
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
            if value.is_empty()
                || !value.bytes().all(|b| b.is_ascii_digit())
                || (store_retry.is_some()
                    && (value.len() > 10 || (value.len() > 1 && value.starts_with('0'))))
            {
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
        } else if name.eq_ignore_ascii_case("content-type") {
            if content_type.is_some() {
                return Err(SourceError::InvalidInput);
            }
            content_type = Some(value.to_owned());
        } else if name.eq_ignore_ascii_case("cache-control") && store_retry.is_some() {
            no_store |= value
                .split(',')
                .any(|v| v.trim().eq_ignore_ascii_case("no-store"));
        } else if name.eq_ignore_ascii_case("retry-after") && store_retry.is_some() {
            if retry_after.is_some()
                || value.is_empty()
                || !value.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(SourceError::InvalidInput);
            }
            let delay = Duration::from_secs(value.parse().map_err(|_| SourceError::InvalidInput)?);
            // Keep zero invalid below; every positive delay is bounded by the poller.
            retry_after = Some(if delay.is_zero() {
                delay
            } else {
                delay.clamp(crate::store::POLL, retry_after_max)
            });
        } else if name.eq_ignore_ascii_case("location") {
            return Err(SourceError::InvalidInput);
        }
    }
    if coins && code == 401 {
        return Err(SourceError::Unauthorized);
    }
    if let Some(target) = store_retry.as_mut() {
        if !no_store
            || (require_retry_after
                && code == 429
                && retry_after.is_none_or(|delay| delay.is_zero()))
        {
            return Err(SourceError::InvalidInput);
        }
        **target = retry_after;
    }
    if require_json
        && (store_retry.is_none() || code == 200 || coins)
        && content_type.as_deref().is_none_or(|value| {
            let mut parts = value.split(';');
            !parts
                .next()
                .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
                || parts.any(|parameter| !parameter.trim().eq_ignore_ascii_case("charset=utf-8"))
        })
    {
        return Err(SourceError::InvalidInput);
    }
    let body = async {
        if chunked {
            return read_chunked_bounded(stream, body_max).await;
        }
        let Some(length) = length else {
            let mut body = Vec::with_capacity(body_max);
            let mut scratch = [0u8; 1024];
            loop {
                // At the cap, probe one byte without retaining any excess body.
                let available = (body_max - body.len()).clamp(1, scratch.len());
                let n = stream.read(&mut scratch[..available]).await?;
                if n == 0 {
                    return Ok(body);
                }
                if n > body_max - body.len() {
                    return Err(SourceError::CapacityExceeded);
                }
                body.extend_from_slice(&scratch[..n]);
            }
        };
        if length > body_max {
            return Err(SourceError::CapacityExceeded);
        }
        let mut body = vec![0; length];
        stream.read_exact(&mut body).await?;
        Ok(body)
    }
    .await;
    match body {
        Err(SourceError::CapacityExceeded) if oversized_ok && code == 200 => {
            Err(SourceError::OversizedOk)
        }
        body => body.map(|body| (code, body)),
    }
}
async fn read_chunked_bounded<S: AsyncRead + Unpin>(
    stream: &mut S,
    body_max: usize,
) -> Result<Vec<u8>, SourceError> {
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
        if chunks > 64 || next > body_max {
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

#[cfg(test)]
mod envelope_tests {
    use super::*;
    const OPERATIONS: [Operation; 19] = [
        Operation::ReadAccountSecurityV1,
        Operation::ReadFreshSigningTrustV1,
        Operation::ReadRecoveryAccountSecurityV2,
        Operation::ReadRecoverySigningTrustV2,
        Operation::ReadCharacterBootstrapIntentV1,
        Operation::ReadPremiumSnapshotV1,
        Operation::ReportRuntimeStatusV1,
        Operation::PublishAccountCharactersV1,
        Operation::PublishProjectionWatermarkV1,
        Operation::ReportScopeAssignmentV1,
        Operation::ReportScopeRevocationV1,
        Operation::ListPendingCharacterCommandsV1,
        Operation::ReadCharacterCommandIntentV1,
        Operation::PublishCharacterCommandReceiptV1,
        Operation::PublishAccountClosureCommandReceiptV2,
        Operation::PublishWorldHighscoresV1,
        Operation::PublishCharacterProfileV1,
        Operation::PublishWorldOnlineV1,
        Operation::PublishPublicProjectionWatermarkV1,
    ];
    /// The operations whose snapshot bound exceeds the shared envelope.
    const WIDENED: [Operation; 3] = [
        Operation::PublishAccountCharactersV1,
        Operation::PublishWorldHighscoresV1,
        Operation::PublishWorldOnlineV1,
    ];
    #[test]
    fn every_maximum_body_fits_its_envelope_and_only_the_snapshots_widen_it() {
        let host = "h".repeat(253);
        for operation in OPERATIONS {
            let head = request_head(operation, &host, usize::MAX).len();
            assert!(head <= REQUEST_HEAD_BYTES, "{operation:?} {head}");
            assert!(
                head + operation.request_bytes_max() <= request_envelope_max(operation),
                "{operation:?}"
            );
            if WIDENED.contains(&operation) {
                assert_eq!(
                    request_envelope_max(operation),
                    operation.request_bytes_max() + REQUEST_HEAD_BYTES
                );
            } else {
                assert_eq!(request_envelope_max(operation), REQUEST_ENVELOPE_BYTES);
            }
        }
        assert_eq!(
            request_envelope_max(Operation::PublishAccountCharactersV1),
            super::super::account_characters::SNAPSHOT_BYTES + REQUEST_HEAD_BYTES
        );
        assert_eq!(
            request_envelope_max(Operation::PublishWorldHighscoresV1),
            262_144 + REQUEST_HEAD_BYTES
        );
        assert_eq!(
            request_envelope_max(Operation::PublishWorldOnlineV1),
            524_288 + REQUEST_HEAD_BYTES
        );
    }
}

#[cfg(test)]
mod premium_response_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    async fn read(bytes: &[u8]) -> Result<(u16, Vec<u8>), SourceError> {
        let mut stream = bytes;
        read_response_bounded(&mut stream, true, 1024, true, &[]).await
    }
    #[test]
    fn premium_transport_enforces_actual_bound_and_framing() {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
            .block_on(async {
                let mut fixed = b"HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: 1024\r\n\r\n".to_vec();
                fixed.extend_from_slice(&[b' ';1024]);
                assert_eq!(read(&fixed).await.unwrap().1.len(), 1024);
                // Oversized declaration refuses without needing any body bytes.
                assert!(matches!(read(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1025\r\n\r\n").await, Err(SourceError::CapacityExceeded)));
                let mut eof = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n".to_vec();
                eof.extend_from_slice(&[b' ';1024]);
                assert_eq!(read(&eof).await.unwrap().1.len(), 1024);
                eof.push(b' ');
                assert!(matches!(read(&eof).await, Err(SourceError::CapacityExceeded)));
                let chunk = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n401\r\n";
                assert!(matches!(read(chunk).await, Err(SourceError::CapacityExceeded)));
                assert_eq!(read(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n\r\n").await.unwrap().1, b"{}");
                for invalid in [
                    &b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}"[..],
                    &b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 2\r\n\r\n{}"[..],
                    &b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}"[..],
                    &b"HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=iso-8859-1\r\nContent-Length: 2\r\n\r\n{}"[..],
                    &b"HTTP/1.1 302 Found\r\nContent-Type: application/json\r\nLocation: https://example.org\r\nContent-Length: 2\r\n\r\n{}"[..],
                    &b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nTransfer-Encoding: chunked\r\n\r\n{}"[..],
                ] {
                    assert!(read(invalid).await.is_err());
                }
            });
    }
}

#[cfg(test)]
mod character_command_response_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    async fn read(bytes: &[u8], oversized_ok: bool) -> Result<(u16, Vec<u8>), SourceError> {
        let mut stream = bytes;
        read_response_framed(&mut stream, false, 4, false, &[], oversized_ok).await
    }
    #[test]
    fn only_an_over_bound_success_is_told_apart() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let ok = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\n12345";
                assert!(matches!(
                    read(ok, true).await,
                    Err(SourceError::OversizedOk)
                ));
                assert!(matches!(
                    read(ok, false).await,
                    Err(SourceError::CapacityExceeded)
                ));
                let eof = b"HTTP/1.1 200 OK\r\n\r\n12345";
                assert!(matches!(
                    read(eof, true).await,
                    Err(SourceError::OversizedOk)
                ));
                let unavailable =
                    b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 5\r\n\r\n12345";
                assert!(matches!(
                    read(unavailable, true).await,
                    Err(SourceError::CapacityExceeded)
                ));
                assert_eq!(
                    read(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\n1234", true)
                        .await
                        .unwrap(),
                    (200, b"1234".to_vec())
                );
            });
    }
}

#[cfg(test)]
mod store_response_tests {
    use super::*;
    #[test]
    fn t94_rate_limit_does_not_require_store_retry_header() -> Result<(), Box<dyn std::error::Error>>
    {
        tokio::runtime::Builder::new_current_thread()
            .build()?
            .block_on(async {
                let raw =
                    b"HTTP/1.1 429 Limited\r\nCache-Control: no-store\r\nContent-Length: 0\r\n\r\n";
                let response = read_t94_response(&mut raw.as_slice()).await?;
                assert_eq!(response, (429, Vec::new(), None));
                let raw = b"HTTP/1.1 429 Limited\r\nCache-Control: no-store\r\nContent-Length: 0\r\nRetry-After: 600\r\n\r\n";
                assert_eq!(read_t94_response(&mut raw.as_slice()).await?.2, Some(Duration::from_secs(10)));
                for mut raw in [
                    b"HTTP/1.1 400 Bad\r\nContent-Length: 0\r\n\r\n".as_slice(),
                    b"HTTP/1.1 400 Bad\r\nCache-Control: no-store\r\nContent-Length: 1\r\n\r\nx",
                    b"HTTP/1.1 500 Bad\r\nCache-Control: no-store\r\nContent-Length: 0\r\n\r\n",
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: 257\r\n\r\n",
                ] {
                    assert!(read_t94_response(&mut raw).await.is_err());
                }
                Ok::<(), SourceError>(())
            })?;
        Ok(())
    }
    #[test]
    fn store_response_bounds_and_retry_after() -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread().build()?.block_on(async {
            for status in [200, 409, 429, 503] {
                let body = if status == 200 { "x".repeat(9000) } else { String::new() };
                let raw = format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\nRetry-After: 17\r\n\r\n{body}", body.len());
                let mut bytes = raw.as_bytes();
                let mut retry_after = None;
                let (got, decoded) = read_response_inner(&mut bytes, false, 12288, true, &[], false, Some(&mut retry_after), true, crate::store::MAX_BACKOFF).await?;
                assert_eq!(got, status);
                assert_eq!(decoded, body.as_bytes());
                assert_eq!(retry_after, Some(Duration::from_secs(17)));
            }
            for fields in ["", "Retry-After: x\r\n", "Retry-After: 1\r\nRetry-After: 2\r\n"] {
                let raw = format!("HTTP/1.1 429 Limited\r\nCache-Control: no-store\r\nContent-Length: 0\r\n{fields}\r\n");
                let mut bytes = raw.as_bytes();
                assert!(read_response_inner(&mut bytes, false, 512, true, &[], false, Some(&mut None), true, crate::store::MAX_BACKOFF).await.is_err());
            }
            let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: 513\r\n\r\n";
            assert!(read_response_inner(&mut raw.as_slice(), false, 512, true, &[], false, Some(&mut None), true, crate::store::MAX_BACKOFF).await.is_err());
            Ok::<(), SourceError>(())
        })?;
        Ok(())
    }

    #[test]
    fn storefix_http_caps_huge_retry_after() -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread().build()?.block_on(async {
            let raw = b"HTTP/1.1 429 Busy\r\nCache-Control: no-store\r\nContent-Length: 0\r\nRetry-After: 4294967295\r\n\r\n";
            let mut retry = None;
            read_response_inner(&mut raw.as_slice(), false, 512, true, &[], false, Some(&mut retry), true, crate::store::MAX_BACKOFF).await?;
            assert_eq!(retry, Some(crate::store::MAX_BACKOFF));
            Ok::<(), SourceError>(())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod bazaar_tests {
    use super::*;
    #[test]
    fn bazaar_http_response_requires_no_store_and_bounded_retry_after()
    -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            let body=br#"{"contract_version":2,"result":"accepted"}"#;
            let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\n\r\n{}",body.len(),std::str::from_utf8(body)?);
            let mut retry=None;
            let accepted=read_response_inner(&mut response.as_bytes(),false,256,true,&[],false,Some(&mut retry),false, crate::store::MAX_BACKOFF).await?;
            assert_eq!(accepted,(200,body.to_vec()));
            assert!(read_response_inner(&mut response.replace("Cache-Control: no-store\r\n","").as_bytes(),false,256,true,&[],false,Some(&mut retry),false, crate::store::MAX_BACKOFF).await.is_err());
            let throttled=b"HTTP/1.1 429 Too Many Requests\r\nCache-Control: no-store\r\nRetry-After: 25\r\nContent-Length: 0\r\n\r\n";
            let response=read_response_inner(&mut throttled.as_slice(),false,256,true,&[],false,Some(&mut retry),false, crate::store::MAX_BACKOFF).await?;
            assert_eq!(response,(429,Vec::new()));
            assert_eq!(retry,Some(Duration::from_secs(25)));
            assert_eq!(super::super::bazaar::transport::retry_delay(Duration::from_secs(1), retry),Duration::from_secs(10));
            assert!(request_envelope_max(Operation::PublishBazaarCharacterCommandReceiptV2)>=super::super::bazaar::transport::RECEIPT_BYTES+REQUEST_HEAD_BYTES);
            Ok(())
        })
    }
}

#[path = "http1_mtls/coins.rs"]
mod coins;
pub(crate) use coins::exchange_coins;
