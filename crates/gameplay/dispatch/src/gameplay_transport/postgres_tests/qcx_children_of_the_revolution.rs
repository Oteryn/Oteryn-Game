//! TibiaWiki curid 41842/oldid 683807 and curid 41844/oldid 1160862.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, loader,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/questline";
const FRONTIER: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission08";
const WRATH: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/mission11";
const RECRUITMENT: &str = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/mission00";
const ACCESS: &str = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/teleport_access";
const ZALAMON: &str = "oteryn:npc.zalamon";
// Crystal 201189f8, data-global/world/world-npc.xml:2442-2444.
const ZALAMON_CELL: (u16, u16, u8) = (33354, 31410, 8);
const MISSION: &str = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/mission05";

#[test]
fn recruitment_battle_access_and_finale_reward() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .name("qcx57-children".into())
        .stack_size(128 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Children quest scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx57-children duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn published_zalamon() -> TestResult<Position> {
    use oteryn_world_bundle_compiler::{bundle, compile, source};
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.."));
    let (x, y, floor) = ZALAMON_CELL;
    let project = source::project_regions(root, &[(x / 256, y / 256)])?;
    let (resolver, palette) = source::registry(root, &project)?;
    let input = compile::Input {
        regions: &project.regions,
        palette: &palette,
        identity: bundle::Identity::default(),
        world: source::world_extent(root)?,
        build_class: bundle::BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = compile::realize_spawns(&input, &resolver)?;
    let (table, report) = compile::realize_npcs(&input, &resolver, &spawns)?;
    let Some(npc) = table.npcs.iter().find(|npc| npc.key == ZALAMON) else {
        return Err(format!(
            "Zalamon missing from bundle: {:?}",
            report
                .held
                .iter()
                .filter(|row| row.npc == ZALAMON)
                .collect::<Vec<_>>()
        )
        .into());
    };
    let [placement] = npc.placements.as_slice() else {
        return Err("expected one published Zalamon placement".into());
    };
    assert_eq!(
        (placement.x, placement.y, placement.floor),
        (x, y, -i8::try_from(floor)?)
    );
    Ok(Position {
        x: placement.x,
        y: placement.y,
        floor: placement.floor,
    })
}

fn scenario() -> TestResult {
    run(async |admin| {
        let placement = published_zalamon()?;
        let h =
            Harness::create_with_admission_seed(admin, "qcx57_phantom_army", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                super::fence(1)?.runtime_scope.world_id(),
                "content-1",
            )?;
        let frame_digest = activated.frame_binding().digest();
        let room = activated.qualified_room().clone();
        let mut deps = Dependencies::with_content(
            activated.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .get_mut()
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: i32::from(placement.x) + 1,
                    y: i32::from(placement.y),
                    floor: i16::from(placement.floor.unsigned_abs()),
                },
            )
            .map_err(debug)?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let premium = SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        );
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?
            .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(frame_digest.as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence,
                &NativeItemWeights(native),
                &crate::durability::starter_grant::StarterTemplate {
                    definition: crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i2854".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    ..crate::durability::starter_grant::StarterTemplate::main()
                },
            )
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.premium_coordinator = Some(&premium);
        let loaded = loader::load_embedded_quest_state("content-1").map_err(debug)?;
        let mut catalogue = loaded.catalogue().clone();
        for (key, quest, values) in [
            ("frontier", "the_new_frontier_quest", vec![(FRONTIER, 2)]),
            (
                "wrath-closed",
                "wrath_of_the_emperor_quest",
                vec![(WRATH, 2)],
            ),
            (
                "wrath-open",
                "wrath_of_the_emperor_quest",
                vec![(WRATH, -1)],
            ),
            (
                "tome-reported",
                "children_of_the_revolution_quest",
                vec![(QUESTLINE, 18)],
            ),
            (
                "army-survived",
                "children_of_the_revolution_quest",
                vec![(QUESTLINE, 20), (MISSION, 2)],
            ),
        ] {
            catalogue = catalogue
                .with_fixture_transition(QuestTransition {
                    key: fixture_key(key),
                    quest: format!("oteryn:quest.{quest}"),
                    effects: values
                        .into_iter()
                        .map(|(track, value)| QuestEffect {
                            track: track.into(),
                            from: QuestComparison::Any,
                            effect: QuestEffectKind::Set(value),
                        })
                        .collect(),
                    completes: false,
                    experience: None,
                    world_change: None,
                })
                .map_err(debug)?;
        }
        owner.quest_catalogue = Some(Arc::new(catalogue));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_premium(id(40), admitted.game_session_id);
        owner.admit_quest_session(&admitted).await;

        let [zalamon] =
            super::questl2_fixture::install_at(&owner, &admitted, [ZALAMON], [placement]).await?;
        assert_eq!(
            talk(&mut owner, &admitted, zalamon, 1, "hi").await?.0,
            NpcIntentDisposition::Ok
        );
        for (command, text) in [(2, "mission"), (3, "yes"), (4, "yes")] {
            talk(&mut owner, &admitted, zalamon, command, text).await?;
        }
        assert_eq!(
            stage(&h, QUESTLINE).await?,
            None,
            "Frontier mission 8 is required"
        );

        arrange(&owner, &admitted, "frontier", 9000).await?;
        arrange(&owner, &admitted, "wrath-closed", 9001).await?;
        for (command, text) in [(5, "mission"), (6, "yes"), (7, "yes")] {
            talk(&mut owner, &admitted, zalamon, command, text).await?;
        }
        assert_eq!(
            stage(&h, QUESTLINE).await?,
            None,
            "Wrath closes Zalamon's access"
        );

        arrange(&owner, &admitted, "wrath-open", 9002).await?;
        let (_, offer) = talk(&mut owner, &admitted, zalamon, 8, "mission").await?;
        assert!(
            offer.iter().any(|line| line.contains("earn my truzzt")),
            "{offer:?}"
        );
        talk(&mut owner, &admitted, zalamon, 9, "yes").await?;
        assert_eq!(
            stage(&h, QUESTLINE).await?,
            None,
            "first yes only opens the confirmation"
        );
        assert_eq!(
            talk(&mut owner, &admitted, zalamon, 10, "yes").await?.0,
            NpcIntentDisposition::Ok
        );
        assert_eq!(stage(&h, QUESTLINE).await?, Some(1));
        assert_eq!(stage(&h, RECRUITMENT).await?, Some(1));

        arrange(&owner, &admitted, "tome-reported", 9003).await?;
        talk(&mut owner, &admitted, zalamon, 11, "yes").await?;
        assert_eq!(
            stage(&h, ACCESS).await?,
            None,
            "without an offer no battle access is granted"
        );
        talk(&mut owner, &admitted, zalamon, 12, "mission").await?;
        assert_eq!(
            talk(&mut owner, &admitted, zalamon, 13, "yes").await?.0,
            NpcIntentDisposition::Ok
        );
        assert_eq!(stage(&h, ACCESS).await?, Some(1));
        assert_eq!(stage(&h, QUESTLINE).await?, Some(19));
        assert_eq!(stage(&h, MISSION).await?, Some(1));

        let before = totals(&h).await?;
        assert_eq!(before.1, 0);
        talk(&mut owner, &admitted, zalamon, 14, "mission").await?;
        assert_eq!(totals(&h).await?, before);
        arrange(&owner, &admitted, "army-survived", 9004).await?;
        assert_eq!(stage(&h, QUESTLINE).await?, Some(20));
        assert_eq!(
            talk(&mut owner, &admitted, zalamon, 15, "mission").await?.0,
            NpcIntentDisposition::Ok
        );
        assert_eq!(totals(&h).await?, (before.0 + 10_000, 1));
        assert_eq!(stage(&h, QUESTLINE).await?, Some(21));
        assert_eq!(stage(&h, MISSION).await?, Some(3));
        let completed: bool = sqlx::query_scalar(
            "SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states
             WHERE quest_key = 'oteryn:quest.children_of_the_revolution_quest'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert!(completed);
        talk(&mut owner, &admitted, zalamon, 16, "mission").await?;
        assert_eq!(totals(&h).await?, (before.0 + 10_000, 1));
        Ok(())
    })
}

fn fixture_key(name: &str) -> String {
    format!("oteryn:quest-transition/qcx57-fixture/{name}")
}

async fn arrange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    name: &str,
    command: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: fixture_key(name),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(command).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("catalogue")?,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "{name}: {outcome:?}"
    );
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc_actor: EntityRef,
    command_id: u64,
    text: &str,
) -> TestResult<(NpcIntentDisposition, Vec<String>)> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor,
                text: text.into(),
            },
        )
        .await;
    assert!(!outcome.unknown);
    Ok((
        outcome.disposition,
        outcome.conversation.map(|c| c.lines).unwrap_or_default(),
    ))
}

async fn stage(h: &Harness, track: &str) -> TestResult<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(track)
            .fetch_optional(&h.pool)
            .await?,
    )
}

async fn totals(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT total_experience::bigint, (SELECT coalesce(sum(i.quantity),0)::bigint
        FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
        JOIN game_item_instances i ON i.item_instance_id=c.item
        WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i10199')
        FROM game_character_progression_state",
    )
    .bind(id(41).as_slice())
    .bind(id(42).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
