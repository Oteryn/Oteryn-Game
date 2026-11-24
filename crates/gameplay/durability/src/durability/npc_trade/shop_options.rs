//! Extra custody for negotiated physical NPC shop options.
use super::*;
use std::collections::BTreeMap;

pub const SHOPPING_BAG_KEY: &str = "oteryn:item.tibia.i21411";
// Crystal 201189f8 npc_functions.cpp:621–654 charges 20 gold for each bag.
pub(in crate::durability) const BAG_PRICE: u64 = 20;

pub(super) fn validate(request: &NpcTradeRequest, sell_all: bool) -> Result<()> {
    if request.include_equipped && (!sell_all || request.cause.side != NpcTradeSide::Sell) {
        return Err(NpcTradeError::InvalidInput);
    }
    if let Some(bag) = &request.shopping_bag
        && (request.cause.side != NpcTradeSide::Buy
            || sell_all
            || bag.definition.family != "Item"
            || bag.definition.production_key != SHOPPING_BAG_KEY
            || bag.stack != super::super::item_transfer::ItemStackClass::NonStackable
            || bag.container_capacity != Some(20)
            || bag.weight.is_none())
    {
        return Err(NpcTradeError::InvalidInput);
    }
    Ok(())
}

pub(super) fn total(request: &NpcTradeRequest, price: u64) -> Result<u64> {
    price
        .checked_add(if request.shopping_bag.is_some() { BAG_PRICE } else { 0 })
        .filter(|total| *total <= rules::NPC_TRADE_BUY_GOLD_UNITS_MAX)
        .ok_or(NpcTradeError::InvalidInput)
}

#[derive(Default)]
pub(super) struct Materialized {
    pub(super) bag_id: Option<[u8; 16]>,
    pub(super) equipment: BTreeMap<[u8; 16], i16>,
    pub(super) equipment_revision: Option<String>,
}

impl Materialized {
    pub(super) async fn resolve(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        fence: &CurrentCharacterItemFence,
        request: &NpcTradeRequest,
        entries: &mut Vec<Entry>,
    ) -> std::result::Result<Self, DurabilityError> {
        let mut shop = Self::default();
        if request.shopping_bag.is_some() {
            let hash = Sha256::new()
                .chain_update(b"oteryn:npc-shopping-bag:v1")
                .chain_update(request.transaction_id)
                .finalize();
            let mut id = [0; 16];
            id.copy_from_slice(&hash[..16]);
            id[6] = (id[6] & 15) | 112;
            id[8] = (id[8] & 63) | 128;
            shop.bag_id = Some(id);
        }
        if !request.include_equipped {
            return Ok(shop);
        }
        let character = fence.character_id.as_bytes().as_slice();
        let revision = sqlx::query_scalar::<_, String>(
            "SELECT revision::text FROM game_character_equipment_state
             WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
        )
        .bind(character)
        .fetch_optional(&mut **tx)
        .await?;
        let definition = &request.item.definition;
        let rows = sqlx::query(
            "SELECT i.item_instance_id::text, i.quantity, s.slot
               FROM game_character_equipment_slots s
               JOIN game_item_instances i USING(item_instance_id)
              WHERE s.character_id=encode($1,'hex')::uuid
                AND s.world_id=encode($2,'hex')::uuid AND i.world_id=s.world_id
                AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=($3,$4,$5)
                AND i.lifecycle=1
                AND NOT EXISTS (SELECT 1 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
                AND NOT EXISTS (SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
                AND NOT EXISTS (SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id)
                AND NOT EXISTS (SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=i.item_instance_id)
                AND NOT EXISTS (SELECT 1 FROM game_item_tiers t WHERE t.item_instance_id=i.item_instance_id)
                AND NOT EXISTS (SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id)
                AND NOT EXISTS (SELECT 1 FROM game_item_texts x WHERE x.item_instance_id=i.item_instance_id)
                AND COALESCE((SELECT f.fluid_subtype FROM game_item_fluid_states f
                               WHERE f.item_instance_id=i.item_instance_id),0)=$6
              ORDER BY s.slot FOR UPDATE OF i",
        )
        .bind(character)
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(&definition.family)
        .bind(&definition.production_key)
        .bind(&definition.revision_ref)
        .bind(i16::from(request.fluid_subtype.unwrap_or(0)))
        .fetch_all(&mut **tx)
        .await?;
        if rows.is_empty() {
            return Ok(shop);
        }
        shop.equipment_revision = Some(revision.ok_or(DurabilityError::InvalidStoredState)?);
        let mut ordinal = entries.first().map_or(0, |entry| entry.placement_ordinal);
        let mut equipped = Vec::with_capacity(rows.len());
        for row in &rows {
            let id = uuid_text(&row.try_get::<String, _>("item_instance_id")?)?;
            ordinal = ordinal.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            shop.equipment.insert(id, row.try_get("slot")?);
            equipped.push(Entry {
                item_instance_id: id,
                placement_ordinal: ordinal,
                definition: definition.clone(),
                quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                live: true,
                not_default: false,
            });
        }
        equipped.reverse();
        equipped.append(entries);
        *entries = equipped;
        Ok(shop)
    }

    pub(super) async fn insert_bag(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        (world, character): (WorldId, CharacterId),
        parent: &[u8; 16],
        request: &NpcTradeRequest,
        plan: &Plan,
    ) -> std::result::Result<(), DurabilityError> {
        let (Some(id), Some(bag)) = (self.bag_id, &request.shopping_bag) else {
            return Ok(());
        };
        let ordinal = first_item_mint(plan)?.placement_ordinal;
        sqlx::query(
            "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
                definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)",
        )
        .bind(id.as_slice())
        .bind(world.as_bytes().as_slice())
        .bind(SHOPPING_BAG_KEY)
        .bind(&bag.definition.revision_ref)
        .bind(request.transaction_id.as_slice())
        .execute(&mut **tx)
        .await?;
        sqlx::query(
            "INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,
                parent_item_instance_id,placement_ordinal,placed_transaction_id)
             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,
                encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)",
        )
        .bind(id.as_slice())
        .bind(world.as_bytes().as_slice())
        .bind(character.as_bytes().as_slice())
        .bind(parent.as_slice())
        .bind(ordinal.to_string())
        .bind(request.transaction_id.as_slice())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(super) async fn remove_equipment(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        fence: &CurrentCharacterItemFence,
        line: &NpcTradeLine,
    ) -> std::result::Result<(), DurabilityError> {
        let slot = self.equipment.get(&line.item_instance_id).copied()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let changed = sqlx::query(
            "DELETE FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid
               AND slot=$2 AND item_instance_id=encode($3,'hex')::uuid",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(slot)
        .bind(line.item_instance_id.as_slice())
        .execute(&mut **tx)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(())
    }

    pub(super) async fn bump_equipment_revision(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        fence: &CurrentCharacterItemFence,
        request: &NpcTradeRequest,
    ) -> std::result::Result<(), DurabilityError> {
        let Some(revision) = &self.equipment_revision else {
            return Ok(());
        };
        let changed = sqlx::query(
            "UPDATE game_character_equipment_state SET revision=revision+1,
                last_transaction_id=encode($1,'hex')::uuid
              WHERE character_id=encode($2,'hex')::uuid AND revision=$3::text::numeric",
        )
        .bind(request.transaction_id.as_slice())
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(revision)
        .execute(&mut **tx)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(())
    }
}

fn first_item_mint(plan: &Plan) -> std::result::Result<&NpcTradeLine, DurabilityError> {
    plan.lines.iter().find(|line| line.kind == NpcTradeLineKind::ItemMint)
        .ok_or(DurabilityError::InvalidStoredState)
}

pub(super) async fn insert_nested(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    world: WorldId,
    bag: &[u8; 16],
    request: &NpcTradeRequest,
    plan: &Plan,
    line: &NpcTradeLine,
) -> std::result::Result<(), DurabilityError> {
    let ordinal = line.placement_ordinal.checked_sub(first_item_mint(plan)?.placement_ordinal)
        .and_then(|offset| offset.checked_add(1))
        .ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query(
        "INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,
             placement_ordinal,placed_transaction_id)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,
             $4::text::numeric,encode($5,'hex')::uuid)",
    )
    .bind(line.item_instance_id.as_slice())
    .bind(world.as_bytes().as_slice())
    .bind(bag.as_slice())
    .bind(ordinal.to_string())
    .bind(request.transaction_id.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}
