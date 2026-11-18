use super::fixture::{Harness, debug};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::durability::character_authority::ReconciledCharacterAuthority;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [peter] = super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.peter"]).await?;
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
    super::classic_quests_fixtures::talk(owner, admitted, peter, 5400, "hi").await?;
    super::classic_quests_fixtures::talk(owner, admitted, peter, 5401, "factory").await?;
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
    super::classic_quests_fixtures::talk(owner, admitted, peter, 5401, "factory").await?;
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
