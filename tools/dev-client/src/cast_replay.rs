//! Qualification-only playback of the original server frames for one position cast.

use super::*;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTargetPosition};
use oteryn_protocol_oteryn::presentation::{
    PresentationEvent, PresentationEventKind, PresentationSource,
};
use std::fs::OpenOptions;
use std::io::{Cursor, Write};
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

type ReplayError = Box<dyn StdError + Send + Sync>;
const MAX_CAPTURE_BYTES: usize = 8 * 1024 * 1024;

struct IncomingCapture<S> {
    inner: S,
    bytes: Arc<Mutex<Option<Vec<u8>>>>,
}

impl<S: AsyncRead + Unpin> AsyncRead for IncomingCapture<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let start = output.filled().len();
        match Pin::new(&mut self.inner).poll_read(cx, output) {
            Poll::Ready(Ok(())) => {
                let mut bytes = self
                    .bytes
                    .lock()
                    .map_err(|_| io::Error::other("cast capture lock poisoned"))?;
                let Some(bytes) = bytes.as_mut() else {
                    return Poll::Ready(Ok(()));
                };
                let received = &output.filled()[start..];
                if received.len() > MAX_CAPTURE_BYTES - bytes.len() {
                    return Poll::Ready(Err(io::Error::other("cast capture bound exceeded")));
                }
                bytes.extend_from_slice(received);
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for IncomingCapture<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, input)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

struct Playback {
    bytes: Cursor<Vec<u8>>,
}

impl AsyncRead for Playback {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.bytes.position() == self.bytes.get_ref().len() as u64 {
            return Poll::Pending;
        }
        Pin::new(&mut self.bytes).poll_read(cx, output)
    }
}

impl AsyncWrite for Playback {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Record one real cast and replay the original server frames through the native session.
/// The caller provides a newly allocated capture path. Outgoing admission credentials are never recorded.
pub async fn replay_live_position_cast(
    request: JoinRequest<'_>,
    spell: NonZeroU32,
    target: SpellTargetPosition,
    expected_vitals: ActorVitals,
    capture_path: &Path,
) -> Result<(CastOutcome, [u8; 16]), ReplayError> {
    let admission = || Admission {
        appearance_items: None,
        schema_revision: request.schema_revision,
        character_id: request.character_id,
        admission_material: request.admission_material,
        client_build_id: request.client_build_id,
        supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
        deadline: request.deadline,
    };
    let stream = oteryn_session_tcp::connect(TcpConnect {
        address: request.address,
        server_name: request.server_name,
        root_certificate: request.root_certificate,
        deadline: request.deadline,
    })
    .await?;
    let bytes = Arc::new(Mutex::new(Some(Vec::new())));
    let mut live = Session::admit(
        IncomingCapture {
            inner: stream,
            bytes: Arc::clone(&bytes),
        },
        admission(),
    )
    .await?;
    let position = live.world_spatial().actor_position;
    if position.x.checked_add(3) != Some(target.x)
        || position.y != target.y
        || position.floor != target.floor
        || live.actor_vitals() != Some(&expected_vitals)
    {
        return Err("C18b cast admission did not retain the prepared position and vitals".into());
    }
    let ready = live
        .actor_cooldown_snapshot()
        .ok_or("C18b cooldown snapshot absent")?;
    if ready
        .entries
        .iter()
        .any(|entry| entry.expires_at_ms > ready.server_now_ms)
    {
        return Err("C18b fixture has an active cooldown".into());
    }
    let session = live.join_snapshot().game_session_id;
    let result = live
        .cast_spell(spell, SpellTarget::Position(target), false)
        .await?;
    if result.disposition != SpellCastDisposition::Cast {
        return Err(format!("C18b first position cast refused: {result:?}").into());
    }
    let impact = cursor_impact(&mut live, target, result.result_server_sequence).await?;
    {
        let capture = bytes
            .lock()
            .map_err(|_| "C18b capture lock poisoned")?
            .take()
            .ok_or("C18b capture already consumed")?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(capture_path)?.write_all(&capture)?;
    }
    let playback = Playback {
        bytes: Cursor::new(std::fs::read(capture_path)?),
    };
    let mut replay = Session::admit(playback, admission()).await?;
    if replay.join_snapshot().game_session_id != session
        || replay.actor_vitals() != Some(&expected_vitals)
    {
        return Err("C18b replay admission differs from the live cast session".into());
    }
    let replay_result = replay
        .cast_spell(spell, SpellTarget::Position(target), false)
        .await?;
    let replay_impact = cursor_impact(&mut replay, target, result.result_server_sequence).await?;
    if replay_impact != impact {
        return Err("C18b replay cursor impact differs from the live cast".into());
    }
    if replay_result != result || replay.actor_vitals() != live.actor_vitals() {
        return Err(
            "C18b replay cast result or applied vitals differ from the live session".into(),
        );
    }
    let grace = tokio::time::Instant::now() + Duration::from_secs(60);
    while live
        .combat_state()
        .ok_or("C18b combat state absent")?
        .in_fight
    {
        let remaining = grace.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("C18b combat lock did not clear within 60 seconds".into());
        }
        live.service_liveness_discarding_events(remaining.min(Duration::from_millis(100)))
            .await?;
    }
    if live.actor_vitals() != result.actor_vitals_delta.as_ref().map(|delta| &delta.value) {
        return Err("C18b cast vitals changed before logout".into());
    }
    let logout = live.logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C18b asserted logout refused: {logout:?}").into());
    }
    Ok((result, *session.as_bytes()))
}

async fn cursor_impact<S: oteryn_session::SessionStream>(
    session: &mut Session<S>,
    target: SpellTargetPosition,
    after: u64,
) -> Result<(u64, PresentationEvent), ReplayError> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("C18b cast emitted no own magic impact at the cursor".into());
        }
        for event in session.wait_for_events(remaining).await? {
            if let SessionEvent::Presentation(delta) = event
                && delta.server_sequence > after
                && let Some(impact) = delta.value.into_iter().find(|event| matches!(
                    event.kind,
                    PresentationEventKind::MagicEffect { position, source: PresentationSource::Own, .. }
                        if position.x == target.x && position.y == target.y && position.floor == target.floor
                ))
            {
                return Ok((delta.server_sequence, impact));
            }
        }
    }
}
