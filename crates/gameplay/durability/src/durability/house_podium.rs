//! W18: fenced house-podium display writes and reads (migration 0561).
//!
//! Runtime reaches the 0339 storage only through `game_house_podium_set_fenced` and
//! `game_house_podium_load_fenced`, which recheck the current named-House session, scope
//! and node-holder fence; this module adds the node-incarnation proof in the same
//! transaction and, for an outfit or mount, the Character's ownership of it.
use super::character_appearance::podium_cosmetic_owned;
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::house_scope_handoff::HouseId;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic};
use oteryn_gameplay_housing::house_podium::{
    Display, DisplayKind, PodiumContent, PodiumKind, Refusal, admit_kind,
};
use sqlx::Row;

/// Displays per House read in one bounded query (0339 `LIMIT 2000`).
pub const HOUSE_PODIUM_DISPLAYS_MAX: usize = 2000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HousePodiumDisplay {
    pub item_instance_id: [u8; 16],
    pub podium_kind: u8,
    pub display_kind: u8,
    pub display_key: String,
    pub direction: u8,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HousePodiumSet {
    pub item_instance_id: [u8; 16],
    pub expected_revision: u64,
    pub display_kind: u8,
    pub display_key: String,
    pub direction: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HousePodiumRefusal {
    StaleSession,
    NoAccess,
    NotHouseItem,
    WrongItem,
    WrongDisplay,
    UnknownContent,
    NotOwned,
    Changed,
    InvalidInput,
}

impl HousePodiumRefusal {
    fn of(error: &sqlx::Error) -> Option<Self> {
        Some(match error.as_database_error()?.message() {
            "PODIUM_STALE_SESSION" => Self::StaleSession,
            "PODIUM_NO_ACCESS" => Self::NoAccess,
            "PODIUM_NOT_HOUSE_ITEM" => Self::NotHouseItem,
            "PODIUM_WRONG_ITEM" => Self::WrongItem,
            "PODIUM_WRONG_DISPLAY" => Self::WrongDisplay,
            "PODIUM_CHANGED" => Self::Changed,
            "PODIUM_INVALID_INPUT" => Self::InvalidInput,
            _ => return None,
        })
    }
}

macro_rules! fence_args {
    () => {
        "encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,\
         $5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9"
    };
}
const SET: &str = concat!(
    "SELECT game_house_podium_set_fenced(",
    fence_args!(),
    ",encode($10,'hex')::uuid,$11::smallint,$12,$13::jsonb,$14::text::numeric)::text"
);
const KIND: &str = concat!(
    "SELECT game_house_podium_kind_fenced(",
    fence_args!(),
    ",encode($10,'hex')::uuid)"
);
const LOAD: &str = concat!("SELECT * FROM game_house_podium_load_fenced(", fence_args!(), ")");

fn fenced(
    sql: &'static str,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    house: &HouseId,
) -> sqlx::query::Query<'static, sqlx::Postgres, sqlx::postgres::PgArguments> {
    let fact = node.fact();
    sqlx::query(sql)
        .bind(fence.game_session_id.as_bytes().to_vec())
        .bind(fence.character_id.as_bytes().to_vec())
        .bind(fence.connection_generation.get().to_string())
        .bind(fence.character_lease_generation.to_string())
        .bind(fence.scope_ownership_generation.get().to_string())
        .bind(fact.node_id().as_bytes().to_vec())
        .bind(fact.registration_revision().to_string())
        .bind(house.world_id().as_bytes().to_vec())
        .bind(house.house_key().to_owned())
}

fn decode(row: &sqlx::postgres::PgRow) -> Result<HousePodiumDisplay, DurabilityError> {
    let invalid = || DurabilityError::InvalidStoredState;
    let item: Vec<u8> = row.try_get("item_instance_id")?;
    let variant: serde_json::Value = row.try_get("display_variant")?;
    let direction = match variant.get("direction") {
        None => 0,
        Some(value) => u8::try_from(value.as_u64().ok_or_else(invalid)?)
            .ok()
            .filter(|d| *d <= 3)
            .ok_or_else(invalid)?,
    };
    Ok(HousePodiumDisplay {
        item_instance_id: item.try_into().map_err(|_| invalid())?,
        podium_kind: u8::try_from(row.try_get::<i16, _>("podium_kind")?).map_err(|_| invalid())?,
        display_kind: u8::try_from(row.try_get::<i16, _>("display_kind")?)
            .map_err(|_| invalid())?,
        display_key: row.try_get("display_key")?,
        direction,
        revision: row
            .try_get::<String, _>("revision")?
            .parse()
            .map_err(|_| invalid())?,
    })
}

impl DurabilityRoot {
    /// Set one podium's display under the current House session fence. The display must
    /// suit the podium and be known to content; an outfit or mount must be one the fenced
    /// Character may wear (Crystal `game.cpp:4692-4699`).
    pub async fn set_house_podium<C>(
        &self,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        house: HouseId,
        request: HousePodiumSet,
        content: C,
    ) -> Result<Result<u64, HousePodiumRefusal>, DurabilityError>
    where
        C: AppearanceContent<String> + PodiumContent + Send + Sync + 'static,
    {
        if fence.runtime_scope != house.runtime_scope()? || request.direction > 3 {
            return Ok(Err(HousePodiumRefusal::InvalidInput));
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    if !prove_current_incarnation(&mut tx, &node).await? {
                        return Ok(Err(HousePodiumRefusal::StaleSession));
                    }
                    let kind = match fenced(KIND, &node, &fence, &house)
                        .bind(request.item_instance_id.to_vec())
                        .fetch_one(&mut *tx)
                        .await
                    {
                        Ok(row) => row.try_get::<Option<i16>, _>(0)?,
                        Err(error) => {
                            return match HousePodiumRefusal::of(&error) {
                                Some(refusal) => Ok(Err(refusal)),
                                None => Err(error.into()),
                            };
                        }
                    };
                    let Some(kind) = kind
                        .and_then(|kind| u8::try_from(kind).ok())
                        .and_then(PodiumKind::from_ordinal)
                    else {
                        return Ok(Err(HousePodiumRefusal::WrongItem));
                    };
                    let Some(display) = DisplayKind::from_ordinal(request.display_kind) else {
                        return Ok(Err(HousePodiumRefusal::InvalidInput));
                    };
                    let display = Display {
                        kind: display,
                        key: &request.display_key,
                        direction: request.direction,
                    };
                    if let Err(refusal) = admit_kind(kind, display, &content) {
                        return Ok(Err(match refusal {
                            Refusal::WrongItem => HousePodiumRefusal::WrongItem,
                            Refusal::WrongDisplayKind => HousePodiumRefusal::WrongDisplay,
                            Refusal::UnknownContent => HousePodiumRefusal::UnknownContent,
                            Refusal::InvalidVariant => HousePodiumRefusal::InvalidInput,
                        }));
                    }
                    let target = match request.display_kind {
                        1 => Some(EarnedCosmetic::Outfit(&request.display_key)),
                        2 => Some(EarnedCosmetic::Mount(&request.display_key)),
                        _ => None,
                    };
                    if let Some(target) = target
                        && !podium_cosmetic_owned(&mut tx, fence.character_id, &content, target)
                            .await?
                    {
                        return Ok(Err(HousePodiumRefusal::NotOwned));
                    }
                    let result = fenced(SET, &node, &fence, &house)
                        .bind(request.item_instance_id.to_vec())
                        .bind(i16::from(request.display_kind))
                        .bind(request.display_key.clone())
                        .bind(serde_json::json!({ "direction": request.direction }))
                        .bind(request.expected_revision.to_string())
                        .fetch_one(&mut *tx)
                        .await;
                    let revision: String = match result {
                        Ok(row) => row.try_get(0)?,
                        Err(error) => {
                            return match HousePodiumRefusal::of(&error) {
                                Some(refusal) => Ok(Err(refusal)),
                                None => Err(error.into()),
                            };
                        }
                    };
                    let revision = revision
                        .parse()
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(revision))
                })
            })
            .await
    }

    /// Every committed display of the House the fenced session is in, ordered by item.
    pub async fn load_house_podiums(
        &self,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        house: HouseId,
    ) -> Result<Result<Vec<HousePodiumDisplay>, HousePodiumRefusal>, DurabilityError> {
        if fence.runtime_scope != house.runtime_scope()? {
            return Ok(Err(HousePodiumRefusal::InvalidInput));
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    if !prove_current_incarnation(&mut tx, &node).await? {
                        return Ok(Err(HousePodiumRefusal::StaleSession));
                    }
                    let rows = match fenced(LOAD, &node, &fence, &house).fetch_all(&mut *tx).await {
                        Ok(rows) => rows,
                        Err(error) => {
                            return match HousePodiumRefusal::of(&error) {
                                Some(refusal) => Ok(Err(refusal)),
                                None => Err(error.into()),
                            };
                        }
                    };
                    if rows.len() > HOUSE_PODIUM_DISPLAYS_MAX {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let displays = rows.iter().map(decode).collect::<Result<Vec<_>, _>>()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(displays))
                })
            })
            .await
    }
}
