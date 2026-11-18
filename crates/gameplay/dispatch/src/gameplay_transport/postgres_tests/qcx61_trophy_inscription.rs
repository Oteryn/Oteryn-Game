//! Crystal 201189f8 movements_arena_trophy.lua:19-25 and svargrond_arena.lua:175-190.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{item_look, item_view};
use oteryn_protocol_oteryn::{
    arena::{ArenaAction, ArenaDisposition},
    item_look::ItemLookResult,
};

#[test]
fn arena_trophy_look_projects_the_original_victor_name() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(
        "qcx61-trophy-look",
        super::arena_match::trophy_look,
    )
}

pub(super) async fn after_first_rank(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
) -> TestResult {
    let claimed = super::arena_match::command(
        owner,
        admitted,
        ArenaAction::ClaimReward {
            rank: 1,
            category: 4,
            choice: 1,
        },
    )
    .await?;
    assert_eq!(claimed.disposition, ArenaDisposition::Applied);
    let fence = admitted.item_fence.ok_or("trophy character")?;
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_instances i
        JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid
        AND i.definition_production_key='oteryn:item.tibia.i5807' AND i.lifecycle=1")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let trophy = item_view::ItemKey::Instance(bytes.as_slice().try_into()?);
    let recipient: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    let backpack = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await?;
    let index = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("Item index")?;
    let inventory = item_view::inventory_from_backpack(backpack.as_ref(), |definition| {
        index.definition_ref(
            &definition.family,
            &definition.production_key,
            &definition.revision_ref,
        )
    })
    .ok_or("inventory")?;
    let mut views = item_view::SessionItemView::resume(item_view::ItemViewContinuity::default());
    views.snapshot(inventory, None).map_err(debug)?;
    let handle = views.table().handle(&trophy).ok_or("trophy handle")?;
    let ItemLookResult::Text(text) = item_look::look_item(
        owner,
        actor,
        admitted.game_session_id,
        Some(fence),
        &views,
        handle,
    )
    .await
    else {
        return Err("the visible trophy must support LOOK".into());
    };
    let inscription = format!(
        "It is given to the courageous victor of the barbarian arena in greenhorn difficulty. Awarded to {recipient}"
    );
    assert!(text.ends_with(&inscription));
    Ok(())
}
