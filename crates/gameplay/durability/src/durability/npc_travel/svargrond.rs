//! Source-bound Buddel and Iskan travel policies. Crystal 201189f8,
//! data-global/npc/buddel.lua:54-126 and data-global/npc/iskan.lua:119-127.
use super::*;
use crate::durability::item_mint_audit::Type2Transaction;
use oteryn_gameplay_content::content::{
    ProjectV2TravelDestination, ProjectV2TravelRoute, npc_catalogue::travel::TravelRoute,
};

pub const BUDDEL: &str = "oteryn:npc.buddel";
pub const ISKAN: &str = "oteryn:npc.iskan";
const BUDDEL_SERVICE: &str = "oteryn:service.travel.buddel";
const ISKAN_SERVICE: &str = "oteryn:service.travel.iskan";
pub const RING: &str = "oteryn:item.tibia.i3097";
const BARBARIAN: &str = oteryn_gameplay_quests::quest::barbarian_mead::TRACKS[0];
const ICE_ISLANDS: &str = oteryn_gameplay_quests::quest::ice_islands_seals::QUESTLINE;
pub const HUSKY_KILL: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/husky_kill";
pub const DESTINATIONS: [(&str, i32, i32); 4] = [
    ("okolnir", 32225, 31381),
    ("helheim", 32462, 31174),
    ("tyrsung", 32333, 31227),
    ("camp", 32021, 31294),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Payment {
    Fare,
    DeclinedRing,
    Ring,
}

pub fn route(npc: &str, keyword: &str, payment: Payment) -> Option<TravelRoute> {
    let (service, x, y) = match npc {
        BUDDEL => {
            let (_, x, y) = DESTINATIONS
                .into_iter()
                .find(|(key, _, _)| *key == keyword)?;
            (BUDDEL_SERVICE, x, y)
        }
        ISKAN if keyword == "passage" => (ISKAN_SERVICE, 32325, 31049),
        _ => return None,
    };
    let suffix = match payment {
        Payment::Fare => "",
        Payment::DeclinedRing => "_declined_ring",
        Payment::Ring => "_ring",
    };
    let fare = npc == BUDDEL && payment != Payment::Ring;
    Some(TravelRoute {
        service: service.into(),
        route: ProjectV2TravelRoute {
            key: format!("{keyword}{suffix}"),
            destination: ProjectV2TravelDestination {
                coordinate_frame: "global-target-2026-09-27".into(),
                x,
                y,
                floor: 7,
            },
            price: if fare { 50 } else { 0 },
            premium: false,
            min_level: None,
        },
        keyword: keyword.into(),
        postman: fare,
        carpet: false,
        held: false,
        yalahar_gate: false,
        yalahar_town_gate: None,
        quest_gate: None,
        quest_clear_gate: None,
        ticket: None,
        return_origin: None,
        world_change: None,
    })
}

pub fn pays_ring(route: &TravelRoute) -> bool {
    route.service == BUDDEL_SERVICE
        && route.route.key.strip_suffix("_ring") == Some(route.keyword.as_str())
}

async fn track(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    key: &str,
) -> Result<i64, DurabilityError> {
    let found = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(key).fetch_optional(&mut ***tx).await?;
    Ok(found.unwrap_or(-1))
}

pub(super) async fn permitted(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    route: &TravelRoute,
) -> Result<bool, DurabilityError> {
    if route.service == ISKAN_SERVICE {
        return Ok(
            self::route(ISKAN, &route.keyword, Payment::Fare).as_ref() == Some(route)
                && track(tx, fence, BARBARIAN).await? >= 8
                && track(tx, fence, ICE_ISLANDS).await? >= 3
                && track(tx, fence, HUSKY_KILL).await? < 1,
        );
    }
    if route.service != BUDDEL_SERVICE {
        return Ok(true);
    }
    let payment = if pays_ring(route) {
        Payment::Ring
    } else if route.route.key.strip_suffix("_declined_ring") == Some(route.keyword.as_str()) {
        Payment::DeclinedRing
    } else {
        Payment::Fare
    };
    let Some(mut expected) = self::route(BUDDEL, &route.keyword, payment) else {
        return Ok(false);
    };
    let to = &route.route.destination;
    if payment != Payment::Ring && DESTINATIONS.iter().any(|&(_, x, y)| (x, y) == (to.x, to.y)) {
        expected.route.destination.x = to.x;
        expected.route.destination.y = to.y;
    }
    if &expected != route {
        return Ok(false);
    }
    if route.keyword == "helheim" && track(tx, fence, ICE_ISLANDS).await? < 30 {
        return Ok(false);
    }
    if payment == Payment::Fare {
        return Ok(track(tx, fence, BARBARIAN).await? == 8);
    }
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
           JOIN game_item_instances i ON i.item_instance_id=c.item
           WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$3)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(RING)
    .fetch_one(&mut ***tx)
    .await
    .map_err(Into::into)
}

/// Take one ring before issuing the corresponding travel receipt.
/// Crystal's Player::removeItemOfType walks carried containers and includes equipment.
pub(super) async fn exchange_ring(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    command: crate::foundation::CommandRef,
    occurrence: super::TravelOccurrence,
    binding: &[u8],
) -> Result<bool, DurabilityError> {
    let character = fence.character_id.as_bytes();
    let world = fence.runtime_scope.world_id();
    let source = sqlx::query(
        "SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,c.location,c.kind
         FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
         JOIN game_item_instances i ON i.item_instance_id=c.item
         WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$3
         ORDER BY CASE c.kind WHEN 'container' THEN 0 WHEN 'nested' THEN 1 ELSE 2 END,
           (c.location->>'placement_ordinal')::numeric, (c.location->>'slot')::smallint, i.item_instance_id LIMIT 1 FOR UPDATE OF i",
    )
    .bind(character.as_slice())
    .bind(world.as_bytes().as_slice())
    .bind(RING)
    .fetch_optional(&mut ***tx)
    .await?;
    let Some(source) = source else {
        return Ok(false);
    };
    let item: String = source.try_get("item")?;
    let before: serde_json::Value = source.try_get("before_item")?;
    let location: serde_json::Value = source.try_get("location")?;
    let kind: String = source.try_get("kind")?;
    let equipment_revision: Option<String> = if kind == "equipment" {
        Some(sqlx::query_scalar(
            "SELECT revision::text FROM game_character_equipment_state
             WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
        )
        .bind(character.as_slice())
        .fetch_one(&mut ***tx)
        .await?)
    } else {
        None
    };
    let transaction: String = sqlx::query_scalar(
        "INSERT INTO game_quest_callback_consumptions(transaction_id,item_instance_id,character_id,
           cause_id,cause_ordinal,quantity,before_item,before_location,location_kind,proof,proof_sha256,
           travel_occurrence,travel_equipment_revision)
         SELECT game_character_uuid_v7(),$1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,
           $4::text::numeric,1,$5,$6,$7,p.proof,sha256(p.proof),encode($9,'hex')::uuid,$10::text::numeric
         FROM (SELECT convert_to(jsonb_build_object('travel_binding',encode($8,'hex'))::text,'UTF8') AS proof) p
         RETURNING transaction_id::text",
    )
    .bind(&item)
    .bind(character.as_slice())
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .bind(before)
    .bind(location)
    .bind(&kind)
    .bind(binding)
    .bind(occurrence.as_bytes().as_slice())
    .bind(equipment_revision)
    .fetch_one(&mut ***tx)
    .await?;
    super::super::quest_world_callback::retire_carried_item(tx, &item, &kind, &transaction)
        .await?;
    if kind == "equipment" {
        sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=$2::uuid WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_slice()).bind(transaction).execute(&mut ***tx).await?;
    }
    Ok(true)
}
