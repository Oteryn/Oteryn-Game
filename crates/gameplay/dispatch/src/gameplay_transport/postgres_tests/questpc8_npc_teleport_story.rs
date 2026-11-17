//! M37: a real source NPC command reaches held placement and arrival consumption.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn m37_peter_npc_teleport_postgres_dispatches_and_replays_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("M37_DISPATCH_POSTGRES_NOT_EXECUTED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("qengine-rewards-m37-peter", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-m37-peter",
            Position {
                x: 32860,
                y: 31303,
                floor: -7,
            },
            false,
        )
    })?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "M37_POSTGRES_NPC_DISPATCH_PLACED_AND_REPLAYED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [peter] = super::classic_quests::install(owner, admitted, ["oteryn:npc.peter"]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let before = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let paid: i64 = sqlx::query_scalar("SELECT count(*) FROM game_newhaven_travel_receipts")
        .fetch_one(&h.pool)
        .await?;
    super::classic_quests::talk(owner, admitted, peter, 5400, "hi").await?;
    super::classic_quests::talk(owner, admitted, peter, 5401, "factory").await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let placed = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        (
            placed.position().x,
            placed.position().y,
            placed.position().floor
        ),
        (32859, 31302, 7)
    );
    assert!(!owner.states.newhaven_travel.held(admitted.game_session_id));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_pending_arrivals")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_quest_npc_teleport_receipts")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_newhaven_travel_receipts")
            .fetch_one(&h.pool)
            .await?,
        paid
    );
    assert_eq!(sqlx::query_scalar::<_,String>("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?,revision);
    let after = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(after.tracks(), before.tracks());
    assert_eq!(after.states(), before.states());
    super::classic_quests::talk(owner, admitted, peter, 5401, "factory").await?;
    let replayed = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        replayed.revision(),
        placed.revision(),
        "replay never places twice"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_quest_npc_teleport_receipts")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    Ok(())
}
