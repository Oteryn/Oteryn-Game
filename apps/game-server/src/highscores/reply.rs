//! RANK-1 (HS-WIRE-1, HIGHSCORES0 §6, CP D1155): command type 14 `HIGHSCORES_QUERY_V1` under
//! capability 9. The handler reads one page of a persisted World snapshot (migration 0129) and
//! never computes ranks. The World and the asking Character come from the admitted session, never
//! from the payload. A category this server does not publish (outside 1..=11, or 11 while no Charm
//! point source is activated) is `NOT_SUPPORTED` (1200) and an echoed token that
//! is no longer retained `SNAPSHOT_EXPIRED` (1201), both FND-02 error results without a payload.
//! A malformed query, unreadable storage or a stored row the wire grammar refuses is `REJECTED`
//! with no payload and no code; a result over its bound fails closed with
//! `PAYLOAD_LIMIT_EXCEEDED`. Nothing is truncated.
//!
//! It lives beside the snapshot store so the PostgreSQL suites drive the same handler the
//! gameplay transport dispatches, including durable resolution of the current session.

use super::super::fresh_admission::FreshAdmissionStore;
use super::{DurabilityRoot, FILTERS, SnapshotError, SnapshotPage, SnapshotRow, served_categories};
use crate::foundation::{
    CharacterId, FoundationProtocolError, GameSessionId, GameSessionState, WorldId,
};
use oteryn_protocol_oteryn::highscores::{
    HighscoreRow, HighscoresQuery, HighscoresResult, HighscoresWireError, encode_highscores_result,
};

/// The terminal outcome of one `HIGHSCORES_QUERY_V1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HighscoresReply {
    /// The encoded `HighscoresResultV1` of the page (`ACCEPTED`).
    Result(Vec<u8>),
    /// `REJECTED` with this registered operation-terminal code and no payload (FND-02 §18).
    Terminal(FoundationProtocolError),
    /// `REJECTED` with no payload and no code.
    Rejected,
}

/// HIGHSCORES0 §6: the asking Character is the one the current, non-terminal session of
/// `served_world` controls, read from durable state. A query for another World, an unknown
/// session, a session without its committed admission, or a terminal session is REJECTED.
pub(crate) async fn answer_session_highscores(
    root: &DurabilityRoot,
    served_world: WorldId,
    world: WorldId,
    session: GameSessionId,
    query: HighscoresQuery,
) -> HighscoresReply {
    if world != served_world {
        return HighscoresReply::Rejected;
    }
    let store = FreshAdmissionStore::from_root(root.clone());
    let Ok((current, _)) = store.current_session_at(session).await else {
        return HighscoresReply::Rejected;
    };
    if current.session_state() == GameSessionState::Terminal {
        return HighscoresReply::Rejected;
    }
    answer_highscores(root, world, current.commit().character_id(), query).await
}

/// Answer `query` for `character` of `world` from the persisted snapshots.
pub(crate) async fn answer_highscores(
    root: &DurabilityRoot,
    world: WorldId,
    character: CharacterId,
    query: HighscoresQuery,
) -> HighscoresReply {
    // The codec admits every nonzero category; one this server does not publish is
    // NOT_SUPPORTED, never an empty or fabricated ranking.
    let Some(category) = u8::try_from(query.category)
        .ok()
        .filter(|category| (1..=served_categories()).contains(category))
    else {
        return HighscoresReply::Terminal(FoundationProtocolError::NotSupported);
    };
    let Some(vocation) = usize::try_from(query.vocation)
        .ok()
        .and_then(|index| FILTERS.get(index))
    else {
        return HighscoresReply::Rejected;
    };
    let page = root
        .read_highscore_page(
            world,
            category,
            *vocation,
            query.page,
            query.snapshot_id,
            character,
        )
        .await;
    highscores_reply(page, character)
}

/// The reply of one read for the asking `character`: the encoded page or its fail-closed
/// outcome. An own row of another Character is a store fault and is `REJECTED`.
pub(crate) fn highscores_reply(
    page: Result<SnapshotPage, SnapshotError>,
    character: CharacterId,
) -> HighscoresReply {
    let page = match page {
        Ok(page) => page,
        Err(SnapshotError::NotSupported) => {
            return HighscoresReply::Terminal(FoundationProtocolError::NotSupported);
        }
        Err(SnapshotError::SnapshotExpired) => {
            return HighscoresReply::Terminal(FoundationProtocolError::SnapshotExpired);
        }
        Err(
            SnapshotError::InvalidQuery
            | SnapshotError::RetryableConflict
            | SnapshotError::CapacityExceeded
            | SnapshotError::Unavailable(_),
        ) => return HighscoresReply::Rejected,
    };
    if page
        .own_row
        .as_ref()
        .is_some_and(|own| own.character_id != character)
    {
        return HighscoresReply::Rejected;
    }
    let result = HighscoresResult {
        category: u32::from(page.snapshot.category),
        snapshot_id: page.snapshot.snapshot_id,
        computed_at: page.snapshot.computed_at_ms,
        page: page.page,
        has_more: page.has_more,
        rows: page.rows.into_iter().map(wire_row).collect(),
        own_row: page.own_row.map(wire_row),
    };
    match encode_highscores_result(&result) {
        Ok(payload) => HighscoresReply::Result(payload),
        Err(HighscoresWireError::LimitExceeded) => {
            HighscoresReply::Terminal(FoundationProtocolError::PayloadLimitExceeded)
        }
        Err(HighscoresWireError::Malformed) => HighscoresReply::Rejected,
    }
}

/// The wire row shows the primary value only; the hidden progress broke ties in the snapshot.
fn wire_row(row: SnapshotRow) -> HighscoreRow {
    HighscoreRow {
        rank: row.rank,
        name: row.name,
        vocation: row.vocation,
        level: row.level,
        value: row.value.0,
    }
}
