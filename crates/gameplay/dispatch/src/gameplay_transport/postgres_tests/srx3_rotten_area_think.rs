//! Real published World regular minion onThink -> canonical named icon + KV.
use super::fixture::{Harness, debug, run};
use super::{TestResult, reset};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn srx3_rotten_area_native_reserved_think_publishes_canonical_icon_and_kv_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    run_rotten_world_case(RottenWorldCase::InitialCharge)
}

#[derive(Clone, Copy)]
pub(super) enum RottenWorldCase {
    InitialCharge,
    ChargeInterval,
    TorkadaExpedition,
}

pub(super) fn run_rotten_world_case(case: RottenWorldCase) -> TestResult {
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(
        "srx3-rotten-area-native",
        move || {
            run(async |admin| {
                reset();
                // Pinned XML178092-178094: Bloated Man-Maggot33887,31662,14.
                let scene =
                    super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                        x: 33887,
                        y: 31663,
                        floor: 14,
                    })?;
                let active = scene.controller.active().ok_or("actual World activation")?;
                let native = active
                    .native_gameplay()
                    .ok_or("actual native27/34 closure")?;
                assert!(
                    native.qualifies_rotten_blood_callbacks(scene.pin.server_artifact_digest())
                );
                assert!(
                    active
                        .qualify_published_world_context(&scene.room, &scene.pin)
                        .is_some()
                );
                let deps = scene.dependencies()?;
                let h = Harness::create_with_admission_seed(
                    admin,
                    "srx3_rotten_area_native",
                    false,
                    false,
                )
                .await?;
                let evidence = super::house_transfer::RecoverySource::start(&h).await?;
                super::house_transfer::prepare_owned_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let mut owner = deps.owner(&h, &authority);
                owner.evidence = &evidence.evidence;
                owner.active_generation = scene.controller.active();
                owner.rooms =
                    crate::gameplay_transport::SeamRooms::new(Some(&scene.room), Some(&scene.room));
                let catalogue =
                    oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                        .map_err(debug)?;
                let key = oteryn_gameplay_quests::quest::rotten_blood::area_icons::transition(3)
                    .ok_or("source hazard/upper-left clock transition")?;
                assert!(
                    catalogue.catalogue().transition(&key).is_some(),
                    "actual production Quest catalogue must include the source clock rule"
                );
                owner.quest_catalogue = Some(std::sync::Arc::new(catalogue.catalogue().clone()));
                super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
                let mut admitted = deps.admitted;
                super::srx3_activity_source_fixture::prepare_player(
                    &h,
                    &owner,
                    &mut admitted,
                    1999,
                )
                .await?;
                let player = admitted.runtime_actor.ok_or("actual World player")?;
                if matches!(case, RottenWorldCase::TorkadaExpedition) {
                    owner.admit_quest_session(&admitted).await;
                    let session = admitted.game_session_id;
                    let npc = super::rotten_blood_progression::place_npc(
                        &owner,
                        player,
                        session,
                        "oteryn:npc.torkada",
                    )
                    .await?;
                    for (number, text) in
                        [(40, "hi"), (41, "expedition"), (42, "mission"), (43, "yes")]
                    {
                        owner.clock_origin -= std::time::Duration::from_secs(2);
                        super::rotten_blood_progression::talk(
                            &owner, player, session, npc, number, text,
                        )
                        .await?;
                    }
                    use oteryn_gameplay_quests::quest::rotten_blood::{
                        ACCESS_DOOR_TRACK, ACCESS_TRACK,
                    };
                    let character = admitted
                        .item_fence
                        .ok_or("actual current character fence")?
                        .character_id;
                    let state = owner
                        .root
                        .read_character_quest_state(&authority, character)
                        .await
                        .map_err(debug)?;
                    // Rotten Blood Quest/Spoiler, curid 102953, oldid 1107075, Method.
                    assert_eq!(state.tracks().get(ACCESS_TRACK), Some(&1));
                    assert_eq!(state.tracks().get(ACCESS_DOOR_TRACK), Some(&1));
                    drop(owner);
                    drop(authority);
                    drop(seal);
                    return h.cleanup().await;
                }
                let at = MovementLocalPosition {
                    x: 33887,
                    y: 31662,
                    floor: 14,
                };
                assert_eq!(
                    scene.room.movement_cells().index().lookup(
                        scene.room.movement_cells().scope(),
                        crate::content::LogicalCell {
                            x: at.x,
                            y: at.y,
                            z: i32::from(at.floor)
                        }
                    ),
                    Ok(crate::content::CollisionClass::Walkable)
                );
                let minion = {
                    let mut runtime = owner.runtime.lock().await;
                    native
                        .install_companion_policies(&mut runtime)
                        .map_err(debug)?;
                    assert!(
                        runtime
                            .rotten_blood_keyed_icons(player)
                            .map_err(debug)?
                            .iter()
                            .all(Option::is_none)
                    );
                    runtime
                        .spawn_arena_opponent("oteryn:creature.bloated_man_maggot", at, |r, p| {
                            Some(p == at && r.native_summon_cell_free(p))
                        })
                        .map_err(debug)?
                };
                for _ in 0..8 {
                    owner.clock_origin -= std::time::Duration::from_secs(1);
                    owner.drain_monster_melee().await;
                }
                {
                    let runtime = owner.runtime.lock().await;
                    let states = owner.states.spell_states.lock().await;
                    assert!(
                        states
                            .monster_melee
                            .creature_ai
                            .state(minion)
                            .is_some_and(|s| s.profile().is_native()),
                        "actual canonical native AI admission"
                    );
                    let area = runtime
                        .rotten_blood_keyed_icons(player)
                        .map_err(debug)?
                        .into_iter()
                        .flatten()
                        .find(|v| v.0 == "rotten-area");
                    assert_eq!(
                        area,
                        Some(("rotten-area", 0, 17, 1)),
                        "actual native reserved think must publish canonical ArrowUp count1"
                    );
                }
                let character = admitted
                    .item_fence
                    .ok_or("actual current character fence")?
                    .character_id;
                for track in [
                    "oteryn:quest-progress/rotten-blood/area-icons/last-hazard-update",
                    "oteryn:quest-progress/rotten-blood/area-icons/last-upper-left-update",
                ] {
                    let value:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(character.as_bytes().as_slice()).bind(track).fetch_optional(&h.pool).await?;
                    assert!(
                        value.is_some_and(|v| v > 0),
                        "native source timestamp must be durable: {track}"
                    );
                }
                if matches!(case, RottenWorldCase::ChargeInterval) {
                    assert_global_charge_interval(&h, &mut owner, character, player).await?;
                }
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await
            })
        },
    );
    eprintln!(
        "SRX3_ROTTEN_AREA_NATIVE_EXECUTED duration={:?}",
        started.elapsed()
    );
    result
}

async fn assert_global_charge_interval(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    character: crate::foundation::CharacterId,
    player: crate::foundation::ExactActorRef,
) -> TestResult {
    use oteryn_gameplay_quests::quest::rotten_blood::area_icons::TRACKS;
    use std::time::Duration;

    let read_clock = || {
        sqlx::query_scalar::<_, i64>(
            "SELECT value FROM game_character_quest_tracks
             WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
        )
        .bind(character.as_bytes().as_slice())
        .bind(TRACKS[1])
        .fetch_one(&h.pool)
    };
    let first = read_clock().await?;
    let deadline = std::time::Instant::now() + Duration::from_secs(25);
    let second = loop {
        owner.clock_origin -= Duration::from_secs(1);
        owner.drain_monster_melee().await;
        let current = read_clock().await?;
        if current > first {
            break current;
        }
        if std::time::Instant::now() >= deadline {
            return Err("no second Rotten Charge within 25 seconds of the first".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    // Rotten Blood Quest/Spoiler, curid 102953, oldid 1107075, Rotten Charges.
    assert_eq!(second - first, 20);
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .rotten_blood_keyed_icons(player)
            .map_err(debug)?
            .into_iter()
            .flatten()
            .find(|icon| icon.0 == "rotten-area"),
        Some(("rotten-area", 0, 17, 2))
    );
    Ok(())
}
