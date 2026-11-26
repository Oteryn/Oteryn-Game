//! Recovery of physical results earned by a fenced World callback payment.
use super::*;
use crate::durability::character_last_position::{
    FinalLastPosition, LastPositionCell, LastPositionKey, LastPositionWrite, load_last_position,
    upsert_last_position,
};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::world_callbacks::{NativeSelector, SceneEdit};
use sqlx::types::Json;

pub type PaidScroll = (CommandRef, NativeSelector, Vec<SceneEdit>);
type PaidScrollRow = (
    String,
    String,
    String,
    Json<NativeSelector>,
    Json<Vec<SceneEdit>>,
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaidArrival {
    pub character: [u8; 16],
    pub lease_generation: u64,
    pub placed: FinalLastPosition,
    pub retain_saved_vitals: bool,
}

impl PaidArrival {
    pub(super) fn matches(
        &self,
        character: [u8; 16],
        lease_generation: u64,
        world: crate::foundation::WorldId,
        frame: &[u8],
        destination: Position,
    ) -> bool {
        self.character == character
            && self.lease_generation == lease_generation
            && self.placed.record.world_id == world
            && self.placed.record.coordinate_frame_digest == frame
            && self.placed.record.cell
                == (LastPositionCell {
                    x: i32::from(destination.x),
                    y: i32::from(destination.y),
                    floor: -i16::from(destination.floor),
                })
    }

    pub(super) async fn write(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<bool, DurabilityError> {
        let mut record = self.placed.record;
        if self.retain_saved_vitals {
            let character = crate::domain::CharacterId::from_bytes(self.character)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if let Some(previous) = load_last_position(tx, character).await? {
                record.vitals = previous.record.vitals;
                record.rest = previous.record.rest;
            }
        }
        Ok(upsert_last_position(
            tx,
            &self.character,
            LastPositionKey {
                character_lease_generation: self.lease_generation,
                write_sequence: self.placed.write_sequence,
            },
            &record,
        )
        .await?
            == LastPositionWrite::Written)
    }
}

/// The exact, already prepared landings share the payment's commit. An earlier
/// position capture cannot overtake them. Fresh admission uses the ordinary
/// saved-position reader if the runtime is gone before it publishes the move.
pub(super) async fn write_arrivals(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    context: &QuestWorldToolContext,
) -> Result<bool, DurabilityError> {
    let arrivals = &context.paid_arrivals;
    let world = fence.runtime_scope.world_id();
    let frame = &context.placement_context;
    if let Some(party) = &callback.effects.party_transfer {
        if arrivals.len() != party.stations.len()
            || !arrivals
                .iter()
                .zip(&context.party)
                .zip(&party.stations)
                .all(|((arrival, member), station)| {
                    arrival.matches(
                        member.character,
                        member.lease_generation,
                        world,
                        frame,
                        station.to,
                    )
                })
        {
            return Ok(false);
        }
    } else {
        let target = matches!(arrivals.as_slice(), [arrival] if callback.teleport
        .into_iter()
        .chain(callback.remote_teleport.iter().map(|remote| remote.destination))
        .any(|to| arrival.matches(
            *fence.character_id.as_bytes(), fence.character_lease_generation, world, frame, to,
        )));
        if !target && !(arrivals.is_empty() && callback.teleport.is_none()) {
            return Ok(false);
        }
    }
    let mut ordered: Vec<_> = arrivals.iter().collect();
    ordered.sort_unstable_by_key(|arrival| arrival.character);
    for arrival in ordered {
        if !arrival.write(tx).await? {
            return Ok(false);
        }
    }
    Ok(true)
}

impl super::super::DurabilityRoot {
    /// The Dread Lord offering already seals the exact scene in its immutable
    /// consumption proof. Read that proof under the successor Channel's fence.
    #[allow(
        clippy::too_many_arguments,
        reason = "the successor's scope, node, and pinned world context are independent"
    )]
    pub async fn read_paid_scroll_scenes(
        &self,
        authority: &super::super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &super::super::runtime_scope_assignment::NodeIncarnationProof,
        permit: &SpellLanePermit,
        scope: crate::foundation::RuntimeScopeRefV1,
        generation: u64,
        map_revision: String,
        frame: [u8; 32],
    ) -> Result<Vec<PaidScroll>, DurabilityError> {
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = scope
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut permit = permit;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut permit, move |holder, deadline, permit| {
                Box::pin(async move {
                    let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
                    super::super::spell_item_transaction::assert_spell_item_scope_with_recovery(
                        &mut tx, permit, &recovery, &node, scope, generation,
                    ).await.map_err(|_| DurabilityError::Unavailable)?;
                    let mut scenes = Vec::new();
                    let mut cursor: Option<String> = None;
                    loop {
                        let rows: Vec<PaidScrollRow> = sqlx::query_as(
                            "SELECT transaction_id::text, cause_id::text, cause_ordinal::text,
                                    paid.proof_json #> '{callback,selector,native}',
                                    paid.proof_json #> '{callback,effects,scene,open}'
                               FROM game_quest_callback_consumptions
                               CROSS JOIN LATERAL (SELECT convert_from(proof,'UTF8')::jsonb AS proof_json) paid
                              WHERE transition_key='oteryn:quest-callback/dreamer-dread-lord-scroll'
                                AND location_kind='ground'
                                AND (before_ground->>'world_id')=encode($1,'hex')::uuid::text
                                AND (before_ground->>'channel_id')=encode($2,'hex')::uuid::text
                                AND (before_ground->>'map_revision')=$3
                                AND (before_ground->>'native_room_placement_context')=(to_jsonb($4::bytea)#>>'{}')
                                AND paid.proof_json #> '{callback,effects,scene,open}' IS NOT NULL
                                AND ($5::uuid IS NULL OR transaction_id > $5::uuid)
                              ORDER BY transaction_id LIMIT 128",
                        )
                        .bind(world_id.as_bytes().as_slice())
                        .bind(channel_id.as_bytes().as_slice())
                        .bind(&map_revision)
                        .bind(frame.as_slice())
                        .bind(cursor.as_deref())
                        .fetch_all(&mut *tx)
                        .await?;
                        if rows.is_empty() {
                            break;
                        }
                        for (id, session, ordinal, Json(source), Json(edits)) in rows {
                            let session = GameSessionId::decode(&super::super::item_mint::uuid_text(&session)?)
                                .map_err(|_| DurabilityError::InvalidStoredState)?;
                            let command = ordinal.parse::<u64>().ok()
                                .and_then(|id| CommandId::new(id).ok())
                                .ok_or(DurabilityError::InvalidStoredState)?;
                            scenes.push((CommandRef::new(session, command), source, edits));
                            cursor = Some(id);
                        }
                    }
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(scenes)
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn all_carried_steps_and_winterlight_ingredients_hold_release() -> Result<(), String> {
        use oteryn_gameplay_quests::quest::{carried_step, world_callbacks};

        let callbacks = world_callbacks::load()?;
        for recipe in carried_step::RECIPES {
            let callback = callbacks
                .iter()
                .find(|callback| callback.key == recipe.key)
                .ok_or_else(|| format!("missing carried STEP {}", recipe.key))?;
            assert!(super::may_debit(callback), "{}", callback.key);
        }
        let winterlight = callbacks
            .iter()
            .filter(|callback| {
                callback.key.ends_with("winterlight-disguise")
                    || callback.key.contains("winterlight-fire-")
            })
            .collect::<Vec<_>>();
        assert_eq!(winterlight.len(), 5);
        for callback in winterlight {
            assert!(
                !callback.effects.carried_inputs.is_empty(),
                "{}",
                callback.key
            );
            assert!(super::may_debit(callback), "{}", callback.key);
        }
        Ok(())
    }
}
