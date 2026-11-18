//! Route the retained quest hazards through the production STEP and carried-tool USE owners.
use crate::bestiary_postgres_harness as fixture;
use crate::foundation::{
    ChannelContentPin, ChannelId, ChannelRuntimeV1, CharacterId, CharacterLease, ExactActorRef,
    GameSessionId, NodeId, WorldId,
};
use fixture::{debug, id};
use oteryn_gameplay_content::content::{
    ReferenceRationalPercent, ReferenceResistanceKind, native_gameplay::NativeGameplayState,
};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{
    self, ComposedTestOwner, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_dispatch::gameplay_transport::{ComposedFreshAdmission, FreshEvidenceSource};
use oteryn_gameplay_quests::quest::{demon_oak as rules, loader::load_embedded_quest_state};
use oteryn_gameplay_spell::spell::{Vocation, cast::CharacterCastFacts};
use oteryn_gameplay_world::map::{boot::BundleCollisionIndexExt, view::MapFacts};
use oteryn_protocol_oteryn::{
    world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget},
    world_spatial::{StepDirection, StepDisposition},
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[path = "engnpc2_paint_transport/world.rs"]
mod world;
type TestResult<T = ()> = fixture::TestResult<T>;

#[derive(Clone, Copy, Debug)]
enum Case {
    PitsMatching,
    PitsOther,
    OakFifth,
    OakFinal,
    OakArea,
    OakBonebeasts,
}

#[test]
fn areadmg_pits_final_field_matches_base_vocation() -> TestResult {
    run(Case::PitsMatching)
}

#[test]
fn areadmg_pits_final_field_hurts_other_vocations() -> TestResult {
    run(Case::PitsOther)
}

#[test]
fn areadmg_demon_oak_fifth_cut_spawns_its_wave_and_harms_the_cutter() -> TestResult {
    run(Case::OakFifth)
}

#[test]
fn areadmg_demon_oak_final_cut_summons_demon_then_finishes_after_clear() -> TestResult {
    run(Case::OakFinal)
}

#[test]
fn areadmg_demon_oak_step_in_rolls_earth_damage() -> TestResult {
    run(Case::OakArea)
}

#[test]
fn areadmg_demon_oak_non_wave_roll_controls_bonebeast_retaliation() -> TestResult {
    run(Case::OakBonebeasts)
}

fn run(case: Case) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(debug)?
                .block_on(async {
                    let h =
                        fixture::Harness::create_with_admission_seed(admin, "areadmg", true, false)
                            .await?;
                    let result = exercise(&h, case).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "area damage scenario panicked")?
        .map_err(Into::into)
}

fn make_world(
    native: &NativeGameplayState,
    world: WorldId,
    channel: ChannelId,
    case: Case,
) -> TestResult<oteryn_gameplay_world::map::boot::BundleWorld> {
    use self::world::{ExpectedTeleport, ExpectedTile, NativeCrop};
    if matches!(case, Case::PitsMatching | Case::PitsOther) {
        return world::native_scene_with(
            native,
            vec![NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../content/world/placements/region-z10-x128-y126.b3"
                )),
                region: (10, 128, 126),
                bounds: (32790, 32330, 32793, 32334),
                start: (32791, 32332),
                expected: vec![ExpectedTile {
                    position: (32791, 32331),
                    ids: vec![354],
                    counted_entry: None,
                }],
            }],
            &[],
            &[],
            world,
            channel,
            |tile| {
                if (tile.x, tile.y) == (32791, 32331) {
                    tile.items[0].attrs.action = Some(if matches!(case, Case::PitsMatching) {
                        2069
                    } else {
                        2045
                    });
                }
            },
        );
    }
    world::native_scene(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../content/world/placements/region-z07-x127-y126.b3"
            )),
            region: (7, 127, 126),
            bounds: (32710, 32340, 32722, 32355),
            start: (32716, 32348),
            expected: vec![ExpectedTile {
                position: (32716, 32349),
                ids: vec![918, 914],
                counted_entry: None,
            }],
        }],
        &[ExpectedTeleport {
            position: (32714, 32345, 7),
            ordinal: 3,
            destination: (32716, 32340, 7),
        }],
        &[],
        world,
        channel,
    )
}

async fn exercise(h: &fixture::Harness, case: Case) -> TestResult {
    super::engnpc2_live_transport::fresh(h).await?;
    let scope = fixture::fence(1)?.runtime_scope;
    let world = scope.world_id();
    let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
    let session = GameSessionId::decode(&id(50)).map_err(debug)?;
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            world, None,
        );
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?;
    let scene = make_world(native, world, channel, case)?;
    let start = scene.start();
    let cells = scene
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let room = pin
        .qualified_room()
        .bundle_view(
            cells,
            scene.frame_binding_digest(world),
            scene.map_revision_digest(),
            oteryn_gameplay_content::content::NativeEntryStart {
                x: i32::from(start.x),
                y: i32::from(start.y),
                floor: -i16::from(start.floor),
            },
        )
        .ok_or("qualified area damage scene")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    let start = room.entry_start();
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        12,
        channel_pin,
    )
    .map_err(debug)?;
    runtime
        .install_companion_policies(native.policies().clone())
        .map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(debug)?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(
                CharacterId::decode(&id(41)).map_err(debug)?,
                current.current_character_lease().generation(),
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let at = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!((at.x, at.y, at.floor), (start.x, start.y, start.floor));
    let runtime = Mutex::new(runtime);
    let content =
        oteryn_gameplay_world::world_runtime::ReferenceContentGeneration::from_content(room.door())
            .map_err(debug)?;
    let fence = oteryn_gameplay_world::world_runtime::ScopeContentGenerationFence::for_test(
        scope,
        fixture::fence(1)?.scope_ownership_generation,
        content,
    );
    let door = Mutex::new(
        oteryn_gameplay_world::world_runtime::bind_native_entry_door(
            room.door(),
            &fence,
            scope,
            fixture::fence(1)?.scope_ownership_generation,
        )
        .map_err(debug)?,
    );
    let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
        "platform-test".into(),
        ("127.0.0.1".into(), 1),
        "source.test".into(),
        "source.test".into(),
        vec![tokio_rustls::rustls::pki_types::CertificateDer::from(
            include_bytes!("../spell-save-tls-fixture/ca.der").to_vec(),
        )],
        vec![tokio_rustls::rustls::pki_types::CertificateDer::from(
            include_bytes!("../spell-save-tls-fixture/client.der").to_vec(),
        )],
        tokio_rustls::rustls::pki_types::PrivateKeyDer::try_from(
            include_bytes!("../spell-save-tls-fixture/client-key.der").to_vec(),
        )?,
    )
    .map_err(debug)?;
    let evidence = FreshEvidenceSource::new(descriptor);
    let spells = oteryn_gameplay_spell::spell::cast::v1_spell_book().map_err(debug)?;
    let achievements =
        oteryn_gameplay_social::achievement_catalogue::AchievementCatalogue::embedded()
            .map_err(debug)?;
    let charms =
        oteryn_gameplay_content::content::charm_source::CanonicalCharmCatalogue::embedded()
            .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner: ComposedFreshAdmission<'_, '_, '_> = ComposedTestOwner {
        root: &h.root,
        character: &authority,
        holder: &h.node,
        evidence: &evidence,
        world_id: world,
        channel_id: channel,
        required_capabilities: composed_test_owner::empty_capabilities(world, channel)?,
        runtime: &runtime,
        room: &room,
        door: &door,
        spells: &spells,
        achievements: &achievements,
        imported_charms: &charms,
    }
    .build();
    owner.active_generation = Some(generation);
    owner.rooms =
        oteryn_gameplay_dispatch::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
    owner.quest_catalogue = Some(Arc::new(
        load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let is_pits = matches!(case, Case::PitsMatching | Case::PitsOther);
    let protections = [
        (
            ReferenceResistanceKind::Fire,
            ReferenceRationalPercent::new(10, 1).map_err(debug)?,
        ),
        (
            ReferenceResistanceKind::Physical,
            ReferenceRationalPercent::new(20, 1).map_err(debug)?,
        ),
    ];
    composed_test_owner::prepare_area_damage_player(
        &owner,
        actor,
        session,
        CharacterCastFacts {
            vocation: if is_pits {
                Vocation::ExaltedMonk
            } else {
                Vocation::Monk
            },
            level: 8,
            magic_level: 0,
            max_health: 10_000,
            max_mana: 90,
            max_soul: 100,
        },
        if is_pits { &protections } else { &[] },
    )
    .await?;
    let fence = composed_test_owner::load_quests(&owner, session).await?;
    match case {
        Case::PitsMatching | Case::PitsOther => {
            let expected = if matches!(case, Case::PitsMatching) {
                270
            } else {
                5760
            };
            check_step(&owner, actor, session, StepDirection::North, expected).await
        }
        Case::OakArea => area(&mut owner, &runtime, actor, session).await,
        Case::OakFifth | Case::OakFinal | Case::OakBonebeasts => {
            super::engnpc2_live_transport::seed_tool(h, native, rules::AXE).await?;
            oak_cut(h, &owner, &runtime, actor, fence, native, case).await
        }
    }
}

async fn check_step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    direction: StepDirection,
    expected: u32,
) -> TestResult {
    let before = composed_test_owner::area_damage_player_health(owner, actor, session).await?;
    let (outcome, _) = owner.paced_step(actor, session, direction).await;
    assert_eq!(outcome.disposition, StepDisposition::Moved);
    let after = composed_test_owner::area_damage_player_health(owner, actor, session).await?;
    assert_eq!(before - after, expected, "typed damage applied on STEP");
    Ok(())
}

async fn oak_cut(
    h: &fixture::Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &Mutex<ChannelRuntimeV1>,
    actor: ExactActorRef,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    native: &NativeGameplayState,
    case: Case,
) -> TestResult {
    let final_cut = matches!(case, Case::OakFinal);
    let track_values: [i64; 4] = match case {
        Case::OakFinal => [10, 11, 11, 11],
        Case::OakBonebeasts => [2, 0, 0, 0],
        _ => [5, 0, 0, 0],
    };
    for (track, value) in rules::CUT_TRACKS.into_iter().zip(track_values) {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(track).bind(rules::QUEST).bind(value)
            .execute(&h.pool).await?;
    }
    use oteryn_gameplay_content::content::native_cell_lookup::NativeMovementCollisionIndex;
    let NativeMovementCollisionIndex::Bundle(index) = owner.movement_cells.index() else {
        return Err("bundle movement cells".into());
    };
    let map = index.map();
    let key =
        oteryn_world_bundle::bundle::placement_key(-7, 32716, 32349, 1).ok_or("weak point")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let axe = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", rules::AXE, "definition-r1"))
        .ok_or("axe reference")?;
    let intent = UseIntent {
        target: UseTarget::ItemByDefinition(axe),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
        use_with_position: None,
    };
    let session = fence.game_session_id;
    let cut = async |command_id| -> TestResult<UseDisposition> {
        let outcome = owner
            .tame_mount(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: Some(fence.item_fence()),
                },
                intent.clone(),
                None,
                None,
                false,
            )
            .await
            .map_err(|_| "axe dispatch rejected")?
            .ok_or("axe action fell through")?;
        Ok(outcome.disposition)
    };
    let first = if matches!(case, Case::OakBonebeasts) {
        bonebeast_command(&*runtime.lock().await, actor, session, 1, true)?
    } else {
        1
    };
    let before = composed_test_owner::area_damage_player_health(owner, actor, session).await?;
    assert_eq!(cut(first).await?, UseDisposition::Committed);
    let after = composed_test_owner::area_damage_player_health(owner, actor, session).await?;
    assert!((170..=210).contains(&(before - after)), "Earth retaliation");
    let (mut monsters, actors) = {
        let runtime = runtime.lock().await;
        let actors = runtime.current_live_creature_census().map_err(debug)?;
        let monsters = actors
            .iter()
            .map(|actor| {
                runtime
                    .current_live_creature_identity(*actor)
                    .map(|key| String::from_utf8_lossy(key).into_owned())
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(debug)?;
        (monsters, actors)
    };
    monsters.sort();
    let tracks = h
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        tracks.tracks().get(rules::CUT_TRACKS[0]),
        Some(&(track_values[0] + 1))
    );
    assert_ne!(tracks.tracks().get(rules::DONE), Some(&1));
    if final_cut {
        assert_eq!(monsters, ["oteryn:creature.demon"]);
        assert_eq!(cut(2).await?, UseDisposition::NotPossible);
        {
            let mut runtime = runtime.lock().await;
            for monster in actors {
                runtime.remove_encounter_actor(monster).map_err(debug)?;
            }
        }
        assert_eq!(cut(3).await?, UseDisposition::Committed);
        let finished = h
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(finished.tracks().get(rules::DONE), Some(&1));
        assert_eq!(finished.tracks().get(rules::PROGRESS), Some(&3));
        assert_eq!(
            runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position()
                .y,
            32340
        );
    } else if matches!(case, Case::OakBonebeasts) {
        assert_eq!(monsters, ["oteryn:creature.bonebeast"; 4]);
        let miss = bonebeast_command(&*runtime.lock().await, actor, session, first + 1, false)?;
        assert_eq!(cut(miss).await?, UseDisposition::Committed);
        assert_eq!(
            runtime
                .lock()
                .await
                .current_live_creature_census()
                .map_err(debug)?,
            actors
        );
    } else {
        assert_eq!(
            monsters,
            [
                "oteryn:creature.blightwalker",
                "oteryn:creature.braindeath",
                "oteryn:creature.demon",
                "oteryn:creature.spectre",
            ]
        );
    }
    Ok(())
}

async fn area(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    runtime: &Mutex<ChannelRuntimeV1>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult {
    let mut unchanged = false;
    let mut struck = false;
    for step in 1_u16..=256 {
        let expected = {
            let runtime = runtime.lock().await;
            next_area_damage(&runtime, actor, session)?
        };
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("clock")?;
        let direction = if step.is_multiple_of(2) {
            StepDirection::East
        } else {
            StepDirection::West
        };
        check_step(owner, actor, session, direction, expected).await?;
        unchanged |= expected == 0;
        struck |= expected > 0;
        if unchanged && struck {
            return Ok(());
        }
    }
    Err("the fixed scenario never observed both StepIn outcomes".into())
}

fn next_area_damage(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<u32> {
    let revision = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .revision()
        + 1;
    let draw = |index| oak_decision(runtime, actor, session, revision, "demon_oak.area", index);
    if !draw(0)?.is_multiple_of(24) {
        return Ok(0);
    }
    oteryn_gameplay_domain::domain::source_normal_draw::source_normal_draw(draw(1)?, 270, 310)
        .ok_or_else(|| "source Earth damage".into())
}

fn bonebeast_command(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    first: u64,
    spawn: bool,
) -> TestResult<u64> {
    for command in first..first + 256 {
        let draw = oak_decision(runtime, actor, session, command, "demon_oak.retaliation", 0)?;
        if (draw % 100 >= 89) == spawn {
            return Ok(command);
        }
    }
    Err("the fixed scenario never drew the requested Bonebeast result".into())
}

fn oak_decision(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    ordinal: u64,
    domain: &str,
    slot: u64,
) -> TestResult<u64> {
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };
    use sha2::{Digest, Sha256};

    let occurrence = Sha256::new()
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(ordinal.to_be_bytes())
        .finalize();
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence[..16].try_into()?);
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    deterministic_decision_u64(&root, occurrence, domain, slot).map_err(debug)
}
