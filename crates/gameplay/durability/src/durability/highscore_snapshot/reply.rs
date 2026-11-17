//! RANK-1 (HS-WIRE-1, HIGHSCORES0 §6, CP D1155): command type 14 `HIGHSCORES_QUERY_V1` under
//! capability 9. The handler reads one page of a persisted World snapshot (migration 0129) and
//! never computes ranks. T143 permits public World selection while the asking Character
//! and authority World remain bound to the admitted session. A category this server does not publish (outside 1..=14, 16 or 17, or 11 while no Charm
//! point source is activated; the dispatch route also withholds 13 without capability 498) is `NOT_SUPPORTED` (1200) and an echoed token that
//! is no longer retained `SNAPSHOT_EXPIRED` (1201), both FND-02 error results without a payload.
//! A malformed query, unreadable storage or a stored row the wire grammar refuses is `REJECTED`
//! with no payload and no code; a result over its bound fails closed with
//! `PAYLOAD_LIMIT_EXCEEDED`. Nothing is truncated.
//!
//! It lives beside the snapshot store so the PostgreSQL suites drive the same handler the
//! gameplay transport dispatches, including durable resolution of the current session.

use super::super::fresh_admission::FreshAdmissionStore;
use super::{DurabilityRoot, SnapshotError, SnapshotPage, SnapshotRow, served_category};
use crate::foundation::{
    CharacterId, FoundationProtocolError, GameSessionId, GameSessionState, WorldId,
};
use oteryn_protocol_oteryn::highscores::{
    HighscoreRow, HighscoresQuery, HighscoresResult, HighscoresWireError, encode_highscores_result,
};

/// The terminal outcome of one `HIGHSCORES_QUERY_V1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HighscoresReply {
    /// The encoded `HighscoresResultV1` of the page (`ACCEPTED`).
    Result(Vec<u8>),
    /// `REJECTED` with this registered operation-terminal code and no payload (FND-02 §18).
    Terminal(FoundationProtocolError),
    /// `REJECTED` with no payload and no code.
    Rejected,
}

/// HIGHSCORES0 §6: the asking Character is the one the current, non-terminal session of
/// `served_world` controls, read from durable state. A mismatched authority World, an unknown
/// session, a session without its committed admission, or a terminal session is REJECTED.
pub async fn answer_session_highscores(
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
    if current.session_state() == GameSessionState::Terminal || current.commit().world_id() != world
    {
        return HighscoresReply::Rejected;
    }
    answer_highscores(root, world, current.commit().character_id(), query).await
}

/// `HighscoresQueryV1.vocation` (`highscores_v1.proto`): 0 all, 1 knight, 2 paladin, 3 sorcerer,
/// 4 druid, 5 monk. The stored lists number their filters by the public ids instead
/// ([`super::FILTERS`]); a query names its list by this vocation key.
pub const QUERY_VOCATIONS: [Option<&str>; 6] = [
    None,
    Some("knight"),
    Some("paladin"),
    Some("sorcerer"),
    Some("druid"),
    Some("monk"),
];

/// Answer `query` for `character` of `world` from the persisted snapshots.
pub async fn answer_highscores(
    root: &DurabilityRoot,
    world: WorldId,
    character: CharacterId,
    query: HighscoresQuery,
) -> HighscoresReply {
    // The codec admits every nonzero category; one this server does not publish is
    // NOT_SUPPORTED, never an empty or fabricated ranking.
    let Some(category) = u8::try_from(query.category)
        .ok()
        .filter(|category| served_category(*category))
    else {
        return HighscoresReply::Terminal(FoundationProtocolError::NotSupported);
    };
    let Some(vocation) = usize::try_from(query.vocation)
        .ok()
        .and_then(|index| QUERY_VOCATIONS.get(index))
    else {
        return HighscoresReply::Rejected;
    };
    let target = query.world.unwrap_or(world);
    let page = root
        .read_highscore_page(
            target,
            category,
            *vocation,
            query.page,
            query.snapshot_id,
            character,
        )
        .await;
    let page = page.map(|mut page| {
        if target != world {
            page.own_row = None;
            page.own_page = None;
        }
        page
    });
    highscores_reply_with_details(page, character, query.details.then_some(target))
}

/// The reply of one read for the asking `character`: the encoded page or its fail-closed
/// outcome. An own row of another Character is a store fault and is `REJECTED`.
pub fn highscores_reply(
    page: Result<SnapshotPage, SnapshotError>,
    character: CharacterId,
) -> HighscoresReply {
    highscores_reply_with_details(page, character, None)
}

fn highscores_reply_with_details(
    page: Result<SnapshotPage, SnapshotError>,
    character: CharacterId,
    world: Option<WorldId>,
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
        world,
        own_page: world.and(page.own_page),
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
