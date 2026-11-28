use super::*;
use crate::gameplay_transport::item_view::ItemViewSnapshotDomain;
use crate::gameplay_transport::spell_cooldowns as owner;
use oteryn_protocol_oteryn::presentation as wire;
pub(super) use owner::CooldownFeed;

pub(super) fn snapshot<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    feed: Option<&mut CooldownFeed>,
    rows: &mut [ItemViewSnapshotDomain],
) -> Result<(), ServeEnd> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_PRESENTATION_V1)
    {
        return Ok(());
    }
    let deadlines = feed
        .and_then(CooldownFeed::take_snapshot)
        .ok_or(ServeEnd::server("cooldown_state_unavailable"))?;
    let row = rows
        .iter_mut()
        .find(|row| row.domain_id == wire::STATE_DOMAIN_ACTOR_COOLDOWNS)
        .ok_or(ServeEnd::server("cooldown_state_unavailable"))?;
    row.revision = admitted.continuity.cooldown_revision;
    row.payload = owner::payload(&deadlines, authority.cooldown_clock_us(), true)
        .map_err(|_| ServeEnd::server("cooldown_state_unavailable"))?;
    Ok(())
}

pub(super) fn frames<A: FreshAdmissionAuthority>(
    authority: &A,
    feed: Option<&CooldownFeed>,
    next: &mut SessionContinuity,
    cast_revision: Option<u64>,
) -> Result<Vec<Vec<u8>>, ServeEnd> {
    let generation = next.connection_generation;
    let Some(committed) = feed.and_then(|feed| feed.take(generation, cast_revision)) else {
        return Ok(Vec::new());
    };
    let rescued = committed
        .rescued
        .filter(|(revision, _)| next.vitals_revision != 0 && *revision > next.vitals_revision);
    let mut frames = Vec::new();
    if let Some(deadlines) = committed.deadlines {
        let sparse = next
            .selected_capabilities
            .contains(wire::CAPABILITY_ACTOR_COOLDOWN_DELTAS_V1);
        let delta_type = if sparse && rescued.is_some() {
            wire::DELTA_TYPE_ACTOR_COOLDOWNS_WITH_VITALS_V1
        } else if sparse {
            wire::DELTA_TYPE_ACTOR_COOLDOWNS_SPARSE_V1
        } else {
            wire::DELTA_TYPE_ACTOR_COOLDOWNS_V1
        };
        let revision = next
            .cooldown_revision
            .checked_add(1)
            .ok_or(ServeEnd::server("cooldown_state_unavailable"))?;
        let sequence = next
            .server_sequence
            .checked_add(1)
            .ok_or(ServeEnd::server("cooldown_state_unavailable"))?;
        let payload = owner::payload(&deadlines, authority.cooldown_clock_us(), !sparse)
            .map_err(|_| ServeEnd::server("cooldown_state_unavailable"))?;
        frames.push(
            encode_state_delta(
                generation,
                sequence,
                wire::STATE_DOMAIN_ACTOR_COOLDOWNS,
                next.cooldown_revision,
                revision,
                delta_type,
                &payload,
            )
            .map_err(|_| ServeEnd::server("cooldown_state_unavailable"))?,
        );
        next.server_sequence = sequence;
        next.cooldown_revision = revision;
    }
    if let Some((revision, value)) = rescued {
        let (sequence, frame) = vitals_delta(
            generation,
            next.server_sequence,
            next.vitals_revision,
            revision,
            &value,
        )
        .ok_or(ServeEnd::server("cooldown_state_unavailable"))?;
        next.server_sequence = sequence;
        next.vitals_revision = revision;
        frames.push(frame);
    }
    Ok(frames)
}

pub(super) async fn write_batch<S: AsyncWrite + Unpin>(
    stream: &mut S,
    frames: &[Vec<u8>],
    result: Option<&[u8]>,
) -> Result<(), ServeEnd> {
    if frames.is_empty() {
        if let Some(frame) = result {
            write_frame(stream, frame)
                .await
                .map_err(|_| ServeEnd::server("frame_write_refused"))?;
        }
        return Ok(());
    }
    let mut bytes = Vec::new();
    for frame in frames.iter().map(Vec::as_slice).chain(result) {
        let length = u32::try_from(frame.len())
            .ok()
            .and_then(|length| crate::foundation::FrameLength::new(length).ok())
            .ok_or(ServeEnd::server("frame_write_refused"))?;
        bytes.extend_from_slice(&length.to_prefix());
        bytes.extend_from_slice(frame);
    }
    tokio::time::timeout(IDLE_LIVENESS.interval, async {
        use tokio::io::AsyncWriteExt;
        stream.write_all(&bytes).await?;
        stream.flush().await
    })
    .await
    .map_err(|_| ServeEnd::server("frame_write_refused"))?
    .map_err(|_| ServeEnd::server("frame_write_refused"))
}

pub(super) async fn refresh<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd> {
    let frames = frames(
        authority,
        session.cooldowns.as_ref(),
        &mut session.admitted.continuity,
        None,
    )?;
    session.sequence = session.admitted.continuity.server_sequence;
    write_batch(stream, &frames, None).await
}
