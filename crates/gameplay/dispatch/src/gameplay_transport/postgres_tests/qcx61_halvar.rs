//! Crystal 201189f8 halvar.lua:70-115 and movements_arena_enter.lua:3-40.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, classic_quests};
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::{
    arena::{ArenaAction, ArenaDisposition},
    world_spatial::StepDirection,
};
use serde_json::json;

const APPROACH: MovementLocalPosition = MovementLocalPosition {
    x: 32246,
    y: 31098,
    floor: 7,
};

#[test]
fn halvar_payment_opens_the_native_forcefield_without_a_second_fee() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(
        "qcx61-halvar",
        super::arena_match::halvar_paid_entrance,
    )
}

fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/qcx61/arena-entrance",
        "quest":"oteryn:quest.barbarian_arena_quest",
        "selector":{"action":25100,"native":{
            "position":{"x":32246,"y":31097,"floor":-7},"ordinal":1,"appearance":1949
        }},
        "trigger":"step_in","reversible":false,"passages":[],"offerings":[],"storage":[],
        "return_on_failure":false,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/svargrond_arena/movements_arena_enter.lua:3-40"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
) -> TestResult {
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback()?])
        .map_err(|_| "Arena native callback")?;
    {
        let mut runtime = owner.runtime.lock().await;
        let current = runtime.read_actor_position(actor).map_err(debug)?;
        runtime
            .relocate_arena_player(actor, admitted.game_session_id, current, APPROACH)
            .map_err(debug)?;
    }
    let [halvar] = classic_quests::install(owner, admitted, ["oteryn:npc.halvar"]).await?;
    let (unpaid, _) = owner
        .ordinary_paced_step(
            actor,
            admitted.game_session_id,
            StepDirection::North,
            None,
            None,
        )
        .await;
    assert_eq!(
        unpaid.moved_to.ok_or("unpaid step")?.actor_position,
        APPROACH
    );
    assert_eq!(
        super::arena_match::command(owner, admitted, ArenaAction::Enter)
            .await?
            .disposition,
        ArenaDisposition::WrongState
    );
    assert_eq!(fees(h).await?, 0);

    classic_quests::talk(owner, admitted, halvar, 20300, "hi").await?;
    let quote = classic_quests::talk(owner, admitted, halvar, 20301, "fight").await?;
    assert!(quote.iter().any(|line| line.contains("1000")));
    let paid = classic_quests::talk(owner, admitted, halvar, 20302, "yes").await?;
    assert!(paid.iter().any(|line| line.contains("pass the door")));
    let staged: (i32, i32) = sqlx::query_as(
        "SELECT (body->>'paid_rank')::int, (body->>'phase')::int FROM game_character_arena_matches",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(staged, (1, 0));
    assert_eq!(fees(h).await?, 1);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        APPROACH
    );

    owner.clock_origin -= std::time::Duration::from_secs(2);
    let (admitted_step, _) = owner
        .ordinary_paced_step(
            actor,
            admitted.game_session_id,
            StepDirection::North,
            None,
            None,
        )
        .await;
    let landing = admitted_step.moved_to.ok_or("paid step")?.actor_position;
    let (x, y) = crate::durability::arena_match::donor::CENTERS[0];
    assert_eq!((landing.x, landing.y, landing.floor), (x, y, 7));
    assert_eq!(fees(h).await?, 1);
    let leases: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_svargrond_pit_leases WHERE pit=1")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(leases, 1);
    Ok(())
}

async fn fees(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_arena_entry_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}
