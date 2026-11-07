//! C19b: the Newhaven profession kit is granted by production first entry, once.
use super::fixture::{Harness, debug, id, run};
use super::tutorial_vitals_fixture::{admit, native_asset_revisions, publish_native_activation};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};

#[test]
fn newhaven_starter_first_entry_grants_profession_equipment_and_supplies_once_postgres()
-> TestResult {
    scenario(KitCase::Initial)
}

#[test]
fn newhaven_starter_extra_placement_rolls_back_entire_kit_postgres() -> TestResult {
    scenario(KitCase::Rollback)
}

#[test]
fn newhaven_starter_history_registered_legacy_root_progresses_and_keeps_backpack_postgres()
-> TestResult {
    scenario(KitCase::Legacy)
}
#[test]
fn newhaven_starter_history_unregistered_legacy_root_refuses_progression_postgres() -> TestResult {
    scenario(KitCase::Unregistered)
}

#[test]
fn newhaven_starter_live_grant_publishes_inventory_equipment_without_relogin_postgres() -> TestResult
{
    scenario(KitCase::Live)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KitCase {
    Initial,
    Rollback,
    Legacy,
    Unregistered,
    Live,
}

fn scenario(case: KitCase) -> TestResult {
    let inject_extra = case == KitCase::Rollback;
    let legacy = matches!(case, KitCase::Legacy | KitCase::Unregistered);
    let unregistered = case == KitCase::Unregistered;
    let live = case == KitCase::Live;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("newhaven-starter", move || {
        run(async move |admin| {
            let h = if legacy {
                Harness::create_with_admission_seed(admin, "starter_history", false, false).await?
            } else {
                Harness::create_newhaven(admin, "newhaven_starter").await?
            };
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, pin) =
                super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
            publish_native_activation(&h, &pin).await?;
            let active = controller.active().ok_or("active")?;
            native_asset_revisions(&h, active).await?;
            let room = pin.qualified_room().clone();
            let deps = Dependencies::with_content(
                pin.into_channel_parts().0,
                room,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            let mut admitted = deps.admitted;
            Box::pin(owner.admit_progression_session(&mut admitted)).await;
            let mut live_view = if live {
                admit(&owner, &deps, &request).await?;
                let mut view = crate::gameplay_transport::item_view::SessionItemView::resume(
                    Default::default(),
                )
                .with_equip_drop()
                .with_container_tree();
                let initial = crate::gameplay_transport::item_move::observe_inventory(
                    &owner,
                    deps.actor,
                    deps.admitted.game_session_id,
                    &view,
                )
                .await
                .ok_or("initial inventory")?;
                view.inventory_committed(initial).map_err(debug)?;
                Some(view)
            } else {
                None
            };
            let fence = owner
                .current_quest_fence(deps.admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            let formula = active
                .native_gameplay()
                .and_then(|n| n.training_formula())
                .ok_or("formula")?;
            h.root
                .commit_character_build(
                    &authority,
                    &h.node,
                    fence,
                    BuildChangeRequest {
                        occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                        cause: BuildCause::VocationChoice,
                        before: DurableBuildState::default(),
                        after: convert_vocation(formula, &DurableBuildState::default(), "paladin")
                            .map_err(debug)?,
                        pruned_stance: None,
                    },
                    formula,
                )
                .await
                .map_err(debug)?;
            let mut legacy_items: Vec<(String, String, i64)> = Vec::new();
            if legacy {
                admit(&owner, &deps, &request).await?;
                legacy_items=sqlx::query_as("SELECT item_instance_id::text,definition_production_key,quantity FROM game_item_instances ORDER BY item_instance_id LIMIT 100").fetch_all(&h.pool).await?;
                assert!(legacy_items.len() < 100);
                use crate::durability::character_progression::{
                    CharacterProgressionError, ExperienceAwardRequest, ExperienceRewardOccurrence,
                };
                use oteryn_simulation_determinism::ExactI64;
                let progression = owner
                    .progression_binding(deps.admitted.game_session_id)
                    .ok_or("progression")?;
                sqlx::query("SELECT game_character_configure_interpretation('profile-1','ruleset-1','content-1','newhaven-starter-r1')").execute(&h.pool).await?;
                if unregistered {
                    let removed=sqlx::query("DELETE FROM game_world_registered_starter_revisions WHERE world_id=encode($1,'hex')::uuid AND starter_template_revision='starter-1'").bind(id(42).as_slice()).execute(&h.pool).await?;
                    assert_eq!(removed.rows_affected(), 1);
                }
                if !unregistered {
                    for (index, (profile, ruleset, content)) in [
                        ("profile-other", "ruleset-1", "content-1"),
                        ("profile-1", "ruleset-other", "content-1"),
                        ("profile-1", "ruleset-1", "content-other"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        sqlx::query("SELECT game_character_configure_interpretation($1,$2,$3,'newhaven-starter-r1')").bind(profile).bind(ruleset).bind(content).execute(&h.pool).await?;
                        let fence = owner
                            .current_quest_fence(deps.admitted.game_session_id)
                            .await
                            .map_err(|()| "fence")?
                            .ok_or("fence")?;
                        let refused = h
                            .root
                            .commit_character_experience(
                                &authority,
                                &h.node,
                                fence,
                                ExperienceAwardRequest {
                                    occurrence: ExperienceRewardOccurrence::from_bytes(id(
                                        u8::try_from(100 + index)?,
                                    ))
                                    .map_err(debug)?,
                                    amount: ExactI64::new(100),
                                    context: progression.context.clone(),
                                    policy_revision: progression.policy_revision.clone(),
                                    reward_revision: progression.reward_revision.clone(),
                                    policy: progression.policy.clone(),
                                },
                            )
                            .await;
                        assert!(
                            matches!(
                                refused,
                                Err(CharacterProgressionError::ProgressionContextMismatch)
                            ),
                            "other revisions must stay current: {refused:?}"
                        );
                    }
                    sqlx::query("SELECT game_character_configure_interpretation('profile-1','ruleset-1','content-1','newhaven-starter-r1')").execute(&h.pool).await?;
                }
                let fence = owner
                    .current_quest_fence(deps.admitted.game_session_id)
                    .await
                    .map_err(|()| "fence")?
                    .ok_or("fence")?;
                let result = h
                    .root
                    .commit_character_experience(
                        &authority,
                        &h.node,
                        fence,
                        ExperienceAwardRequest {
                            occurrence: ExperienceRewardOccurrence::from_bytes(id(91))
                                .map_err(debug)?,
                            amount: ExactI64::new(100),
                            context: progression.context.clone(),
                            policy_revision: progression.policy_revision.clone(),
                            reward_revision: progression.reward_revision.clone(),
                            policy: progression.policy.clone(),
                        },
                    )
                    .await;
                if unregistered {
                    assert!(
                        matches!(
                            result,
                            Err(CharacterProgressionError::ProgressionContextMismatch)
                        ),
                        "{result:?}"
                    );
                    assert!(
                        h.root
                            .read_character_root_interpretation(&authority, fence.character_id)
                            .await
                            .map_err(debug)?
                            .is_none()
                    );
                    drop(owner);
                    drop(authority);
                    drop(seal);
                    return h.cleanup().await;
                }
                assert!(
                    result.is_ok(),
                    "registered historical starter must allow actual progression: {result:?}"
                );
                let stored = h
                    .root
                    .read_character_root_interpretation(&authority, fence.character_id)
                    .await
                    .map_err(debug)?
                    .ok_or("historical interpretation")?;
                assert_eq!(stored.revisions()[3], "starter-1");
            }
            if inject_extra {
                // Inject a second custody row in the exact kit mint transaction. Its kit receipt
                // names equipment, so the unrelated backpack row must never inherit that proof.
                sqlx::query("CREATE FUNCTION fixture_extra_starter_placement() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
                    IF NEW.slot=5 THEN
                        INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id)
                        SELECT NEW.character_id,s.item_instance_id,NEW.item_instance_id,NEW.world_id,
                            coalesce((SELECT max(placement_ordinal) FROM game_item_container_entries WHERE character_id=NEW.character_id),0)+1,
                            NEW.placed_transaction_id
                        FROM game_item_container_slots s WHERE s.character_id=NEW.character_id;
                    END IF;
                    RETURN NEW;
                END $$").execute(&h.pool).await?;
                sqlx::query("CREATE TRIGGER fixture_extra_starter_placement AFTER INSERT ON game_character_equipment_slots FOR EACH ROW EXECUTE FUNCTION fixture_extra_starter_placement()")
                    .execute(&h.pool).await?;
                let (outcome, _, _) = Box::pin(
                    owner.initialize_first_entry(
                        &request,
                        deps.admitted.game_session_id,
                        crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16])
                            .map_err(debug)?,
                        deps.actor,
                    ),
                )
                .await;
                assert_eq!(
                    outcome,
                    crate::gameplay_transport::connection::FirstEntryOutcome::RefusedUnavailable
                );
                let kit_rows:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_starter_grants WHERE template_key LIKE 'oteryn:starter.newhaven.%'").fetch_one(&h.pool).await?;
                assert_eq!(
                    kit_rows, 0,
                    "a refused destination must roll back every kit receipt"
                );
                let equipment: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_slots")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(
                    equipment, 0,
                    "no earlier kit equipment may survive the refusal"
                );
                sqlx::query("DROP TRIGGER fixture_extra_starter_placement ON game_character_equipment_slots").execute(&h.pool).await?;
                sqlx::query("DROP FUNCTION fixture_extra_starter_placement()")
                    .execute(&h.pool)
                    .await?;
            }
            if let Some(view) = live_view.as_mut() {
                use crate::gameplay_transport::connection::FreshAdmissionAuthority;
                use tokio::io::AsyncReadExt;
                let before = view.continuity().inventory_revision;
                let fence = owner
                    .current_quest_fence(deps.admitted.game_session_id)
                    .await
                    .map_err(|()| "fence")?
                    .ok_or("fence")?
                    .item_fence();
                assert_eq!(
                    owner.grant_newhaven_starter(fence).await.map_err(debug)?,
                    crate::durability::starter_grant::NewhavenStarterOutcome::Granted
                );
                let (mut writer, mut reader) = tokio::io::duplex(256 * 1024);
                let mut admitted = deps.admitted;
                admitted.item_fence = Some(fence);
                let mut sequence = 1;
                crate::gameplay_transport::connection::refresh_newhaven_inventory(
                    &mut writer,
                    &owner,
                    deps.actor,
                    &mut admitted,
                    Some(&mut *view),
                    None,
                    &mut sequence,
                )
                .await
                .ok_or("live kit publisher")?;
                drop(writer);
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).await?;
                let mut cursor = 0;
                let mut published = None;
                while cursor < bytes.len() {
                    let size =
                        usize::try_from(u32::from_be_bytes(bytes[cursor..cursor + 4].try_into()?))?;
                    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(
                        &bytes[cursor + 4..cursor + 4 + size],
                    )
                    .map_err(debug)?;
                    cursor += 4 + size;
                    if envelope.message_type() == oteryn_protocol_oteryn::MessageType::StateDelta {
                        let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                            .map_err(debug)?;
                        if delta.domain_id
                            == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                        {
                            published=Some(oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(delta.payload,true).map_err(debug)?);
                        }
                    }
                }
                let published = published.ok_or("live inventory delta must be transmitted")?;
                assert_eq!(published.equipment.len(), 7);
                assert_eq!(published.entries.len(), 4);
                assert!(view.continuity().inventory_revision > before);
                assert!(!owner.take_newhaven_inventory_changed(deps.admitted.game_session_id));
            } else {
                admit(&owner, &deps, &request).await?;
            }
            let items: Vec<(String, i64)> = sqlx::query_as("SELECT definition_production_key,quantity FROM game_item_instances WHERE definition_production_key IN ('oteryn:item.tibia.i3350','oteryn:item.tibia.i35562','oteryn:item.tibia.i3447','oteryn:item.tibia.i268','oteryn:item.tibia.i266','oteryn:item.tibia.i3031','oteryn:item.tibia.i3578') ORDER BY definition_production_key,quantity")
                .fetch_all(&h.pool).await?;
            if legacy {
                let retained:Vec<(String,String,i64)>=sqlx::query_as("SELECT item_instance_id::text,definition_production_key,quantity FROM game_item_instances ORDER BY item_instance_id LIMIT 100").fetch_all(&h.pool).await?;
                assert_eq!(
                    retained, legacy_items,
                    "historical loadout items and quantities are retained exactly"
                );
                let new_kits: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_character_starter_grants WHERE kit_destination>0",
                )
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(new_kits, 0, "old roots never receive the new Newhaven kit");
                let bags:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_slots s JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i2854'").fetch_one(&h.pool).await?;
                assert_eq!(bags, 1, "old backpack component remains intact");
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            assert_eq!(
                items,
                vec![
                    ("oteryn:item.tibia.i266".into(), 5),
                    ("oteryn:item.tibia.i268".into(), 7),
                    ("oteryn:item.tibia.i3031".into(), 50),
                    ("oteryn:item.tibia.i3350".into(), 1),
                    ("oteryn:item.tibia.i3447".into(), 100),
                    ("oteryn:item.tibia.i3447".into(), 100),
                    ("oteryn:item.tibia.i35562".into(), 1),
                    ("oteryn:item.tibia.i3578".into(), 1),
                ]
            );
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
                .fetch_one(&h.pool)
                .await?;
            if live {
                let fence = owner
                    .current_quest_fence(deps.admitted.game_session_id)
                    .await
                    .map_err(|()| "fence")?
                    .ok_or("fence")?
                    .item_fence();
                assert_eq!(
                    owner.grant_newhaven_starter(fence).await.map_err(debug)?,
                    crate::durability::starter_grant::NewhavenStarterOutcome::AlreadyGranted
                );
            } else {
                admit(&owner, &deps, &request).await?;
            }
            let replay: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(replay, count, "reconciliation cannot duplicate the kit");
            let mut stale = owner
                .current_quest_fence(deps.admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?
                .item_fence();
            stale.connection_generation =
                crate::foundation::ConnectionGeneration::new(999).map_err(debug)?;
            assert!(
                owner.grant_newhaven_starter(stale).await.is_err(),
                "a historical kit receipt is not current session authority"
            );
            let after_stale: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(after_stale, count);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "C19B_PG_CASE_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
