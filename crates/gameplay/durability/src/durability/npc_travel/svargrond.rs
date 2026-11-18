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
        Payment::DeclinedRing => "-declined-ring",
        Payment::Ring => "-ring",
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
    route.service == BUDDEL_SERVICE && route.route.key.strip_suffix("-ring") == Some(route.keyword.as_str())
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
        return Ok(self::route(ISKAN, &route.keyword, Payment::Fare).as_ref() == Some(route)
            && track(tx, fence, BARBARIAN).await? >= 8
            && track(tx, fence, ICE_ISLANDS).await? >= 3
            && track(tx, fence, HUSKY_KILL).await? < 1);
    }
    if route.service != BUDDEL_SERVICE {
        return Ok(true);
    }
    let payment = if pays_ring(route) {
        Payment::Ring
    } else if route.route.key.strip_suffix("-declined-ring") == Some(route.keyword.as_str()) {
        Payment::DeclinedRing
    } else {
        Payment::Fare
    };
    let Some(mut expected) = self::route(BUDDEL, &route.keyword, payment) else {
        return Ok(false);
    };
    let to = &route.route.destination;
    if payment != Payment::Ring
        && DESTINATIONS.iter().any(|&(_, x, y)| (x, y) == (to.x, to.y))
    {
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
        "SELECT EXISTS(SELECT 1 FROM game_item_container_slots s
           JOIN game_item_container_entries e ON e.character_id=s.character_id
             AND e.parent_item_instance_id=s.item_instance_id AND e.world_id=s.world_id
           JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id
           WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid
             AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$3)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(RING)
    .fetch_one(&mut ***tx)
    .await
    .map_err(Into::into)
}
