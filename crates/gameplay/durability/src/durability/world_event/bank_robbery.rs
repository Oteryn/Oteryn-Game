//! World-owned Bank Robbery selection and Channel projection binding.
use super::*;
use crate::durability::character_progression::assert_runtime_scope_fence;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const HOUR_MS: i64 = 3_600_000;
const MAX_HOURS_PER_POLL: i64 = 24;

/// Reserved carrier wave number for the robbery's one normal-map cohort.
pub const RAID_WAVE: u16 = u16::MAX;
/// Native Item key used by the qualified robber loot tables.
pub const BAG: &str = "oteryn:item.tibia.i13429";

#[derive(Deserialize)]
struct Index {
    bank_robbery: Config,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    hourly_roll: Roll,
    city_weights_parity: String,
    spawn_cells_parity: String,
    source: String,
    cities: [City; 4],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Roll {
    numerator: u32,
    denominator: u32,
    parity: String,
}

/// Source-qualified city and its exact normal-map placements.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct City {
    pub key: String,
    weight: u32,
    pub robber: String,
    pub cell: [u16; 3],
    pub escort: Option<Escort>,
    escort_parity: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Escort {
    pub creature: String,
    pub cells: [[u16; 3]; 4],
}

fn config() -> Result<&'static Config, DurabilityError> {
    static CONFIG: std::sync::OnceLock<Option<Config>> = std::sync::OnceLock::new();
    CONFIG.get_or_init(|| {
        let config = serde_json::from_str::<Index>(include_str!(
            "../../../../../../content/encounters/mini-world-changes/index.json"
        )).ok()?.bank_robbery;
        let pending = "PARITY_PENDING";
        if config.hourly_roll.numerator == 0
            || config.hourly_roll.numerator > config.hourly_roll.denominator
            || config.hourly_roll.parity != pending
            || config.city_weights_parity != pending
            || config.spawn_cells_parity != pending
            || config.source.is_empty()
        {
            return None;
        }
        let mut keys = std::collections::BTreeSet::new();
        let mut robbers = std::collections::BTreeSet::new();
        for city in &config.cities {
            if !keys.insert(&city.key)
                || !robbers.insert(&city.robber)
                || city.weight == 0
                || !city.robber.starts_with("oteryn:creature.")
                || city.cell[2] > 15
            {
                return None;
            }
            let valid_escort = match (&city.escort, city.escort_parity.as_deref()) {
                (Some(escort), None) if escort.creature.starts_with("oteryn:creature.") => {
                    let mut cells = std::collections::BTreeSet::from([city.cell]);
                    escort.cells.iter().all(|cell| cell[2] == city.cell[2] && cells.insert(*cell))
                }
                (None, Some("PARITY_PENDING")) => true,
                _ => false,
            };
            if !valid_escort {
                return None;
            }
        }
        Some(config)
    }).as_ref().ok_or(DurabilityError::InvalidStoredState)
}

/// The authored placements, in selection order.
pub fn cities() -> Result<&'static [City; 4], DurabilityError> {
    Ok(&config()?.cities)
}

/// A committed live robbery on the polling Channel.
#[derive(Debug, Clone, Copy)]
pub struct Projection {
    pub firing_id: [u8; 16],
    pub city: &'static City,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
}

impl Projection {
    /// The durable Channel claim is valid only for this runtime generation.
    pub fn matches(self, scope: RuntimeScopeRefV1, generation: ScopeOwnershipGeneration) -> bool {
        scope == self.scope && generation == self.generation
    }
}


fn draw(world: WorldId, epoch: u64, hour: i64, domain: u8, upper: u64) -> u64 {
    let limit = u64::MAX - u64::MAX % upper;
    let mut counter = 0_u64;
    loop {
        let mut hash = Sha256::new();
        hash.update(b"oteryn:bank-robbery:hourly:v1\0");
        hash.update(world.as_bytes());
        hash.update(epoch.to_be_bytes());
        hash.update(hour.to_be_bytes());
        hash.update([domain]);
        hash.update(counter.to_be_bytes());
        let digest = hash.finalize();
        let mut first = [0; 8];
        first.copy_from_slice(&digest[..8]);
        let sample = u64::from_be_bytes(first);
        if sample < limit {
            return sample % upper;
        }
        counter = counter.wrapping_add(1);
    }
}

fn select(config: &Config, world: WorldId, epoch: u64, hour: i64) -> Option<&City> {
    #[cfg(any(test, feature = "test-support"))]
    if let Ok(forced) = TEST_CITY.try_with(|key| *key) {
        return config.cities.iter().find(|city| city.key == forced);
    }
    if draw(world, epoch, hour, 0, u64::from(config.hourly_roll.denominator))
        >= u64::from(config.hourly_roll.numerator)
    {
        return None;
    }
    let mut choice = draw(
        world,
        epoch,
        hour,
        1,
        config.cities.iter().map(|city| u64::from(city.weight)).sum(),
    );
    config.cities.iter().find(|city| {
        let weight = u64::from(city.weight);
        if choice < weight {
            true
        } else {
            choice -= weight;
            false
        }
    })
}

#[cfg(any(test, feature = "test-support"))]
tokio::task_local! {
    static TEST_CITY: &'static str;
}

/// Force a city in the scoped PostgreSQL test future.
#[cfg(any(test, feature = "test-support"))]
pub async fn with_test_roll<T>(city: &'static str, test: impl Future<Output = T>) -> T {
    TEST_CITY.scope(city, test).await
}

impl DurabilityRoot {
    /// Establish the World boot basis on its first fenced native poll. Later
    /// Channels and restarts use the same epoch row and draw cursor.
    pub async fn poll_bank_robbery(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        epoch: u64,
        loaded_cities: [bool; 4],
    ) -> Result<Option<Projection>, DurabilityError> {
        let config = config()?;
        let channel = scope.channel_id().ok_or(DurabilityError::InvalidStoredState)?;
        let world = scope.world_id();
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                return Ok(None);
            }
            let world_id = world.as_bytes().as_slice();
            let channel_id = channel.as_bytes().as_slice();
            let epoch_id = epoch.to_string();
            let activated: Option<i64> = sqlx::query_scalar(
                "SELECT activated_at_unix_ms FROM game_world_reset_read_locked(encode($1,'hex')::uuid) WHERE reset_epoch=$2::text::numeric"
            ).bind(world_id).bind(&epoch_id).fetch_optional(&mut *tx).await?;
            let Some(activated) = activated else { return Ok(None); };
            lock_epoch(&mut tx, world, &epoch_id).await?;
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut *tx).await?;
            // Keep the prior boot basis across committed saves. A World with
            // no native robbery poll in any previous epoch starts its clock now.
            sqlx::query("INSERT INTO game_bank_robbery_occurrences(world_id,reset_epoch,boot_ms,last_checked_hour)
                SELECT encode($1,'hex')::uuid,$2::text::numeric,coalesce(prior.boot_ms,$3),
                       greatest(0,($4-coalesce(prior.boot_ms,$3))/$5)
                FROM (SELECT 1) seed
                LEFT JOIN LATERAL (
                    SELECT boot_ms FROM game_bank_robbery_occurrences
                    WHERE world_id=encode($1,'hex')::uuid AND reset_epoch<$2::text::numeric
                    ORDER BY reset_epoch DESC LIMIT 1
                ) prior ON true ON CONFLICT DO NOTHING")
                .bind(world_id).bind(&epoch_id).bind(now).bind(activated).bind(HOUR_MS)
                .execute(&mut *tx).await?;
            let row = sqlx::query("SELECT boot_ms,last_checked_hour,state,city,robber,
                       uuid_send(firing_id) AS firing,channel_id=encode($3,'hex')::uuid AS owns
                    FROM game_bank_robbery_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric FOR UPDATE")
                    .bind(world_id).bind(&epoch_id).bind(channel_id).fetch_one(&mut *tx).await?;
            let state: i16 = row.try_get("state")?;
            let owns: Option<bool> = row.try_get("owns")?;
            let mut selected = match state {
                0 => None,
                1 => {
                    let city: &str = row.try_get("city")?;
                    let robber: &str = row.try_get("robber")?;
                    let firing: &[u8] = row.try_get("firing")?;
                    let city = config.cities.iter().position(|c| c.key == city && c.robber == robber)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    Some((city, firing.try_into().map_err(|_| DurabilityError::InvalidStoredState)?))
                }
                2..=4 => {
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(None);
                }
                _ => return Err(DurabilityError::InvalidStoredState),
            };
            if state == 0 {
                let boot: i64 = row.try_get("boot_ms")?;
                let checked: i64 = row.try_get("last_checked_hour")?;
                let last = ((now-boot)/HOUR_MS).min(checked.saturating_add(MAX_HOURS_PER_POLL));
                if last > checked {
                    let mut checked_through = last;
                    for hour in checked + 1..=last {
                        if let Some(city) = select(config, world, epoch, hour) {
                            let index = config.cities.iter().position(|c| c.key == city.key)
                                .ok_or(DurabilityError::InvalidStoredState)?;
                            // A different Channel may provide this site. Leave the same
                            // deterministic hour pending while no loaded site claims it.
                            if !loaded_cities[index] {
                                checked_through = hour - 1;
                                break;
                            }
                            let firing = crate::durability::starter_grant::new_uuid(&mut tx).await?;
                            selected = Some((index, firing));
                            checked_through = hour;
                            break;
                        }
                    }
                    let city = selected.as_ref().map(|(index, _)| &config.cities[*index]);
                    let firing = selected.as_ref().map(|(_, firing)| firing.as_slice());
                    sqlx::query("UPDATE game_bank_robbery_occurrences
                        SET last_checked_hour=$3, state=$4, city=$5,robber=$6,firing_id=encode($7,'hex')::uuid
                        WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
                        .bind(world_id).bind(&epoch_id).bind(checked_through).bind(i16::from(selected.is_some()))
                        .bind(city.map(|city| city.key.as_str())).bind(city.map(|city| city.robber.as_str())).bind(firing)
                        .execute(&mut *tx).await?;
                }
            }
            let mut projection = None;
            if let Some((index, firing_id)) = selected
                && loaded_cities[index] && owns != Some(false)
            {
                if owns.is_none() {
                    sqlx::query("UPDATE game_bank_robbery_occurrences SET channel_id=encode($3,'hex')::uuid
                        WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
                        .bind(world_id).bind(&epoch_id).bind(channel_id).execute(&mut *tx).await?;
                }
                projection = Some(Projection { firing_id, city: &config.cities[index], scope, generation });
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(projection)
        })).await
    }
}

/// Fence the committed kill against the selected Channel and current save.
/// The surrounding corpse transaction rolls the transition back on any mint failure.
pub(crate) async fn claim_death(
    tx: &mut Transaction<'_, Postgres>,
    scope: RuntimeScopeRefV1,
    firing: [u8; 16],
) -> Result<Option<String>, DurabilityError> {
    let Some(channel) = scope.channel_id() else { return Ok(None); };
    let world = scope.world_id();
    let world_id = world.as_bytes().as_slice();
    let epoch: Option<String> = sqlx::query_scalar(
        "SELECT reset_epoch::text FROM game_world_reset_read_locked(encode($1,'hex')::uuid)"
    ).bind(world_id).fetch_optional(&mut **tx).await?;
    let Some(epoch) = epoch else { return Ok(None); };
    sqlx::query_scalar("UPDATE game_bank_robbery_occurrences SET state=2
        WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric
          AND firing_id=encode($3,'hex')::uuid AND channel_id=encode($4,'hex')::uuid AND state=1
        RETURNING reset_epoch::text")
        .bind(world_id).bind(epoch).bind(firing.as_slice()).bind(channel.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await.map_err(Into::into)
}

pub(crate) async fn bind_bags(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: &str,
    bags: &[(u32, [u8; 16])],
) -> Result<(), DurabilityError> {
    for &(ordinal, bag) in bags {
        let ordinal = i16::try_from(ordinal).map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_bank_robbery_bags(world_id,reset_epoch,ordinal,item_instance_id)
            VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,encode($4,'hex')::uuid)")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind(ordinal).bind(bag.as_slice())
            .execute(&mut **tx).await?;
    }
    Ok(())
}

pub const BOARD_LINE: &str = "Several banks in major coastal towns are being robbed! The thieves are still on the loose!";
pub const TOWNCRYER: &str = "oteryn:npc.towncryer";
pub const TOWNCRYER_LINE: &str = "Hear ye! Hear ye! Stand and deliver!";

pub fn banker_city(npc: &str) -> Option<&'static str> {
    match npc {
        "oteryn:npc.naji" | "oteryn:npc.suzy" => Some("thais"),
        "oteryn:npc.eva" => Some("carlin"),
        "oteryn:npc.rokyn" => Some("venore"),
        "oteryn:npc.finarfin" => Some("ab_dendriel"),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    RobberAlive,
    BagOutstanding,
    Returned,
}

impl Phase {
    pub fn suspended(self) -> bool {
        matches!(self, Self::RobberAlive | Self::BagOutstanding)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Occurrence {
    epoch: u64,
    city: String,
    phase: Phase,
}

async fn lock_epoch(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: &str,
) -> Result<(), DurabilityError> {
    // An epoch may have no row yet when the first native poll starts.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:bank-robbery:' || encode($1,'hex') || ':' || $2,0))")
        .bind(world.as_bytes().as_slice()).bind(epoch).execute(&mut **tx).await?;
    Ok(())
}

async fn current(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
) -> Result<Option<Occurrence>, DurabilityError> {
    let epoch: Option<String> = sqlx::query_scalar(
        "SELECT reset_epoch::text FROM game_world_reset_read_locked(encode($1,'hex')::uuid)",
    ).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(epoch) = epoch else { return Ok(None); };
    lock_epoch(tx, world, &epoch).await?;
    let state: Option<(i16, String)> = sqlx::query_as(
        "SELECT state, city FROM game_bank_robbery_occurrences
         WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND state BETWEEN 1 AND 3",
    ).bind(world.as_bytes().as_slice()).bind(&epoch).fetch_optional(&mut **tx).await?;
    state.map(|(state, city)| {
        let phase = match state {
            1 => Phase::RobberAlive,
            2 => Phase::BagOutstanding,
            3 => Phase::Returned,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        Ok(Occurrence {
            epoch: epoch.parse().map_err(|_| DurabilityError::InvalidStoredState)?,
            phase,
            city,
        })
    }).transpose()
}

pub(super) async fn announced(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    expected_epoch: u64,
) -> Result<bool, DurabilityError> {
    sqlx::query_scalar("SELECT EXISTS(
        SELECT 1 FROM game_world_reset_active a JOIN game_bank_robbery_occurrences o USING(world_id,reset_epoch)
        WHERE a.world_id=encode($1,'hex')::uuid AND a.reset_epoch=$2::text::numeric AND o.state IN (1,2)
    )").bind(world.as_bytes().as_slice()).bind(expected_epoch.to_string())
        .fetch_one(&mut **tx).await.map_err(Into::into)
}

pub(in crate::durability) async fn bank_suspended(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    npc: Option<&str>,
) -> Result<bool, DurabilityError> {
    let Some(city) = npc.and_then(banker_city) else { return Ok(false); };
    Ok(current(tx, world).await?.is_some_and(|o| o.city == city && o.phase.suspended()))
}

/// The exact admitted conversation. The caller resolves it under the Channel owner lock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Conversation {
    pub npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    pub key: String,
    pub generation: u64,
    pub reset_epoch: u64,
}

/// Created only by a fenced greeting while the occurrence was outstanding.
#[derive(Debug)]
pub struct RecognizedBag {
    fence: CurrentCharacterItemFence,
    conversation: Conversation,
    bag: [u8; 16],
}

pub struct Dialogue {
    pub phase: Phase,
    pub bag: Option<RecognizedBag>,
}

pub struct HandIn {
    pub command: crate::foundation::CommandRef,
    pub conversation: Conversation,
    pub text: String,
    pub bag: RecognizedBag,
}

#[derive(Debug)]
pub struct ReturnReceipt {
    pub npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    pub npc_key: String,
    bag: [u8; 16],
    reset_epoch: u64,
    prompt_generation: u64,
}

use crate::durability::character_progression::{CharacterProgressionError as ProgressionError, uuid_text};
use crate::durability::item_loop_move::ItemLoopContent;
use crate::foundation::CommandRef;
use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;

async fn player_fenced(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
) -> Result<bool, DurabilityError> {
    if fence.runtime_scope.channel_id().is_none()
        || assert_item_only_fence(tx, fence, node).await?.is_err()
    {
        return Ok(false);
    }
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions
        WHERE game_session_id=encode($1,'hex')::uuid AND session_state=2 AND current_transport_ref IS NOT NULL)")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await.map_err(Into::into)
}

async fn carried_bag(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    epoch: u64,
    bag: Option<[u8; 16]>,
) -> Result<Option<[u8; 16]>, DurabilityError> {
    let world = fence.runtime_scope.world_id();
    let bag: Option<String> = sqlx::query_scalar(
        "SELECT i.item_instance_id::text FROM game_bank_robbery_bags b
         JOIN game_item_instances i USING (item_instance_id)
         JOIN game_quest_callback_carried(encode($3,'hex')::uuid,encode($1,'hex')::uuid) c ON c.item=i.item_instance_id
         WHERE b.world_id=encode($1,'hex')::uuid AND b.reset_epoch=$2::text::numeric
           AND ($4::bytea IS NULL OR i.item_instance_id=encode($4,'hex')::uuid)
           AND b.retired_transaction_id IS NULL AND i.lifecycle=1 AND i.quantity=1
           AND i.world_id=b.world_id AND i.definition_production_key=$5 AND c.kind IN ('container','nested')
         ORDER BY b.ordinal LIMIT 1 FOR UPDATE OF i",
    )
    .bind(world.as_bytes().as_slice()).bind(epoch.to_string())
    .bind(fence.character_id.as_bytes().as_slice()).bind(bag.as_ref().map(|id| id.as_slice()))
    .bind(BAG).fetch_optional(&mut **tx).await?;
    bag.as_deref().map(uuid_text).transpose()
}

impl DurabilityRoot {
    /// Read the current World announcement under the live Channel owner fence.
    pub async fn bank_robbery_announcement(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        epoch: u64,
    ) -> Result<bool, DurabilityError> {
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                return Ok(false);
            }
            announced(&mut tx, scope.world_id(), epoch).await
        })).await
    }

    pub async fn bank_robbery_dialogue(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        conversation: Conversation,
        greeting: bool,
    ) -> Result<Option<Dialogue>, ProgressionError> {
        let Some(city) = banker_city(&conversation.key) else {
            return Ok(None);
        };
        let recovery = authority.record_for(self).map_err(|_| ProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let Some(o) = current(&mut tx, fence.runtime_scope.world_id()).await? else { return Ok(Ok(None)); };
            if city != o.city || o.epoch != conversation.reset_epoch {
                return Ok(Ok(None));
            }
            if !player_fenced(&mut tx, &node, &fence).await? {
                return Ok(Err(ProgressionError::AuthorityRejected));
            }
            let bag = if greeting && o.phase == Phase::BagOutstanding {
                carried_bag(&mut tx, &fence, o.epoch, None).await?.map(|bag| RecognizedBag {
                    fence, conversation, bag,
                })
            } else {
                None
            };
            // The observation linearizes under the occurrence lock, before any first return.
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(Some(Dialogue { phase: o.phase, bag })))
        })).await?
    }

    /// Query the immutable outcome for the exact command, optionally matching the original wire intent.
    pub async fn reconcile_bank_robbery_return(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        intent: Option<NpcTalkIntent>,
    ) -> Result<Option<ReturnReceipt>, ProgressionError> {
        let recovery = authority.record_for(self).map_err(|_| ProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let outcome = read_return(&mut tx, &fence, command, intent.as_ref()).await?;
            if matches!(&outcome, Ok(Some(_))) && !player_fenced(&mut tx, &node, &fence).await? {
                return Ok(Err(ProgressionError::AuthorityRejected));
            }
            Ok(outcome)
        })).await?
    }

    pub async fn return_bank_robbery_bag<C: ItemLoopContent>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: HandIn,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        content: &C,
    ) -> Result<Option<ReturnReceipt>, ProgressionError> {
        permit.check_scope(fence.runtime_scope)?;
        let HandIn { command, conversation, text, bag } = request;
        if command.game_session_id() != fence.game_session_id || bag.fence != fence
            || bag.conversation != conversation || !text.trim_matches(' ').eq_ignore_ascii_case("yes")
        {
            return Ok(None);
        }
        let recovery = authority.record_for(self).map_err(|_| ProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut content = content;
        self.try_issue_semantic_pass()?.run_with_context(&mut content, move |holder, deadline, content| Box::pin(async move {
            let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let world = fence.runtime_scope.world_id();
            let occurrence = current(&mut tx, world).await?;
            if !player_fenced(&mut tx, &node, &fence).await? {
                return Ok(Err(ProgressionError::AuthorityRejected));
            }
            let intent = NpcTalkIntent { npc_actor: conversation.npc, text };
            match read_return(&mut tx, &fence, command, Some(&intent)).await? {
                Ok(None) => {}
                Ok(Some(receipt)) if receipt.bag != bag.bag
                    || receipt.reset_epoch != conversation.reset_epoch
                    || receipt.prompt_generation != conversation.generation
                    || receipt.npc_key != conversation.key => {
                    return Ok(Err(ProgressionError::ConflictingOccurrence));
                }
                replay => return Ok(replay),
            }
            if !occurrence.is_some_and(|o| o.epoch == conversation.reset_epoch
                && banker_city(&conversation.key) == Some(o.city.as_str())
                && matches!(o.phase, Phase::BagOutstanding | Phase::Returned))
                || carried_bag(&mut tx, &fence, conversation.reset_epoch, Some(bag.bag)).await?.is_none()
            {
                return Ok(Ok(None));
            }
            let id = fence.character_id.as_bytes().as_slice();
            let prior: Option<(i64, bool)> = sqlx::query_as(
                "SELECT return_count, reset_epoch=$2::text::numeric AND world_id=encode($3,'hex')::uuid
                 FROM game_bank_robbery_returns WHERE character_id=encode($1,'hex')::uuid
                 ORDER BY return_count DESC LIMIT 1",
            ).bind(id).bind(conversation.reset_epoch.to_string()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            if prior.is_some_and(|(_, same_epoch)| same_epoch) {
                return Ok(Ok(None));
            }
            let return_count = prior.map_or(0, |(count, _)| count).checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            // The receipt trigger retires the exact bag before the native reward
            // owner checks weight and available backpack slots.
            sqlx::query("INSERT INTO game_bank_robbery_returns
                (game_session_id,command_id,world_id,reset_epoch,bag_id,character_id,npc_identity,npc_generation,npc_key,prompt_generation,request_text,return_count)
                VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,
                    $7,$8::text::numeric,$9,$10::text::numeric,$11,$12)")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .bind(world.as_bytes().as_slice()).bind(conversation.reset_epoch.to_string()).bind(bag.bag.as_slice()).bind(id)
                .bind(conversation.npc.identity.as_slice()).bind(conversation.npc.generation.to_string())
                .bind(&conversation.key).bind(conversation.generation.to_string()).bind(&intent.text).bind(return_count)
                .execute(&mut **tx).await?;
            match crate::durability::reward_claim_mint::npc_program::claim(
                &mut tx, &node, fence, command, *content, &payout_plan(bag.bag),
            ).await? {
                Ok(_) => {}
                Err(crate::durability::reward_claim_mint::RewardClaimMintError::Refused(_)) => return Ok(Ok(None)),
                Err(_) => return Err(DurabilityError::InvalidStoredState),
            }
            let achievements: &[&str] = match return_count {
                1 => &["oteryn:achievement/honest_finder"],
                5 => &["oteryn:achievement/goldhunter"],
                _ => &[],
            };
            crate::durability::account_achievement::grant_missing(
                &mut tx, fence.character_id, achievements, "oteryn:bank-robbery", &bag.bag,
            ).await?;
            tx.commit(deadline).await?;
            Ok(Ok(Some(ReturnReceipt {
                npc: conversation.npc, npc_key: conversation.key, bag: bag.bag,
                reset_epoch: conversation.reset_epoch, prompt_generation: conversation.generation,
            })))
        })).await?
    }
}

async fn read_return(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    intent: Option<&NpcTalkIntent>,
) -> Result<Result<Option<ReturnReceipt>, ProgressionError>, DurabilityError> {
    if fence.game_session_id != command.game_session_id() {
        return Ok(Err(ProgressionError::AuthorityRejected));
    }
    let world = fence.runtime_scope.world_id();
    let row = sqlx::query("SELECT npc_identity,npc_generation::text,npc_key,request_text,reset_epoch::text,prompt_generation::text,bag_id::text,
        character_id=encode($3,'hex')::uuid AND world_id=encode($4,'hex')::uuid AS matches
        FROM game_bank_robbery_returns WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(Ok(None)); };
    let npc = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: row.try_get::<Vec<u8>, _>("npc_identity")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
        generation: crate::durability::character_progression::numeric_u64(&row, "npc_generation")?,
    };
    let text: &str = row.try_get("request_text")?;
    if !row.try_get::<bool, _>("matches")? || intent.is_some_and(|i| i.npc_actor != npc || i.text != text) {
        return Ok(Err(ProgressionError::ConflictingOccurrence));
    }
    Ok(Ok(Some(ReturnReceipt {
        npc, npc_key: row.try_get("npc_key")?,
        bag: uuid_text(row.try_get("bag_id")?)?,
        reset_epoch: crate::durability::character_progression::numeric_u64(&row, "reset_epoch")?,
        prompt_generation: crate::durability::character_progression::numeric_u64(&row, "prompt_generation")?,
    })))
}

fn payout_plan(bag: [u8; 16]) -> oteryn_gameplay_quests::quest::npc_rules::NpcRulePlan {
    use oteryn_gameplay_quests::quest::npc_rules::{NpcRuleEffect, NpcRulePlan};
    let id = bag.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    NpcRulePlan {
        quest: "oteryn:world-change.bank-robbery".into(),
        completes: false,
        program_digest: Sha256::digest(b"oteryn:bank-robbery:return:v1").into(),
        selected_rule_keys: vec![format!("oteryn:npc-reward/bank-robbery/{id}")],
        reply: Vec::new(),
        next_topic: 0,
        observed_tracks: Default::default(),
        track_changes: Vec::new(),
        effects: vec![NpcRuleEffect::GiveItem { item: "oteryn:item.tibia.i3035".into(), count: 10, subtype: None }],
        inventory_before: Default::default(),
        inventory_after: Default::default(),
        inventory_failure_reply: Vec::new(),
    }
}
