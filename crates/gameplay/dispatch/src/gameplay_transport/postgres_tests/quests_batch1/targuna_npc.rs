//! Camilla reaches the real NPC caller, QuestState receipt and existing XP/live-vitals owners.
//! Source: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/camilla.lua:72–105.
//! The placed test NPC is a fixture, not qualification of a production Targuna map.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{ChannelContentPin, CharacterLease, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition, targuna as quest};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const NPC: &str = "oteryn:npc.camilla";
const SEED: &str = "oteryn:quest-transition/targuna-camilla-fixture-seed";

#[test]
fn targuna_camilla_real_npc_awards_xp_once_and_refreshes_live_level_after_public_hi() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("TARGUNA_CAMILLA_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("targuna-camilla", scenario)
}

fn report_catalogue() -> TestResult<(QuestStateCatalogue, String, String, String, String)> {
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
    let source = loaded.catalogue();
    let key = format!("{}camilla-reward", quest::TRANSITION_PREFIX);
    // Clone the actual public-loader transition without mocking its mutation or XP.
    let report = source.transition(&key).ok_or("production Camilla transition")?.clone();
    assert_eq!(report.experience, Some(1500));
    let tracks = report.effects.iter().map(|effect|
        source.track(&effect.track).cloned().ok_or("source track"))
        .collect::<Result<Vec<_>, _>>()?;
    let killed = format!("{}ancient_enemy/lizard_commander_killed", quest::TRACK_PREFIX);
    let claimed = format!("{}ancient_enemy/camilla_reward", quest::TRACK_PREFIX);
    let mission = format!("{}ancient_enemy/mission", quest::TRACK_PREFIX);
    // Test setup records the already completed source kill; the subject is the NPC claim.
    let seed = QuestTransition {
        key: SEED.into(), quest: quest::QUEST.into(),
        effects: vec![QuestEffect { track: killed.clone(), from: QuestComparison::Any, effect: QuestEffectKind::Set(1) }],
        completes: false, experience: None,
    };
    Ok((QuestStateCatalogue::new("content-1", tracks, vec![seed, report]).map_err(debug)?, key, killed, claimed, mission))
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "targuna_camilla", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = super::super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("native content")?;
        let bundle = npc_world(world)?;
        let cells = bundle.movement_cells(pin.qualified_room().movement_cells()).map_err(debug)?;
        let native = active.native_gameplay().ok_or("native")?;
        let policy = native.progression().ok_or("progression")?
            .policy_for("profile-1", "ruleset-1", "content-1").map_err(debug)?;
        let progression = crate::gameplay_transport::SessionProgressionBinding::from_content(policy);
        // A fixture800XP below the admitted level9 threshold proves live level refresh too.
        let initial_xp = progression.policy.thresholds.iter().find(|threshold|threshold.level == 9)
            .ok_or("level9")?.minimum_experience.get() - 800;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_progression_state SET level=8,total_experience=$2,simulation_revision=$3,evidence_revision=$4,declaration_revision=$5,policy_revision=$6,reward_revision=$7 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(initial_xp)
            .bind(&progression.context.simulation).bind(&progression.context.evidence)
            .bind(&progression.context.declaration).bind(&progression.policy_revision)
            .bind(&progression.reward_revision).execute(&mut *tx).await?;
        tx.commit().await?;
        let channel_pin = ChannelContentPin::from_activation(world, pin.activation_sequence(),
            pin.identity().server_artifact_digest(), pin.identity().client_artifact_digest(),
            bundle.frame_binding_digest(world), bundle.map_revision_digest(), (2, 0, 7));
        let deps = Dependencies::with_content(channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        let (catalogue, report_key, killed, claimed, mission) = report_catalogue()?;
        owner.quest_catalogue = Some(Arc::new(catalogue));
        let model = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"), npc_catalogue_preproduction_limits(),
        ).map_err(debug)?.service_model_for_project().map_err(debug)?;
        let talk_book = NpcTalkBook::from_model(&model);
        assert!(talk_book.get(NPC).is_some(), "real source greeting, no injected NPC reply");
        owner.states.chat.install_npc_talk(Arc::new(talk_book));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        owner.states.chat.admit(admitted.game_session_id, "Targuna reporter", owner.owner_now());
        owner.states.chat.set_npc_service(admitted.game_session_id);
        let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
        let formula = native.training_formula().ok_or("training")?;
        let build = h.root.read_character_build_state(&authority, fence.character_id).await.map_err(debug)?;
        h.root.commit_character_build(&authority, &h.node, fence,
            crate::durability::character_build::BuildChangeRequest {
                occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(id(91)).map_err(debug)?,
                cause: crate::durability::character_build::BuildCause::VocationChoice,
                after: crate::durability::character_build::convert_vocation(formula, &build, "druid").map_err(debug)?,
                before: build, pruned_stance: None,
            }, formula).await.map_err(debug)?;
        let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"classed fence")?.ok_or("classed fence")?;
        let crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready { facts, .. } =
            crate::gameplay_transport::spell_character_facts::load_character_cast_facts(&h.root, &authority, fence.character_id).await
        else { return Err("durable Druid facts".into()); };
        admitted.item_fence = Some(fence.item_fence());
        let npc = {
            let mut runtime = owner.runtime.lock().await;
            runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
            runtime.bind_attacker_lease(deps.actor, admitted.game_session_id,
                CharacterLease::new(crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation).map_err(debug)?).map_err(debug)?;
            let at = runtime.read_actor_position(deps.actor).map_err(debug)?.position();
            runtime.place_npcs(&NpcPlacementTable { npcs: vec![NpcPlacements {
                key: NPC.into(), placements: vec![NpcPlacement { x: u16::try_from(at.x+1)?, y: u16::try_from(at.y)?,
                    floor: -i8::try_from(at.floor)?, direction: MovementFacing::South }],
            }] }, |_|true, true).map_err(debug)?;
            let placed = runtime.visible_entities().npcs.into_iter()
                .find(|npc|runtime.npc_key(npc.actor)==Some(NPC)).ok_or("Camilla")?;
            assert!(owner.states.spell_states.lock().await.initialize(&runtime, deps.actor, admitted.game_session_id,
                facts, (0, 0), owner.owner_now()).is_some());
            EntityRef { identity: placed.actor.placement_identity(), generation: placed.generation }
        };
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        assert!(owner.progression_binding(admitted.game_session_id).is_some());
        assert!(owner.enter_input(admitted.game_session_id).await.is_some());
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let seed = slot.commit_quest_transition(owner.root, owner.character, owner.holder, fence,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key: SEED.into(),
                cause: crate::durability::quest_state::QuestCause::Command(crate::foundation::CommandRef::new(
                    admitted.game_session_id, crate::foundation::CommandId::new(900).map_err(debug)?)),
            }, owner.quest_catalogue.clone().ok_or("catalogue")?).await.map_err(debug)?;
        assert!(matches!(seed, crate::durability::quest_state::QuestTransitionOutcome::Committed(_)));
        drop(slot);
        owner.refresh_quest_session(admitted.game_session_id, false).await;
        // Public hi opens the native conversation. The very first command7
        // mission must select Camilla's actual topic, not swallow it as another greet.
        let hi = owner.chat_speak(deps.actor, admitted.game_session_id, 1,
            &oteryn_protocol_oteryn::chat::ChatIntent::Say {
                mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say, text: "hi".into(),
            }).await;
        assert_eq!(hi.disposition, oteryn_protocol_oteryn::chat::ChatDisposition::Ok);
        assert!(owner.states.chat.engaged(admitted.game_session_id).is_some());
        let render = |line| quest::dialogue::render(quest::npc_rules::Npc::Camilla,
            &quest::npc_rules::Reply { source_line: line, numbers: Vec::new() }).map(|lines| lines.join(" ")).ok_or("source dialogue");
        assert_eq!(talk(&mut owner, &admitted, npc, 2, "mission").await?, render(83)?);
        assert_eq!(talk(&mut owner, &admitted, npc, 3, "yes").await?, render(98)?);
        let copy = h.root.read_character_quest_state(&authority, fence.character_id).await.map_err(debug)?;
        assert_eq!(copy.tracks().get(&killed), Some(&1), "the commander's proof remains intact");
        assert_eq!(copy.tracks().get(&claimed), Some(&1), "one Camilla claim");
        assert_eq!(copy.tracks().get(&mission), Some(&4), "source mission completion");
        let (xp, level): (i64, i64) = sqlx::query_as("SELECT total_experience,level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(xp, initial_xp+1500, "the XP obligation actually reached its owner");
        assert!(level>8);
        let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_xp_obligations WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(pending, 0);
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert_eq!(i64::from(states.get(&runtime,deps.actor,admitted.game_session_id).ok_or("live actor")?.character_facts().level), level);
        }
        // Retrying the exact command and then asking again must never award
        // a second receipt or XP. This does not simulate an unavailable DB reply.
        assert!(talk(&mut owner, &admitted, npc, 3, "yes").await?.is_empty());
        assert_eq!(talk(&mut owner, &admitted, npc, 4, "mission").await?, render(74)?);
        assert!(talk(&mut owner, &admitted, npc, 5, "yes").await?.is_empty());
        let (count, xp_again): (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$2), (SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(&report_key).fetch_one(&h.pool).await?;
        assert_eq!((count,xp_again), (1,initial_xp+1500));
        Ok(())
    })
}

async fn talk(owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession,
    npc: EntityRef, id: u64, text: &str) -> TestResult<String> {
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
    let outcome = owner.npc_talk(admitted.runtime_actor.ok_or("actor")?, UseCommand {
        game_session_id: admitted.game_session_id, command_id:id, item_fence:Some(fence.item_fence()),
    }, NpcTalkIntent { npc_actor:npc, text:text.into() }).await;
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{id}:{text}");
    assert!(!outcome.unknown);
    Ok(outcome.conversation.map(|conversation|conversation.lines.join(" ")).unwrap_or_default())
}

fn npc_world(world: crate::foundation::WorldId) -> TestResult<crate::map::boot::BundleWorld> {
    // Existing Grizzly NPC test's legal terrain-only bundle, copied to avoid widening helper visibility.
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes)?;
    bundle.manifest.palette.truncate(1);
    bundle.manifest.palette[0].key = "oteryn:item.tibia.i100".into();
    for sector in &mut bundle.sectors {
        sector.tiles.retain(|tile|tile.x<=3);
        for tile in &mut sector.tiles { tile.items.retain(|item|item.palette==0); }
    }
    let bytes = oteryn_world_bundle::writer::write(&bundle.manifest,&bundle.sectors,&Default::default())?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes,&pins).map_err(debug)?;
    crate::map::boot::boot(&bytes,&crate::map::boot::BootPins {
        map_revision:crate::map::overlay::map_revision(&base), bundle:pins,
        start:crate::map::overlay::TilePos {x:2,y:0,floor:-7},
    },world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|_|None).map_err(debug)
}
