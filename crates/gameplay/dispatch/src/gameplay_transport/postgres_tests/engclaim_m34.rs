//! ENGCLAIM: native authored server content through the production dispatch owner and PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

fn claim_case(suffix: &'static str) -> TestResult {
    let shard =
        include_str!("../../../../../../content/interactions/reward_claims/engclaim-native.json");
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let callback = callbacks
        .iter()
        .find(|c| c.key.ends_with(suffix))
        .cloned()
        .ok_or("callback")?;
    let siblings = callbacks
        .into_iter()
        .filter(|c| c.quest == callback.quest)
        .collect::<Vec<_>>();
    let claim = callback
        .world_claim
        .as_ref()
        .ok_or("native binding")?
        .key
        .clone();
    let document: serde_json::Value = serde_json::from_str(shard)?;
    let definition = document["records"]
        .as_array()
        .ok_or("records")?
        .iter()
        .find(|r| r["definition"]["identity"]["key"] == claim)
        .ok_or("claim")?["definition"]
        .clone();
    let source = &definition["placements"][0];
    let chest = (
        u16::try_from(
            source["source_binding"]["project_position"]["x"]
                .as_u64()
                .ok_or("x")?,
        )?,
        u16::try_from(
            source["source_binding"]["project_position"]["y"]
                .as_u64()
                .ok_or("y")?,
        )?,
        -i8::try_from(
            source["source_binding"]["project_position"]["z"]
                .as_u64()
                .ok_or("z")?,
        )?,
    );
    let appearance = u16::try_from(source["appearance_tibia_id"].as_u64().ok_or("appearance")?)?;
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "qready1_claim", false, false).await?;
        prepare_current_session(&harness).await?;
        let plan = serde_json::to_value(&callback)?;
        let valid: bool = sqlx::query_scalar("SELECT game_quest_native_claim_plan($1)")
            .bind(sqlx::types::Json(&plan))
            .fetch_one(&harness.pool)
            .await?;
        assert!(valid);
        for field in ["key", "revision", "authority_sha256"] {
            let mut forged = plan.clone();
            forged["world_claim"][field] = "unbound".into();
            if field == "revision" {
                forged["world_claim"][field] = "".into();
            }
            let valid: bool = sqlx::query_scalar("SELECT game_quest_native_claim_plan($1)")
                .bind(sqlx::types::Json(forged))
                .fetch_one(&harness.pool)
                .await?;
            assert!(!valid, "{field}");
        }

        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let positions = siblings
            .iter()
            .enumerate()
            .map(|(i, c)| -> TestResult<_> {
                let key = &c.world_claim.as_ref().ok_or("binding")?.key;
                let d = document["records"]
                    .as_array()
                    .ok_or("records")?
                    .iter()
                    .find(|r| r["definition"]["identity"]["key"] == *key)
                    .ok_or("record")?;
                let p = &d["definition"]["placements"][0]["source_binding"]["project_position"];
                Ok((
                    u16::try_from(p["x"].as_u64().ok_or("x")?)?,
                    u16::try_from(p["y"].as_u64().ok_or("y")?)?,
                    6000 + u16::try_from(i)?,
                ))
            })
            .collect::<TestResult<Vec<_>>>()?;
        let world = super::quest_callbacks::source_world_positions(chest, &positions, appearance)?;
        let shards = [shard];
        let mut start = world.start();
        if siblings.len() > 1 {
            start.x += 1;
        }
        let report = crate::content::world_reward_claims::bind_reward_claims(
            world.base().as_ref(),
            &shards,
            &|_| true,
        )?;
        assert!(
            report
                .served
                .iter()
                .any(|entry| entry.key.as_str() == claim),
            "{claim}: {:?}",
            report.unbound
        );
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(start.floor),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: i32::from(start.x),
                    y: i32::from(start.y),
                    floor: -i16::from(start.floor),
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated_pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        let weights = NativeItemWeights(native);
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        harness
            .root
            .initialize_admission_equipment(
                &authority,
                &harness.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        harness
            .root
            .grant_starter_items(&authority, &harness.node, &fence, &weights, &template)
            .await
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let key = report
            .served
            .iter()
            .find(|entry| entry.key.as_str() == claim)
            .and_then(|entry| {
                entry
                    .placements
                    .iter()
                    .find(|p| p.cell.x == chest.0 && p.cell.y == chest.1)
            })
            .ok_or("bound source placement")?
            .bundle_placement_key;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        let south = report
            .served
            .iter()
            .find(|c| c.key.as_str().ends_with("magic_sword.south"))
            .map(|c| {
                let mut placement = world.base().digest().to_vec();
                placement.extend_from_slice(&c.placements[0].bundle_placement_key.to_be_bytes());
                WorldObjectTarget {
                    placement,
                    expected_revision: 0,
                }
            });
        let mut qualified_catalogue = None;
        let admission_start = admitted;
        for cold in [false, true] {
            admitted = admission_start;
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.quest_catalogue = qualified_catalogue.clone().or(Some(std::sync::Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            )));
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            owner.admit_quest_session(&admitted).await;
            deps.initialize(&owner).await;
            let mut stale = fence;
            stale.character_lease_generation += 1;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(1))
                .ok_or("clock")?;
            let refused = owner
                .use_bundle_chest_from_shards(
                    deps.actor,
                    crate::gameplay_transport::connection::UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 99,
                        item_fence: Some(stale),
                    },
                    owner.bundle_map().ok_or("bundle map")?,
                    &target,
                    &shards,
                )
                .await
                .ok_or("binding")?;
            assert_eq!(refused.disposition, UseDisposition::Rejected);
            let before: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_reward_claim_mint_reservations WHERE command_id=99",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(
                before, 0,
                "stale authority cannot reserve an authored reward"
            );
            if !cold {
                seed(&mut owner, admitted.game_session_id, &callback, &siblings)
                    .await
                    .map_err(|e| format!("seed route prerequisites: {e}"))?;
                qualified_catalogue = owner.quest_catalogue.clone();
                if suffix.starts_with("magic") {
                    super::l6_paradox::raise_level(&owner, &admitted, &authority, &harness, 100)
                        .await?;
                    owner
                        .refresh_quest_session(admitted.game_session_id, false)
                        .await;
                }
            }
            let result = wire(&owner, &mut admitted, &[1, 2], &target).await?;
            assert_eq!(
                result,
                vec![UseDisposition::Committed, UseDisposition::Rejected],
                "{suffix} cold={cold}"
            );
            if let Some(south) = &south {
                let before:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL").bind(&callback.quest).fetch_one(&harness.pool).await?;
                assert_eq!(
                    before,
                    i64::from(cold),
                    "north alone must not complete the quest"
                );
                assert_eq!(
                    wire(&owner, &mut admitted, &[3, 4], south).await?,
                    vec![UseDisposition::Committed, UseDisposition::Rejected]
                );
            }
            let bag = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("bag")?;
            let expected = siblings
                .iter()
                .flat_map(|c| c.give.iter())
                .flat_map(|r| {
                    std::iter::once((&r.item, u32::from(r.count)))
                        .chain(r.additional.iter().map(|i| (&i.item, u32::from(i.count))))
                })
                .collect::<Vec<_>>();
            assert_eq!(bag.entries.len(), expected.len());
            let mut actual = bag
                .entries
                .iter()
                .map(|entry| (&entry.item.definition.production_key, entry.item.quantity))
                .collect::<Vec<_>>();
            let mut expected = expected;
            actual.sort();
            expected.sort();
            assert_eq!(actual, expected);
        }
        let claims: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid), (SELECT count(*) FROM game_reward_claim_mint_receipts)")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
        let count = i64::try_from(siblings.len())?;
        assert_eq!(claims, (count, count));
        let completed:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL),(SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes)")
            .bind(&callback.quest).fetch_one(&harness.pool).await?;
        assert_eq!(completed, (1, 1));
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn wire(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    commands: &[u64],
    target: &WorldObjectTarget,
) -> TestResult<Vec<UseDisposition>> {
    let commands = commands.iter().map(|&id| (id, target)).collect::<Vec<_>>();
    Ok(
        super::quest_callbacks::wire_use_targets_retained(owner, admitted, &commands)
            .await?
            .into_iter()
            .map(|(_, disposition)| disposition)
            .collect(),
    )
}

async fn seed(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
    callback: &oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback,
    siblings: &[oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback],
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    };
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut keys = siblings
        .iter()
        .flat_map(|c| {
            c.storage
                .iter()
                .map(|s| s.track.clone())
                .chain(c.requires.iter().map(|r| r.track.clone()))
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    let tracks = keys
        .iter()
        .map(|k| canonical.track(k).cloned().ok_or("track"))
        .collect::<Result<Vec<_>, _>>()?;
    let setup = "oteryn:quest-transition/engclaim-pg-setup";
    let mut transitions = siblings
        .iter()
        .map(|c| canonical.transition(&c.key).cloned().ok_or("transition"))
        .collect::<Result<Vec<_>, _>>()?;
    transitions.push(QuestTransition {
        world_change: None,
        key: setup.into(),
        quest: callback.quest.clone(),
        completes: false,
        experience: None,
        effects: callback
            .requires
            .iter()
            .map(|r| QuestEffect {
                track: r.track.clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(r.minimum),
            })
            .collect(),
    });
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(canonical.content_revision(), tracks, transitions)
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    let result = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: setup.into(),
                cause: QuestCause::Command(CommandRef::new(
                    session,
                    CommandId::new(100).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    owner.refresh_quest_session(session, false).await;
    Ok(())
}

fn execute(suffix: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || claim_case(suffix).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M34 scenario panic")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED engclaim_m34 {suffix} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
#[test]
fn engclaim_m34_kormarak_production_multi_reward_once() -> TestResult {
    execute("kormarak-corpse")
}
#[test]
fn engclaim_m34_magic_sword_production_chosen_vault_once() -> TestResult {
    execute("magic_sword-north")
}
