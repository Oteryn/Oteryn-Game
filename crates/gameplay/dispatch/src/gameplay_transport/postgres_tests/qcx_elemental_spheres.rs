//! Arkulius dialogue and shop through the production talk dispatcher.
//! TibiaWiki Elemental Spheres Quest/Spoiler, curid 25641, oldid 1179410.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, CharacterId, CharacterLease, MovementFacing, NpcPlacement,
    NpcPlacementTable, NpcPlacements, WorldId,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcOffer, NpcTalkIntent, NpcTradeSide,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const ARKULIUS: &str = "oteryn:npc.arkulius";

#[derive(Clone, Copy)]
enum Case {
    Shop,
    Dialogue(u32),
    Placement,
}

#[test]
fn arkulius_only_buys_neutral_matter() -> TestResult {
    run_case("arkulius-shop", Case::Shop)
}

#[test]
fn arkulius_explains_alverus_shrines_and_spheres() -> TestResult {
    run_case("arkulius-introduction", Case::Dialogue(80))
}

#[test]
fn arkulius_explains_level_refusal() -> TestResult {
    run_case("arkulius-level-refusal", Case::Dialogue(79))
}

#[test]
fn arkulius_can_be_addressed_at_his_edron_placement() -> TestResult {
    run_case("arkulius-placement", Case::Placement)
}

fn run_case(name: &'static str, case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "qcx_elemental_spheres", false, false)
                    .await?;
            let result = exercise(&h, case).await;
            h.cleanup().await?;
            result
        })
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx_elemental_spheres {name} {:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn exercise(h: &Harness, case: Case) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active content")?;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
        match case {
            Case::Placement => (33280, 31846, 3),
            _ => (2, 0, 7),
        },
    );
    let deps = Dependencies::with_content_and_capacity(
        channel_pin,
        pin.qualified_room().clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        2,
    )?;
    let admitted = deps.admitted;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let placement = match case {
        Case::Placement => edron_placement()?,
        _ => NpcPlacement {
            x: 2,
            y: 1,
            floor: -7,
            direction: MovementFacing::South,
        },
    };
    let arkulius = install(&owner, &admitted, placement).await?;
    let greeting = talk(&mut owner, &admitted, arkulius, 700, "hi").await?;
    if matches!(case, Case::Placement) {
        assert_eq!(
            greeting
                .conversation
                .and_then(|reply| reply.open)
                .map(|(npc, _)| npc),
            Some(arkulius),
        );
        return Ok(());
    }

    if let Case::Dialogue(level) = case {
        let policy = active
            .native_gameplay()
            .and_then(|native| native.progression())
            .ok_or("native progression")?
            .policy_for("profile-1", "ruleset-1", "content-1")
            .map_err(debug)?;
        let binding = crate::gameplay_transport::SessionProgressionBinding::from_content(policy);
        let experience = binding
            .policy
            .thresholds
            .iter()
            .find(|entry| entry.level == level)
            .ok_or("level threshold")?
            .minimum_experience
            .get();
        sqlx::query(
            "INSERT INTO game_character_progression_state VALUES \
         (encode($1,'hex')::uuid,1,$2,$3,'profile-1','ruleset-1','content-1',$4,$5,$6,$7,$8)",
        )
        .bind(id(41).as_slice())
        .bind(i64::from(level))
        .bind(experience)
        .bind(&binding.context.simulation)
        .bind(&binding.context.evidence)
        .bind(&binding.context.declaration)
        .bind(&binding.policy_revision)
        .bind(&binding.reward_revision)
        .execute(&h.pool)
        .await?;
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        owner
            .refresh_quest_session(admitted.game_session_id, true)
            .await;
        {
            let runtime = deps.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    deps.actor,
                    admitted.game_session_id,
                    crate::spell::cast::CharacterCastFacts {
                        level,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("fixture level projection")?;
        }
        let expected: &[(_, _)] = if level < 80 {
            &[("mission", "this task is highly dangerous")]
        } else {
            &[
                ("alverus", "He accidentally created an Ice Overlord"),
                ("shrines", "matching elemental sphere is very close"),
                ("sphere", "I found a way to visit them"),
            ]
        };
        for (offset, (keyword, expected)) in expected.iter().copied().enumerate() {
            let lines = talk(
                &mut owner,
                &admitted,
                arkulius,
                701 + u64::try_from(offset)?,
                keyword,
            )
            .await?;
            assert!(
                lines
                    .conversation
                    .iter()
                    .flat_map(|reply| &reply.lines)
                    .any(|line| line.contains(expected)),
                "{:?}",
                lines.conversation
            );
        }
        let stage: Option<i64> = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks
             WHERE character_id=encode($1,'hex')::uuid
             AND track_key='oteryn:quest-progress/elemental_spheres/quest_line'",
        )
        .bind(id(41).as_slice())
        .fetch_optional(&h.pool)
        .await?;
        assert_eq!(stage, None);
        // Catalogue hash captured from origin/main 871c1dec7 before adding the dialogue.
        let predecessor = [
            0xf9, 0xdc, 0x8b, 0xf1, 0x0b, 0xd4, 0x36, 0xcd, 0x4a, 0xbd, 0xea, 0x28, 0x85, 0xc9,
            0xc6, 0x95, 0x10, 0xa8, 0x20, 0xdb, 0x08, 0xf3, 0xd1, 0xf9, 0x11, 0x38, 0x69, 0x8f,
            0xc2, 0xbf, 0x79, 0x63,
        ];
        let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
        let quest = oteryn_gameplay_quests::quest::elemental_spheres::QUEST;
        assert_ne!(catalogue.definition_hash(quest), Some(predecessor));
        assert!(
            oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(
                catalogue,
                quest,
                &predecessor,
            )
            .is_some()
        );
        return Ok(());
    }

    let trade = talk(&mut owner, &admitted, arkulius, 704, "trade").await?;
    let neutral_matter = active
        .native_gameplay()
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i954", "definition-r1"))
        .ok_or("Neutral Matter definition")?;
    assert_eq!(
        trade.window.ok_or("Arkulius shop window")?.offers,
        vec![NpcOffer {
            item_definition_ref: neutral_matter.get(),
            side: NpcTradeSide::Sell,
            count: 1,
            unit_price: 5000,
        }]
    );
    Ok(())
}

fn edron_placement() -> TestResult<NpcPlacement> {
    let shard = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/npc-placements/placements-00000-00055.json",
    ))?;
    let document: serde_json::Value = serde_json::from_slice(&shard)?;
    let row = document["records"]
        .as_array()
        .ok_or("placement records")?
        .iter()
        .map(|row| &row["declaration"])
        .find(|row| row["npc"] == ARKULIUS)
        .ok_or("Arkulius is missing from the production Edron placement shard")?;
    let cell = &row["cell"];
    assert_eq!(
        (
            cell["x"].as_u64(),
            cell["y"].as_u64(),
            cell["floor"].as_i64()
        ),
        (Some(33280), Some(31845), Some(3))
    );
    assert_eq!(row["direction"], "north");
    Ok(NpcPlacement {
        x: 33280,
        y: 31845,
        floor: -3,
        direction: MovementFacing::North,
    })
}

async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    placement: NpcPlacement,
) -> TestResult<EntityRef> {
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &[ARKULIUS]),
    ));
    let session = admitted.game_session_id;
    owner
        .states
        .chat
        .admit(session, "Quest witness", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("current fence")?;
    let mut runtime = owner.runtime.lock().await;
    runtime
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            session,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: ARKULIUS.into(),
                    placements: vec![placement],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    let npc = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find(|npc| runtime.npc_key(npc.actor) == Some(ARKULIUS))
        .ok_or("Arkulius placement")?;
    Ok(EntityRef {
        identity: npc.actor.placement_identity(),
        generation: npc.generation,
    })
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: EntityRef,
    id: u64,
    text: &str,
) -> TestResult<oteryn_gameplay_npc::npc_command::NpcTalkOutcome> {
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("current item fence")?;
    let reply = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: target,
                text: text.into(),
            },
        )
        .await;
    assert_eq!(reply.disposition, NpcIntentDisposition::Ok, "{text}");
    assert!(!reply.unknown, "{text}");
    Ok(reply)
}
