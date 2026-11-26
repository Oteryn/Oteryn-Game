//! The item, personal quest progress and timed Nomads receipt commit together.
use super::{
    DurabilityError, DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    db::{begin_semantic_transaction, commit_semantic_transaction},
    item_loop_move::{ItemLoopContent, LoopDestination, LoopMoveRequest, write_nomads_drop},
    item_transfer::CurrentCharacterItemFence,
    item_use::{
        ItemUseError,
        fluid::{ItemFluidOutcome, ItemFluidRequest, ItemFluidSplash, write_fluid},
    },
    quest_world_callback::QuestWorldContext,
    runtime_scope_assignment::NodeIncarnationProof,
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use oteryn_gameplay_quests::quest::world_callbacks::nomads_ritual;
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};

#[derive(Clone, Copy, Debug)]
pub struct NomadsRitualTarget {
    pub map_digest: [u8; 32],
    pub reset_epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NomadsRitualGesture {
    Drop,
    Pour,
}

#[derive(Debug)]
pub struct NomadsRitualReceipt {
    pub target: NomadsRitualTarget,
    pub gesture: NomadsRitualGesture,
    pub item: [u8; 16],
    pub deadline: std::time::Instant,
}

impl NomadsRitualGesture {
    pub const fn callback(self) -> &'static str {
        match self {
            Self::Drop => nomads_ritual::PLACE_SCIMITAR,
            Self::Pour => nomads_ritual::OPEN_ENTRANCE,
        }
    }
}

pub fn is_callback(key: &str) -> bool {
    matches!(
        key,
        nomads_ritual::PLACE_SCIMITAR | nomads_ritual::OPEN_ENTRANCE
    )
}

pub enum NomadsPhysical<'a> {
    Drop {
        request: &'a LoopMoveRequest,
        slot: Option<u32>,
        content: &'a dyn ItemLoopContent,
    },
    Pour {
        request: &'a ItemFluidRequest,
        splash: &'a ItemFluidSplash,
    },
}

impl NomadsPhysical<'_> {
    pub const fn gesture(&self) -> NomadsRitualGesture {
        match self {
            Self::Drop { .. } => NomadsRitualGesture::Drop,
            Self::Pour { .. } => NomadsRitualGesture::Pour,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn write(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        permit: &SpellLanePermit,
        recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
        context: &QuestWorldContext,
    ) -> Result<bool, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Ok(false);
        };
        if fence.game_session_id != command.game_session_id() {
            return Ok(false);
        }
        // The channel lane covers publication. This lock also orders committing
        // writers on the durable scope, including across holder generations.
        sqlx::query(
            "SELECT pg_advisory_xact_lock(hashtextextended('oteryn:nomads:'||encode($1,'hex'),0))",
        )
        .bind(channel_id.as_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
        let map = context
            .map_revision
            .strip_prefix("sha256:")
            .ok_or(DurabilityError::InvalidStoredState)?;
        let mut map_digest = [0; 32];
        for (index, byte) in map_digest.iter_mut().enumerate() {
            let pair = map
                .get(index * 2..index * 2 + 2)
                .ok_or(DurabilityError::InvalidStoredState)?;
            *byte =
                u8::from_str_radix(pair, 16).map_err(|_| DurabilityError::InvalidStoredState)?;
        }
        let target = NomadsRitualTarget {
            map_digest,
            reset_epoch: context.reset_epoch,
        };
        let drop = self.gesture() == NomadsRitualGesture::Drop;
        let active = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM game_nomads_ritual_gestures
                  WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid
                    AND map_digest=$3 AND reset_epoch=$4::text::numeric
                    AND (move_transaction_id IS NOT NULL)=$5 AND expires_at>clock_timestamp())",
        )
        .bind(world_id.as_bytes().as_slice())
        .bind(channel_id.as_bytes().as_slice())
        .bind(target.map_digest.as_slice())
        .bind(target.reset_epoch.to_string())
        .bind(drop)
        .fetch_one(&mut **tx)
        .await?;
        if active {
            return Ok(false);
        }
        let lifetime = i64::from(
            callback
                .effects
                .reset_after_ms
                .ok_or(DurabilityError::InvalidStoredState)?,
        );
        match self {
            Self::Drop {
                request,
                slot,
                content,
            } => {
                if !matches!(&request.destination, LoopDestination::Ground(place) if place.spatial_position == crack_cell())
                {
                    return Ok(false);
                }
                let Some(receipt) = write_nomads_drop(
                    tx, permit, recovery, node, &fence, command, *content, request, *slot,
                )
                .await?
                else {
                    return Ok(false);
                };
                // The loop writer checks its own deferred constraints while
                // the item is still on the ground. The offering then consumes
                // it under the quest receipt, whose proof is written below.
                sqlx::query("SET CONSTRAINTS ALL DEFERRED")
                    .execute(&mut **tx)
                    .await?;
                record_drop(tx, receipt.transaction_id, request.item, target, lifetime).await
            }
            Self::Pour { request, splash } => {
                if request.command_id != command.command_id().get()
                    || request.other.is_some()
                    || request.change.fluid_after != 0
                    || i16::from(request.change.fluid_before) != nomads_ritual::WATER_SUBTYPE
                    || request.change.fluid_before != splash.fluid_subtype
                    || !is_water_placement(&splash.placement)
                    || splash.placement[..32] != target.map_digest
                    || splash.splash_item_id != 2886
                    || splash.lifetime_millis != 10_000
                    || request.validate().is_err()
                {
                    return Ok(false);
                }
                match write_fluid(tx, &fence, request, Some(splash)).await {
                    Ok(ItemFluidOutcome::Written) => {
                        record_pour(tx, request.transaction_id, target, lifetime).await
                    }
                    Err(ItemUseError::Unavailable(error)) => Err(error),
                    _ => Ok(false),
                }
            }
        }
    }
}

/// Checked at the end of the transaction as well as at receipt insertion.
pub(super) async fn crack_still_primed(
    tx: &mut Transaction<'_, Postgres>,
    command: CommandRef,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM game_nomads_ritual_gestures pour
           JOIN game_nomads_ritual_gestures crack ON crack.world_id=pour.world_id
             AND crack.channel_id=pour.channel_id AND crack.map_digest=pour.map_digest
             AND crack.reset_epoch=pour.reset_epoch
          WHERE pour.game_session_id=encode($1,'hex')::uuid AND pour.command_id=$2::text::numeric
            AND crack.move_transaction_id IS NOT NULL AND crack.expires_at>clock_timestamp())",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .fetch_one(&mut **tx)
    .await?)
}

pub(super) fn crack_cell() -> [u8; 10] {
    let pos = nomads_ritual::CRACK;
    let mut bytes = [0; 10];
    bytes[..4].copy_from_slice(&i32::from(pos.x).to_be_bytes());
    bytes[4..8].copy_from_slice(&i32::from(pos.y).to_be_bytes());
    bytes[8..].copy_from_slice(&i16::from(pos.floor).to_be_bytes());
    bytes
}

pub fn is_water_placement(placement: &[u8]) -> bool {
    let water = nomads_ritual::WATER;
    oteryn_world_bundle::bundle::placement_key(water.floor, water.x, water.y, 0)
        .is_some_and(|key| placement.len() == 40 && placement[32..] == key.to_be_bytes())
}

async fn record_drop(
    tx: &mut Transaction<'_, Postgres>,
    transaction_id: [u8; 16],
    moved: [u8; 16],
    target: NomadsRitualTarget,
    lifetime: i64,
) -> Result<bool, DurabilityError> {
    let inserted = sqlx::query(
        "INSERT INTO game_nomads_ritual_gestures(
            game_session_id,command_id,character_id,world_id,channel_id,item_instance_id,
            move_transaction_id,map_digest,reset_epoch,expires_at)
         SELECT r.game_session_id,r.command_id,r.character_id,r.world_id,r.channel_id,l.item_instance_id,
                r.transaction_id,$2,$3::text::numeric,$4::bigint * interval '1 millisecond' + clock_timestamp()
           FROM game_item_loop_move_receipts r JOIN game_item_loop_move_lines l USING(transaction_id)
          WHERE r.transaction_id=encode($1,'hex')::uuid AND l.item_instance_id=encode($5,'hex')::uuid
            AND r.created_xact_id=pg_current_xact_id()
            AND l.to_kind=3 AND l.to_spatial_position=$6
            AND l.definition_family='Item' AND l.definition_production_key=$7",
    )
    .bind(transaction_id.as_slice())
    .bind(target.map_digest.as_slice())
    .bind(target.reset_epoch.to_string())
    .bind(lifetime)
    .bind(moved.as_slice())
    .bind(crack_cell().as_slice())
    .bind(nomads_ritual::SCIMITAR)
    .execute(&mut **tx)
    .await?;
    Ok(inserted.rows_affected() == 1)
}

async fn record_pour(
    tx: &mut Transaction<'_, Postgres>,
    transaction_id: [u8; 16],
    target: NomadsRitualTarget,
    lifetime: i64,
) -> Result<bool, DurabilityError> {
    let inserted = sqlx::query(
        "INSERT INTO game_nomads_ritual_gestures(
            game_session_id,command_id,character_id,world_id,channel_id,item_instance_id,
            fluid_transaction_id,map_digest,reset_epoch,expires_at)
         SELECT t.game_session_id,t.command_id,t.character_id,s.world_id,s.channel_id,t.item_instance_id,
                t.transaction_id,$2,$3::text::numeric,$4::bigint * interval '1 millisecond' + clock_timestamp()
           FROM game_item_fluid_splashes s
           JOIN game_item_fluid_transitions t USING(transaction_id,item_instance_id)
          WHERE s.transaction_id=encode($1,'hex')::uuid AND t.created_xact_id=pg_current_xact_id()
            AND t.fluid_before=$5 AND t.fluid_after=0 AND s.fluid_subtype=t.fluid_before
            AND EXISTS (SELECT 1 FROM game_nomads_ritual_gestures crack
              WHERE crack.world_id=s.world_id AND crack.channel_id=s.channel_id
                AND crack.map_digest=$2 AND crack.reset_epoch=$3::text::numeric
                AND crack.move_transaction_id IS NOT NULL AND crack.expires_at>clock_timestamp())",
    )
    .bind(transaction_id.as_slice())
    .bind(target.map_digest.as_slice())
    .bind(target.reset_epoch.to_string())
    .bind(lifetime)
    .bind(nomads_ritual::WATER_SUBTYPE)
    .execute(&mut **tx)
    .await?;
    Ok(inserted.rows_affected() == 1)
}

/// The original command's commit and a disconnected receipt read serialize
/// here, including when a cancelled database connection is still committing.
pub(super) async fn lock_command(
    tx: &mut Transaction<'_, Postgres>,
    command: CommandRef,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:nomads-command:'||encode($1,'hex')||':'||$2,0))")
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

fn decode_receipt(
    row: PgRow,
    sampled: std::time::Instant,
) -> Result<NomadsRitualReceipt, DurabilityError> {
    Ok(NomadsRitualReceipt {
        target: NomadsRitualTarget {
            map_digest: row
                .try_get::<Vec<u8>, _>("map_digest")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            reset_epoch: row
                .try_get::<String, _>("reset_epoch")?
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        },
        gesture: if row.try_get("drop")? {
            NomadsRitualGesture::Drop
        } else {
            NomadsRitualGesture::Pour
        },
        item: row
            .try_get::<Vec<u8>, _>("item")?
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        deadline: sampled
            .checked_add(std::time::Duration::from_micros(
                u64::try_from(row.try_get::<i64, _>("remaining_micros")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            ))
            .ok_or(DurabilityError::InvalidStoredState)?,
    })
}

impl DurabilityRoot {
    pub async fn nomads_ritual_gesture(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
    ) -> Result<Option<NomadsRitualReceipt>, DurabilityError> {
        if command.game_session_id() != fence.game_session_id {
            return Err(DurabilityError::InvalidStoredState);
        }
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Ok(None);
        };
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_command(&mut tx, command).await?;
                    let row = sqlx::query(
                        "SELECT map_digest,reset_epoch::text,move_transaction_id IS NOT NULL AS drop,
                                uuid_send(item_instance_id) AS item,
                                greatest(0,extract(epoch FROM expires_at-clock_timestamp()) * 1000000)::bigint AS remaining_micros
                           FROM game_nomads_ritual_gestures
                          WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric
                            AND character_id=encode($3,'hex')::uuid AND world_id=encode($4,'hex')::uuid
                            AND channel_id=encode($5,'hex')::uuid"
                    )
                        .bind(command.game_session_id().as_bytes().as_slice())
                        .bind(command.command_id().get().to_string())
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(world_id.as_bytes().as_slice())
                        .bind(channel_id.as_bytes().as_slice())
                        .fetch_optional(&mut *tx)
                        .await?;
                    let receipt = row.map(|row| decode_receipt(row, std::time::Instant::now())).transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(receipt)
                })
            })
            .await
    }

    pub async fn read_nomads_fluid_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        item: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        if command.game_session_id() != fence.game_session_id {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_command(&mut tx, command).await?;
            let committed = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_item_fluid_transitions
                    WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric
                      AND character_id=encode($3,'hex')::uuid AND item_instance_id=encode($4,'hex')::uuid
                      AND fluid_before=$5 AND fluid_after=0)",
            )
            .bind(command.game_session_id().as_bytes().as_slice())
            .bind(command.command_id().get().to_string())
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(item.as_slice())
            .bind(nomads_ritual::WATER_SUBTYPE)
            .fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(committed)
        })).await
    }

    pub async fn active_nomads_rituals(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        permit: &SpellLanePermit,
        scope: RuntimeScopeRefV1,
        target: NomadsRitualTarget,
    ) -> Result<Vec<NomadsRitualReceipt>, DurabilityError> {
        permit.check_scope(scope)?;
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = scope
        else {
            return Ok(Vec::new());
        };
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                let rows = sqlx::query(
                    "WITH sampled AS MATERIALIZED (SELECT clock_timestamp() AS at)
                     SELECT map_digest,reset_epoch::text,move_transaction_id IS NOT NULL AS drop,
                            uuid_send(item_instance_id) AS item,
                            greatest(0,extract(epoch FROM expires_at-sampled.at) * 1000000)::bigint AS remaining_micros
                       FROM game_nomads_ritual_gestures CROSS JOIN sampled
                      WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid
                        AND map_digest=$3 AND reset_epoch=$4::text::numeric AND expires_at>sampled.at
                      ORDER BY (move_transaction_id IS NULL),expires_at DESC LIMIT 2"
                )
                    .bind(world_id.as_bytes().as_slice())
                    .bind(channel_id.as_bytes().as_slice())
                    .bind(target.map_digest.as_slice())
                    .bind(target.reset_epoch.to_string())
                    .fetch_all(&mut *tx).await?;
                let sampled = std::time::Instant::now();
                let receipts = rows.into_iter().map(|row| decode_receipt(row, sampled)).collect::<Result<_, _>>()?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(receipts)
            })).await
    }
}
