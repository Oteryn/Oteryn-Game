//! Finite selected Winterlight route over native island geometry and existing owners.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn questpc8_winterlight_native_disguise_four_fires_queen_and_base_outfit() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc8-winterlight", || {
        super::quest_world_callbacks::scenario(
            "authored-winterlight",
            Position {
                x: 33745,
                y: 31061,
                floor: -8,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_FINISHED questpc8_winterlight_native_route duration={:?} result={result:?}",
        started.elapsed()
    );
    result
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let active = controller.active().ok_or("active content")?;
    super::questpc8_native_slice::world(
        active.native_gameplay().ok_or("native")?,
        &[
            (33740, 31058, 33747, 31063, -8),
            (33752, 31065, 33764, 31078, -9),
        ],
        &[],
        TilePos {
            x: 33744,
            y: 31061,
            floor: -8,
        },
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    assert!(
        owner.premium_coordinator.is_some(),
        "Winterlight uses the native Premium purchase fixture"
    );
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let look = native
        .policies()
        .policies()
        .iter()
        .find(|p| p.definition_key == "oteryn:creature.percht")
        .and_then(|p| p.outfit_appearance())
        .ok_or("native Percht look")?;
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;
    {
        let sessions = owner.states.quest_sessions.lock().map_err(debug)?;
        let copy = sessions
            .get(&admitted.game_session_id)
            .and_then(|entry| entry.copy.as_ref())
            .ok_or("admitted Quest copy")?;
        assert_eq!(
            copy.read_tracks(owner.quest_catalogue.as_ref().ok_or("Quest catalogue")?)
                .ok_or("Quest read projection")?
                .get("oteryn:quest-progress/chosen/winterlight/stage"),
            Some(&0),
            "the real admission read projection supplies the declared initial stage"
        );
    }
    super::questpc8_mystery_geometry::arrive(
        owner,
        admitted,
        TilePos {
            x: 33744,
            y: 31061,
            floor: -8,
        },
    )
    .await?;
    let target = super::questpc8_authored_programmes::object_target(
        owner,
        TilePos {
            x: 33743,
            y: 31061,
            floor: -8,
        },
        1,
    )?;
    assert_eq!(
        super::questpc8_authored_programmes::use_target(owner, admitted, 1, &target).await?,
        UseDisposition::Committed,
        "the native western bonfire must grant the selected three physical disguise components"
    );
    for item in [30337, 30185, 30186] {
        assert_eq!(quantity(h, item).await?, 1);
    }
    assert_eq!(progress(h, "stage").await?, 1);
    assert_eq!(
        super::questpc8_authored_programmes::use_target(owner, admitted, 1, &target).await?,
        UseDisposition::Committed
    );
    for item in [30337, 30185, 30186] {
        assert_eq!(
            quantity(h, item).await?,
            1,
            "replay cannot duplicate components"
        );
    }
    let mut command = 2;
    use_at(
        owner,
        admitted,
        &mut command,
        (33743, 31061, -8),
        (33744, 31061, -8),
    )
    .await?;
    assert_eq!(progress(h, "stage").await?, 2);
    for item in [30337, 30185, 30186] {
        assert_eq!(quantity(h, item).await?, 0);
    }
    assert_eq!(appearance(owner, admitted).await?, Some(look));
    // Lose both disposable caches, then reload the authoritative Quest copy.
    {
        let runtime = owner.runtime.lock().await;
        let actor = admitted.runtime_actor.ok_or("actor")?;
        let mut states = owner.states.spell_states.lock().await;
        let current = states
            .get(&runtime, actor, admitted.game_session_id)
            .ok_or("state")?;
        let next = crate::spell::actor_conditions::stage_quest_disguise(
            current,
            None,
            owner.owner_now().get() / 1000,
        )
        .map_err(debug)?
        .ok_or("clear projection")?;
        assert!(states.commit(&runtime, actor, admitted.game_session_id, next));
    }
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(debug)?
        .remove(&admitted.game_session_id);
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;
    assert_eq!(
        appearance(owner, admitted).await?,
        Some(look),
        "durable reload restores the real Percht appearance"
    );
    use_at(
        owner,
        admitted,
        &mut command,
        (33743, 31061, -8),
        (33744, 31061, -8),
    )
    .await?;
    assert_eq!(quantity(h, 30214).await?, 1);
    for fire in 1..=4 {
        super::questpc8_mystery_geometry::arrive(owner, admitted, tile((33757, 31074, -9))).await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        assert!(
            owner.quest_guardians_for_test(QUEEN).await.is_empty(),
            "Queen stays unavailable before the fourth fire"
        );
        let target =
            super::questpc8_authored_programmes::object_target(owner, tile((33758, 31074, -9)), 1)?;
        assert_eq!(
            super::questpc8_authored_programmes::use_target(owner, admitted, command, &target)
                .await?,
            UseDisposition::Rejected,
            "a fire requires its five real branches"
        );
        command += 1;
        use_at(
            owner,
            admitted,
            &mut command,
            (33745, 31059, -8),
            (33744, 31059, -8),
        )
        .await?;
        assert_eq!(quantity(h, 30226).await?, 5);
        use_at(
            owner,
            admitted,
            &mut command,
            (33758, 31074, -9),
            (33757, 31074, -9),
        )
        .await?;
        assert_eq!(quantity(h, 30226).await?, 0);
        assert_eq!(
            quantity(h, 30214).await?,
            1,
            "exact tool match retains the uncharged torch"
        );
        assert_eq!(progress(h, "stage").await?, 3 + fire);
        assert_eq!(appearance(owner, admitted).await?, Some(look));
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let queens = owner.quest_guardians_for_test(QUEEN).await;
    assert_eq!(
        queens.len(),
        1,
        "four fires activate the native thawed Queen"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "Queen attacker fence")?
        .ok_or("Queen attacker fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("Queen attacker")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let before_xp = experience(h).await?;
    super::grizzly_npc::questpc8_campaign::lethal(
        owner,
        admitted,
        queens[0],
        "oteryn:creature.the_percht_queen",
        command,
        1,
    )
    .await?;
    assert_eq!(progress(h, "kills").await?, 1);
    assert!(
        experience(h).await? > before_xp,
        "native Queen death also commits its ordinary XP"
    );
    use_at(
        owner,
        admitted,
        &mut command,
        (33758, 31074, -9),
        (33757, 31074, -9),
    )
    .await?;
    assert_eq!(progress(h, "stage").await?, 8);
    assert_eq!(
        appearance(owner, admitted).await?,
        None,
        "completion removes only the Quest disguise"
    );
    let keys:Vec<String>=sqlx::query_scalar("SELECT unlock_key FROM game_account_appearance_unlocks WHERE account_id=encode($1,'hex')::uuid AND unlock_key LIKE 'oteryn:outfit.percht_raider/quest-grant/%' ORDER BY unlock_key")
        .bind(id(40).as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        keys,
        vec!["oteryn:outfit.percht_raider/quest-grant/0"],
        "base outfit only"
    );
    let target =
        super::questpc8_authored_programmes::object_target(owner, tile((33758, 31074, -9)), 1)?;
    assert_eq!(
        super::questpc8_authored_programmes::use_target(owner, admitted, command, &target).await?,
        UseDisposition::Rejected
    );
    eprintln!(
        "POSTGRES_WINTERLIGHT_PHYSICAL_COMPONENTS_DISGUISE_COLD_FOUR_FIRES_QUEEN_BASE_OUTFIT_PASS"
    );
    Ok(())
}
const QUEEN: &str = "oteryn:quest-death/authored-winterlight-queen";
fn tile((x, y, floor): (u16, u16, i8)) -> TilePos {
    TilePos { x, y, floor }
}
async fn use_at(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    target: (u16, u16, i8),
    landing: (u16, u16, i8),
) -> TestResult {
    super::questpc8_mystery_geometry::arrive(owner, admitted, tile(landing)).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let target = super::questpc8_authored_programmes::object_target(owner, tile(target), 1)?;
    assert_eq!(
        super::questpc8_authored_programmes::use_target(owner, admitted, *command, &target).await?,
        UseDisposition::Committed
    );
    *command += 1;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}
async fn appearance(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<Option<u32>> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    Ok(states
        .get(
            &runtime,
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
        )
        .ok_or("state")?
        .conditions
        .outfit_at(owner.owner_now().get()))
}
async fn progress(h: &Harness, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/chosen/winterlight/{suffix}")).fetch_one(&h.pool).await?)
}
async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1 AND e.character_id=encode($2,'hex')::uuid").bind(format!("oteryn:item.tibia.i{item}")).bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
