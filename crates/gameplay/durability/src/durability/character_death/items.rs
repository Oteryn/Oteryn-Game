//! Immutable item selection in the fenced death transaction (DEATH-COMPLETE0 §4.3).
use super::*;
use crate::domain::death::{EquipmentSlot, EquippedItem, PveDeathInput, calculate_pve_death};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeathItemSelection {
    pub root: [u8; 32],
    pub amulet: Option<[u8; 16]>,
    pub lost: Vec<[u8; 16]>,
    pub occurred_at: i64,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub simulation_revision: String,
}

pub(super) fn uuid(id: [u8; 16]) -> String {
    // PostgreSQL accepts the compact UUID spelling.
    id.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn binding(base: &[u8], items: &DeathItemSelection) -> Vec<u8> {
    let mut hash = Sha256::new();
    hash.update(b"oteryn.death.item_selection.v1");
    hash.update(base);
    hash.update(items.root);
    hash.update([u8::from(items.amulet.is_some())]);
    if let Some(id) = items.amulet {
        hash.update(id);
    }
    hash.update((items.lost.len() as u64).to_be_bytes());
    for id in &items.lost {
        hash.update(id);
    }
    let mut binding = base.to_vec();
    binding.extend_from_slice(&hash.finalize());
    binding
}

pub(super) fn decode(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<Option<DeathItemSelection>, DurabilityError> {
    let root: Option<Vec<u8>> = row.try_get("item_selection_root")?;
    let Some(root) = root else {
        return Ok(None);
    };
    Ok(Some(DeathItemSelection {
        root: root
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        amulet: row
            .try_get::<Option<String>, _>("amulet_of_loss_item_id")?
            .map(|s| uuid_text(&s))
            .transpose()?,
        lost: row
            .try_get::<Vec<String>, _>("lost_item_ids_text")?
            .iter()
            .map(|s| uuid_text(s))
            .collect::<std::result::Result<_, _>>()?,
        occurred_at: row.try_get("committed_at")?,
        content_revision: row.try_get("content_revision")?,
        ruleset_revision: row.try_get("ruleset_revision")?,
        simulation_revision: row.try_get("simulation_revision")?,
    }))
}

#[allow(
    clippy::too_many_arguments,
    reason = "transaction-owned death facts and canonical sampler input"
)]
pub(super) async fn select<const N: usize>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &CharacterDeathRequest<N>,
    level: u32,
    blessings: u8,
    adventurer: bool,
    skulled: bool,
    root: [u8; 32],
    occurred_at: i64,
) -> std::result::Result<DeathItemSelection, DurabilityError> {
    let vocation: Option<String> = sqlx::query_scalar(
        "SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let rows = sqlx::query("SELECT s.slot, i.item_instance_id::text, i.definition_production_key, i.definition_family FROM (SELECT slot,item_instance_id FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid UNION ALL SELECT 9::smallint,item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid) s JOIN game_item_instances i USING(item_instance_id) ORDER BY i.item_instance_id LIMIT 11 FOR UPDATE OF i")
        .bind(character.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() > 10 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut equipment = Vec::with_capacity(rows.len());
    let mut amulet = false;
    for row in rows {
        let slot = match row.try_get::<i16, _>("slot")? {
            1 => EquipmentSlot::Head,
            7 => EquipmentSlot::Necklace,
            9 => EquipmentSlot::Container,
            2 => EquipmentSlot::Armor,
            5 => EquipmentSlot::RightHand,
            6 => EquipmentSlot::LeftHand,
            3 => EquipmentSlot::Legs,
            4 => EquipmentSlot::Feet,
            8 => EquipmentSlot::Ring,
            10 => EquipmentSlot::Ammo,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        amulet |= slot == EquipmentSlot::Necklace
            && row.try_get::<String, _>("definition_family")? == "Item"
            && row.try_get::<String, _>("definition_production_key")? == "oteryn:item.tibia.i3057";
        equipment.push(EquippedItem {
            slot,
            item: uuid_text(row.try_get("item_instance_id")?)?,
        });
    }
    // The pure sampler selects an AoL even on an exempt input. Apply the accepted
    // exemption and skull rules first, so neither can accidentally consume it.
    let exempt = level <= crate::domain::death::ITEM_LOSS_EXEMPT_MAX_LEVEL
        || vocation
            .as_deref()
            .is_none_or(|v| v == super::super::character_build::NO_VOCATION)
        || adventurer;
    let (amulet, lost) = if exempt {
        (None, Vec::new())
    } else if skulled {
        equipment.sort_by_key(|e| e.slot);
        (None, equipment.iter().map(|e| e.item).collect())
    } else {
        let outcome = calculate_pve_death(
            &PveDeathInput {
                level,
                total_experience: ExactI64::new(0),
                regular_blessings: blessings,
                promotion_benefit_current: false,
                has_vocation: true,
                equipment: &equipment,
                necklace_is_amulet_of_loss: amulet,
            },
            &GameplayDecisionRoot::from_bytes(root),
            DecisionOccurrenceId::from_bytes(*request.occurrence.as_bytes()),
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        (
            outcome.amulet_of_loss_consumed,
            outcome.lost_items.into_iter().map(|e| e.item).collect(),
        )
    };
    Ok(DeathItemSelection {
        root,
        amulet,
        lost,
        occurred_at,
        content_revision: request.context.content.clone(),
        ruleset_revision: request.context.ruleset.clone(),
        simulation_revision: request.context.simulation.clone(),
    })
}

pub(super) async fn completed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: PlayerDeathOccurrence,
) -> std::result::Result<bool, DurabilityError> {
    Ok(
        sqlx::query_scalar("SELECT game_player_death_items_complete(encode($1,'hex')::uuid)")
            .bind(occurrence.as_bytes().as_slice())
            .fetch_one(&mut **tx)
            .await?,
    )
}
