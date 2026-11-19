use super::fixture::{Harness, debug};
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, classic_quests_fixtures as classic_quests,
};
use crate::content::npc_catalogue::travel::{
    ConditionalTravel, Diversion, ProgressWindow, TravelCell, TravelRoute,
};
use crate::content::{ProjectV2TravelDestination, ProjectV2TravelRoute};
use crate::gameplay_transport::{
    connection::UseCommand,
    newhaven_travel::route_draw,
    npc_talk::{NpcTalkBook, NpcTalkEntry},
};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

pub(super) const NAME: &str = "qcx62-buddel-conditional-diversion";
const NPC: &str = "oteryn:npc.buddel";
const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
const BARBARIAN: &str = "oteryn:quest-progress/quest/u8_0/barbarian_test/questline";
const ICE: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline";
pub(super) const ARRIVAL_CELLS: [Position; 4] = [
    Position {
        x: 32225,
        y: 31381,
        floor: -7,
    },
    Position {
        x: 32462,
        y: 31174,
        floor: -7,
    },
    Position {
        x: 32333,
        y: 31227,
        floor: -7,
    },
    Position {
        x: 32021,
        y: 31294,
        floor: -7,
    },
];

fn cell(at: Position) -> TravelCell {
    TravelCell {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}

pub(super) fn route_fixture(keyword: &str, target: Position, reply: &str) -> TravelRoute {
    let preferred = cell(target);
    let diversion = Diversion {
        denominator: 5,
        preferred_successes: 4,
        diversion_successes: 4,
        alternatives: ARRIVAL_CELLS.map(cell).to_vec(),
    };
    TravelRoute {
        service: "oteryn:service.travel.buddel".into(),
        route: ProjectV2TravelRoute {
            key: keyword.into(),
            destination: ProjectV2TravelDestination {
                coordinate_frame: "global-target-2026-09-27".into(),
                x: preferred.x,
                y: preferred.y,
                floor: preferred.floor,
            },
            price: 50,
            premium: false,
            min_level: None,
        },
        keyword: keyword.into(),
        postman: true,
        carpet: false,
        held: false,
        yalahar_gate: false,
        yalahar_town_gate: None,
        quest_gate: None,
        quest_clear_gate: None,
        ticket: None,
        return_origin: None,
        world_change: None,
        conditional: Some(ConditionalTravel {
            progress: vec![ProgressWindow {
                track: BARBARIAN.into(),
                absent: -1,
                minimum: 8,
                maximum: 8,
            }],
            diversion,
            offer_reply: reply.into(),
            unavailable_reply: Some(reply.into()),
            declined_exchange: None,
            cancel_reply: None,
        }),
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    at: Position,
) -> TestResult {
    let [buddel] = classic_quests::install(owner, admitted, [NPC]).await?;
    let preferred = cell(ARRIVAL_CELLS[1]);
    let denied = "T'at is a small island to the east.";
    let mut route = route_fixture("helheim", ARRIVAL_CELLS[1], denied);
    let policy = route.conditional.as_mut().ok_or("Helheim policy")?;
    policy.progress.push(ProgressWindow {
        track: ICE.into(),
        absent: -1,
        minimum: 30,
        maximum: i64::MAX,
    });
    let diversion = policy.diversion.clone();
    let mut entry = NpcTalkEntry::new("Buddel", None);
    entry.travel_routes.push(route);
    owner
        .states
        .chat
        .install_npc_talk(std::sync::Arc::new(NpcTalkBook::default().with(NPC, entry)));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            item: "oteryn:item.tibia.i3031".into(),
            count: 50,
            position: at,
        },
        984,
        true,
    )
    .await?;
    let character = fence.character_id.as_bytes().as_slice();
    seed(h, character, BARBARIAN, 8).await?;
    seed(h, character, ICE, 29).await?;
    classic_quests::talk(owner, admitted, buddel, 19500, "hi").await?;
    let refused = classic_quests::talk(owner, admitted, buddel, 19501, "helheim").await?;
    assert!(refused.iter().any(|line| line == denied));
    classic_quests::talk(owner, admitted, buddel, 19502, "yes").await?;
    assert!(fares(h).await?.is_empty());

    let digest = owner
        .runtime
        .lock()
        .await
        .content_pin()
        .server_artifact_digest();
    let mut confirmations = (19600..20100).filter_map(|command_id| {
        let command = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id,
            item_fence: Some(fence.item_fence()),
        };
        let mut ordinal = 0;
        let selected = diversion.select(preferred, |bound| {
            let value = route_draw(digest, command, buddel, bound, ordinal);
            ordinal += 1;
            value
        })?;
        (selected != preferred).then_some((command_id, selected))
    });
    let (stale, _) = confirmations.next().ok_or("diversion for stale quote")?;
    let (valid, selected) = confirmations
        .find(|(command, _)| *command > stale + 1)
        .ok_or("diversion for valid quote")?;

    seed(h, character, ICE, 30).await?;
    classic_quests::talk(owner, admitted, buddel, 19503, "helheim").await?;
    seed(h, character, ICE, 29).await?;
    super::qengine_rewards_fixtures::talk(owner, admitted, buddel, stale, "yes").await?;
    assert!(
        fares(h).await?.is_empty(),
        "confirmation must recheck the exact track"
    );

    seed(h, character, ICE, 30).await?;
    classic_quests::talk(owner, admitted, buddel, stale + 1, "helheim").await?;
    let confirmed = classic_quests::talk(owner, admitted, buddel, valid, "yes").await?;
    assert!(confirmed.iter().any(|line| line.contains("Set the sails")));
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let placed = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!(
        (placed.x, placed.y, placed.floor),
        (selected.x, selected.y, selected.floor)
    );
    assert_eq!(fares(h).await?, [50]);
    Ok(())
}

async fn seed(h: &Harness, character: &[u8], track: &str, value: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let written = sqlx::query("INSERT INTO game_character_quest_tracks
        (character_id,track_key,quest_key,value,committed_character_revision)
        SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots
        WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value")
        .bind(character).bind(track).bind(QUEST).bind(value).execute(&mut *tx).await?;
    assert_eq!(written.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}

async fn fares(h: &Harness) -> TestResult<Vec<i64>> {
    Ok(sqlx::query_scalar(
        "SELECT fare_gold_units FROM game_newhaven_travel_receipts WHERE arrived_at IS NOT NULL",
    )
    .fetch_all(&h.pool)
    .await?)
}
