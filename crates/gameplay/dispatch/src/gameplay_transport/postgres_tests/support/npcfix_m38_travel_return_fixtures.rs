use super::fixture::debug;

use super::npcfix_m38_travel_guards_fixtures::NoPremium;

use super::{ComposedFreshAdmission, Dependencies, TestResult};

use crate::content::npc_catalogue::travel::{TravelReturnRequirement, TravelRoute};

use crate::durability::item_fee_burn::{FeeBurnCause, FeeBurnRequest, FeeChangeFacts};

use crate::durability::item_mint::TypedDefinitionRef;

use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};

use crate::durability::npc_travel::{ArrivalConsume, PaidTravelRequest, TravelOccurrence};

use crate::foundation::{CommandId, CommandRef, GameSessionId};

pub(super) const A: &str = "oteryn:service.travel.m38_origin_a/travora";

pub(super) const B: &str = "oteryn:service.travel.m38_origin_b/travora";

pub(super) fn requirement(expected: &str) -> TravelReturnRequirement {
    TravelReturnRequirement {
        inbound_routes: vec![A.into(), B.into()],
        return_from: expected.into(),
    }
}

pub(super) async fn request(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    command: u64,
    service: &str,
) -> TestResult<PaidTravelRequest> {
    let runtime = owner.runtime.lock().await;
    let snapshot = runtime.read_actor_position(deps.actor).map_err(debug)?;
    let cell = snapshot.position();
    let session = deps.admitted.game_session_id;
    let derive = |slot: u8| {
        crate::gameplay_transport::npc_trade_command::derived(
            format!("npcfix:m38-return:{slot}").as_bytes(),
            session,
            command,
        )
    };
    let occurrence = TravelOccurrence::from_bytes(derive(0)).map_err(debug)?;
    let service = format!("oteryn:service.travel.{service}");
    let definition = |key: &str| TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    };
    Ok(PaidTravelRequest {
        command: CommandRef::new(session, CommandId::new(command).map_err(debug)?),
        arrival: crate::durability::npc_travel::PendingArrival {
            occurrence,
            world_id: owner.world_id,
            destination: crate::durability::character_last_position::LastPositionCell {
                x: cell.x,
                y: cell.y,
                floor: cell.floor,
            },
        },
        route: TravelRoute {
            service: service.clone(),
            route: crate::content::ProjectV2TravelRoute {
                key: "travora".into(),
                destination: crate::content::ProjectV2TravelDestination {
                    coordinate_frame: "global-target-2026-09-27".into(),
                    x: cell.x,
                    y: cell.y,
                    floor: cell.floor,
                },
                price: 0,
                premium: false,
                min_level: None,
                required_quest_tracks: Vec::new(),
            },
            keyword: "travora".into(),
            postman: false,
            carpet: false,
            held: false,
            yalahar_gate: false,
            yalahar_town_gate: None,
            quest_gate: None,
            quest_clear_gate: None,
            ticket: None,
            return_origin: None,
            world_change: None,
        },
        fee: FeeBurnRequest {
            cause: FeeBurnCause::NpcTravel {
                service,
                occurrence: occurrence.as_bytes(),
            },
            fee_gold_units: 0,
            transaction_id: derive(1),
            event_id: derive(2),
            occurred_at_unix_ms: crate::gameplay_transport::npc_trade_command::unix_ms_now(),
            server_build_id: "npcfix-origin-fixture".into(),
            // Synthetic zero-fare fixtures exercise the travel owner without a MINT or debit.
            change: FeeChangeFacts {
                item_instance_ids: [derive(3), derive(4)],
                gold: definition("oteryn:item.tibia.i3031"),
                platinum: definition("oteryn:item.tibia.i3035"),
                crystal: definition("oteryn:item.tibia.i3043"),
                backpack: ItemDefinitionFacts {
                    weight: None,
                    definition: definition("oteryn:item.tibia.i2854"),
                    stack: ItemStackClass::NonStackable,
                    container_capacity: Some(20),
                    container_slot_equip_pattern: true,
                },
            },
        },
    })
}

pub(super) async fn commit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    request: PaidTravelRequest,
) -> TestResult {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_npc_travel(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request,
            &NoPremium,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, Ok(Some(_))), "travel commit: {result:?}");
    Ok(())
}

pub(super) async fn complete(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    occurrence: TravelOccurrence,
) -> TestResult {
    let placed = {
        let runtime = owner.runtime.lock().await;
        let snapshot = runtime.read_actor_position(deps.actor).map_err(debug)?;
        owner.placed_record(&runtime, snapshot.position())
    };
    let fence = owner
        .current_quest_fence(deps.admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    assert_eq!(
        owner
            .root
            .consume_npc_pending_arrival_placed(
                owner.character,
                owner.holder,
                fence,
                occurrence,
                placed
            )
            .await
            .map_err(debug)?,
        ArrivalConsume::Consumed
    );
    Ok(())
}
