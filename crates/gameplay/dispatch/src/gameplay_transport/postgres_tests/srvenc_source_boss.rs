//! W02: genuine B3 geometry and the captured Sugar Daddy lever reach the
//! production template constructor, native Instance owner and death settlement.
//! Crystal 201189f data-global/scripts/quests/candia_bosses/sugar_daddy_lever.lua:1-23;
//! Canary 47dfd51f data-otservbr-global/monster/bosses/sugar_daddy.lua:21-25,76-132.
use super::fixture::{self, Harness, debug, run};
use super::{Dependencies, TestResult, reset};
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{NativeEntryStart, ReferenceItemField, ReferenceItemType};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{
    ChannelContentPin, CharacterLease, CommandId, CommandRef, MovementLocalPosition,
    RuntimeScopeRefV1,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::activity_scope::ActivityScope;
use oteryn_protocol_oteryn::boss_encounter::{self as wire, BossActivityReply};
use oteryn_world_bundle_compiler::{bundle, compile, source};

const BOSS: &str = "oteryn:boss.sugar_daddy";
type PublishedRoom = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, String, String, i64);

const CREATURE: &str = "oteryn:creature.sugar_daddy";
const LEVER: MovementLocalPosition = MovementLocalPosition {
    x: 33339,
    y: 32221,
    floor: 9,
};
const ENTRY: MovementLocalPosition = MovementLocalPosition {
    x: 33372,
    y: 32234,
    floor: 9,
};
const EXIT: MovementLocalPosition = MovementLocalPosition {
    x: 33397,
    y: 32201,
    floor: 9,
};

/// Boot exact current Item definition references and policy flags, as
/// node/serve.rs::boot_bundle_items does; no test-wide walkability override.
pub(super) fn source_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    native: &NativeGameplayState,
    candidate: &crate::content::bosses::activity_template::BossLeverCandidate,
) -> TestResult<crate::map::boot::BundleWorld> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let project = source::project_regions(&root, &[(130, 125)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let identity: bundle::Identity = serde_json::from_slice(&source::read(
        &root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let input = compile::Input {
        regions: &project.regions,
        palette: &palette,
        identity: identity.clone(),
        world: source::world_extent(&root)?,
        build_class: bundle::BuildClass::NonProduction,
        draft_areas: vec![],
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    let low = candidate.zone_from.ok_or("source room lower bound")?;
    let high = candidate.zone_to.ok_or("source room upper bound")?;
    let held: std::collections::BTreeSet<_> = compiled
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            let at = [
                i32::from(diagnostic.x),
                i32::from(diagnostic.y),
                -i32::from(diagnostic.floor),
            ];
            let in_room = (low[0]..=high[0]).contains(&at[0])
                && (low[1]..=high[1]).contains(&at[1])
                && (low[2]..=high[2]).contains(&at[2]);
            in_room
                || candidate
                    .lever_positions
                    .iter()
                    .chain(&candidate.entry_positions)
                    .chain(candidate.boss_position.iter())
                    .chain(candidate.exit_position.iter())
                    .any(|anchor| *anchor == at)
        })
        .map(|diagnostic| diagnostic.key.as_str())
        .collect();
    assert!(
        held.is_empty(),
        "a real source boss room cannot silently omit provisional geometry: {held:?}"
    );
    let parity = compile::equivalence(&input, &registry, &compiled.bytes)?;
    assert!(parity.tiles > 0 && parity.entries > 0);
    let pins = crate::map::boot::BootPins {
        bundle: crate::map::BundlePins {
            digest: compiled.digest,
            project_format_version: identity.project_format_version,
            world_schema_version: identity.world_schema_version,
            content_revision: identity.content_revision,
            production: false,
        },
        map_revision: crate::map::overlay::internal_map_revision(
            &compiled
                .digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        ),
        start: crate::map::overlay::TilePos {
            x: 33339,
            y: 32221,
            floor: -9,
        },
    };
    let index = native.item_index().ok_or("active native Item index")?;
    crate::map::boot::boot(&compiled.bytes, &pins, world, channel, |key| {
        let revision = index.revision(key)?;
        let reference = index.definition_ref("Item", key, revision)?;
        let profile = native
            .item_policy(key, revision)
            .map(|policy| policy.record());
        let pickupable = profile.is_some_and(|profile| {
            matches!(
                &profile.semantics.physical, ReferenceItemField::Known(physical)
                    if physical.pickupable == ReferenceItemField::Known(true)
            )
        });
        let door = profile.is_some_and(|profile| matches!(
            &profile.semantics.classification, ReferenceItemField::Known(classification)
                if classification.item_type == ReferenceItemField::Known(ReferenceItemType::Door)
        ));
        Some(crate::map::facts::ItemDefinition {
            reference,
            solid: profile.and_then(|profile| profile.attributes.blocks_movement),
            blocks_projectile: profile
                .and_then(|profile| profile.attributes.blocks_projectile)
                .unwrap_or(true),
            pickupable,
            door,
        })
    })
    .map_err(debug)
}

#[test]
fn srvenc_source_sugar_daddy_template_creates_instance_and_settles_personal_rewards_postgres()
-> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "source boss scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "srvenc_source_sugar", true, false).await?;
        let evidence = super::house_transfer::RecoverySource::start(&h).await?;
        super::house_transfer::prepare_owned_session(&h).await?;
        let expected = fixture::fence(1)?;
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = expected.runtime_scope
        else {
            return Err("source Channel".into());
        };
        let (controller, activation) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world_id, None,
            );
        let generation = controller.active().ok_or("active gameplay")?;
        let native = generation.native_gameplay().ok_or("native gameplay")?;
        let source_room = activation.qualified_room().clone();
        let activation_pin = activation.into_channel_pin();
        let candidate = crate::content::bosses::activity_template::captured_boss_levers()
            .map_err(debug)?
            .into_iter()
            .find(|candidate| candidate.creature_key.as_deref() == Some(CREATURE))
            .ok_or("captured Sugar lever")?;
        let bundle = source_bundle(world_id, channel_id, native, &candidate)?;
        let cells = bundle
            .movement_cells(source_room.movement_cells())
            .map_err(debug)?;
        let room = source_room
            .bundle_view(
                cells.clone(),
                bundle.frame_binding_digest(world_id),
                bundle.map_revision_digest(),
                NativeEntryStart {
                    x: LEVER.x,
                    y: LEVER.y,
                    floor: LEVER.floor,
                },
            )
            .ok_or("source bundle combat room")?;
        let template = controller
            .qualify_activity_template(&source_room, &room, "content-1", &candidate)
            .map_err(|reason| {
                let anchors:Vec<_> = candidate.lever_positions.iter().chain(&candidate.entry_positions)
                    .chain(candidate.boss_position.iter()).chain(candidate.exit_position.iter())
                    .map(|&[x,y,z]| ([x,y,z],room.movement_cells().index().lookup(
                        room.movement_cells().scope(),crate::content::LogicalCell {x,y,z},
                    ))).collect();
                format!("Sugar source qualification {reason:?}; actual bundle anchor collisions: {anchors:?}")
            })?;
        assert_eq!(template.boss_key(), BOSS);
        assert_eq!(
            template.boss_position(),
            crate::content::LogicalCell {
                x: 33369,
                y: 32220,
                z: 9
            }
        );
        let policy = native
            .policies()
            .policies()
            .iter()
            .find(|policy| policy.definition_key == CREATURE)
            .ok_or("native Sugar policy")?;
        assert_eq!(policy.reward_boss, Some(true));
        let rewards = native.reward_table().row(CREATURE).map_err(debug)?;
        assert_eq!(
            rewards.corpse_item.production_key,
            "oteryn:item.tibia.i48416"
        );
        assert!(!rewards.loot_table.entries.is_empty());
        let pin = ChannelContentPin::from_activation(
            world_id,
            generation.activation_sequence(),
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            bundle.frame_binding_digest(world_id),
            bundle.map_revision_digest(),
            (LEVER.x, LEVER.y, LEVER.floor),
        );
        assert_ne!(
            activation_pin.frame_binding_digest(),
            pin.frame_binding_digest(),
            "source Content issuance and bundle runtime retain distinct frame bindings"
        );
        let mut deps = Dependencies::with_content(
            pin.clone(),
            source_room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.combat_room = Some(room);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.evidence = &evidence.evidence;
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(
            FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0,
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[wire::CAPABILITY_BOSS_ENCOUNTER_V1],
        )
        .ok_or("boss capability")?;
        let fence = admitted.item_fence.ok_or("current source item fence")?;
        super::item_look::seed_main_backpack(&h, &[]).await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
            .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            assert_eq!(
                runtime
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                LEVER
            );
        }
        deps.initialize(&owner).await;
        owner
            .register_current_runtime()
            .await
            .map_err(|_| "register source runtime")?;
        owner
            .activity_runtimes
            .remember(admitted)
            .map_err(|_| "remember source connection")?;
        owner
            .activity_runtimes
            .register_template(&template)
            .map_err(|_| "register source template")?;
        super::boss_activity::issue_content_and_reset_with_runtime_pin(
            &h,
            &activation_pin,
            &pin,
            bundle.map_revision(),
        )
        .await?;
        let catalogue =
            crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone());
        catalogue
            .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template))
            .await?;
        catalogue
            .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template))
            .await?;
        let published: PublishedRoom = sqlx::query_as(
            "SELECT origin_frame_digest,origin_map_digest,server_artifact_digest,activation_frame_digest,content_revision,template_revision,count(*) OVER () FROM game_boss_room_publications WHERE world_id=encode($1,'hex')::uuid AND boss_key=$2",
        ).bind(world_id.as_bytes().as_slice()).bind(BOSS).fetch_one(&h.pool).await?;
        assert_eq!(
            published.0.as_slice(),
            pin.frame_binding_digest().as_slice()
        );
        assert_eq!(published.1.as_slice(), pin.map_revision_digest().as_slice());
        assert_eq!(
            published.2.as_slice(),
            pin.server_artifact_digest().as_slice()
        );
        assert_eq!(
            published.3.as_slice(),
            activation_pin.frame_binding_digest().as_slice()
        );
        assert_eq!(published.4, "content-1");
        assert_eq!(published.5, template.template_revision());
        assert_eq!(
            published.6, 1,
            "replaying the sealed source publication is idempotent"
        );
        let instance = super::activity_runtime::allocate_ready_instance(&h, 88).await?;
        let assignment = h
            .root
            .read_runtime_scope_assignment(instance)
            .await
            .map_err(debug)?
            .ok_or("ready Instance allocation")?;
        assert!(
            owner
                .activity_runtimes
                .get(instance, assignment.ownership_generation)
                .is_none(),
            "only the production template caller may instantiate the destination"
        );
        let old = admitted;
        let (_, reply, barrier) =
            super::boss_activity::exchange(&owner, &mut admitted, BOSS, true).await?;
        let BossActivityReply::Accepted {
            handoff,
            deadline_unix_ms: Some(_),
        } = reply
        else {
            return Err(format!("source Sugar entry refused: {reply:?}").into());
        };
        let barrier = barrier.ok_or("actual scope barrier")?;
        let RuntimeScopeRefV1::Instance { instance_id, .. } = instance else {
            return Err("Instance allocation".into());
        };
        assert_eq!(
            barrier.scope,
            ActivityScope::Instance {
                world: *world_id.as_bytes(),
                instance: instance_id
            }
        );
        assert_eq!(barrier.handoff, handoff);
        assert_eq!(barrier.source_session, *old.game_session_id.as_bytes());
        assert_eq!(barrier.session, *admitted.game_session_id.as_bytes());
        let destination = owner
            .resolve_runtime_view(admitted)
            .await
            .map_err(|_| "resolve production Instance")?
            .ok_or("Instance owner switched")?;
        assert_eq!(
            destination
                .bundle_map()
                .ok_or("Instance map")?
                .overlay
                .runtime_scope(),
            instance
        );
        assert!(
            destination
                .bundle_map()
                .ok_or("Instance map")?
                .overlay
                .base()
                .tile(33339, 32221, -9)
                .is_none(),
            "the Instance cannot expose the source lever outside its bounds"
        );
        {
            let runtime = destination.runtime.lock().await;
            assert_eq!(runtime.binding().runtime_scope(), instance);
            assert_eq!(
                runtime
                    .read_actor_position(admitted.runtime_actor.ok_or("Instance player")?)
                    .map_err(debug)?
                    .position(),
                ENTRY
            );
            let creatures = runtime.visible_entities().creatures;
            assert_eq!(creatures.len(), 1);
            assert_eq!(
                runtime
                    .companion_snapshot(creatures[0].actor)
                    .map_err(debug)?
                    .state
                    .policy
                    .definition_key,
                CREATURE
            );
        }
        settle_source_boss(&h, &destination, admitted, policy).await?;
        let exit_deadline: (i64,i64) = sqlx::query_as("SELECT terminal_at,exit_deadline FROM game_boss_room_activities WHERE admission=encode($1,'hex')::uuid")
            .bind(handoff.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            exit_deadline.1 - exit_deadline.0,
            60_000,
            "the source after-kill timer is unchanged"
        );
        // Source boss_lever_death.lua:21-32 retains its actual sixty-second exit.
        // Wait for the authoritative SQL deadline, never shorten authored policy.
        loop {
            let now: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
            )
            .fetch_one(&h.pool)
            .await?;
            if now >= exit_deadline.1 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(u64::try_from(
                exit_deadline.1 - now,
            )?))
            .await;
        }
        let (_, returned, barrier) = super::boss_activity::exchange_command(
            &owner,
            &mut admitted,
            BOSS,
            true,
            wire::COMMAND_TYPE_BOSS_DEFEATED,
        )
        .await?;
        assert!(matches!(returned, BossActivityReply::Accepted { .. }));
        assert!(matches!(
            barrier.ok_or("return barrier")?.scope,
            ActivityScope::Channel { .. }
        ));
        let returned_fence = admitted.item_fence.ok_or("returned source fence")?;
        assert_eq!(returned_fence.runtime_scope, expected.runtime_scope);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(admitted.runtime_actor.ok_or("returned player")?)
                .map_err(debug)?
                .position(),
            EXIT
        );
        let retained_rewards: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_boss_reward_items")
                .fetch_one(&h.pool)
                .await?;
        assert!(
            retained_rewards > 0,
            "returning from the private room must retain personal reward custody"
        );
        eprintln!(
            "SRVENC_SOURCE_SUGAR_TEMPLATE_INSTANCE_CORPSE_REWARDS_RETURN POSTGRES_CASE_COMPLETED"
        );
        Ok(())
    })
}

async fn settle_source_boss(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    policy: &crate::foundation::CompiledCreaturePolicy,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("Instance item fence")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("Instance death required".into());
    };
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("active rewards")?;
    let now = owner.owner_now();
    let map = owner.bundle_map().ok_or("current Instance bundle map")?;
    let cached_before_death = crate::gameplay_transport::loop_move::loop_ground(
        owner,
        map.overlay.base(),
        native.item_index().map(AsRef::as_ref),
    )
    .await
    .ok_or("prime ordinary Ground cache before real death")?;
    assert!(
        !cached_before_death
            .overlay()
            .has_added_item(crate::map::overlay::TilePos {
                x: 33369,
                y: 32220,
                floor: -9,
            })
    );
    let batch = {
        let mut runtime = owner.runtime.lock().await;
        let target = runtime
            .visible_entities()
            .creatures
            .first()
            .ok_or("actual Sugar actor")?
            .actor;
        let batch = OwnerCombatBatch {
            caster: admitted.runtime_actor.ok_or("native player")?,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(admitted.game_session_id, CommandId::new(91).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "srvenc:source-sugar-lethal".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"srvenc-source-sugar-lethal".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                    magnitude: policy.maximum_health,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(receipt.applied);
        assert!(receipt.effects.iter().any(|effect| effect.target == target
            && effect.health.is_some_and(|health| health.health_after == 0)));
        let attack = owner.states.attack.lock().await;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
        assert!(attack.kills().has_session(admitted.game_session_id));
        batch
    };
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    let corpse:(String,bool,Vec<u8>)=sqlx::query_as("SELECT i.definition_production_key,g.channel_id IS NULL,uuid_send(g.instance_id) FROM game_item_mint_receipts r JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_item_instances i USING(item_instance_id) WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION'").fetch_one(&h.pool).await?;
    assert_eq!(
        (corpse.0.as_str(), corpse.1, corpse.2.as_slice()),
        ("oteryn:item.tibia.i48416", true, instance_id.as_slice())
    );
    let map = owner.bundle_map().ok_or("current Instance bundle map")?;
    let ground = crate::gameplay_transport::loop_move::loop_ground(
        owner,
        map.overlay.base(),
        native.item_index().map(AsRef::as_ref),
    )
    .await
    .ok_or("native physical corpse map projection")?;
    assert!(
        !std::sync::Arc::ptr_eq(&cached_before_death, &ground),
        "committed corpse settlement must invalidate the pre-death ordinary map cache"
    );
    assert!(
        ground
            .overlay()
            .has_added_item(crate::map::overlay::TilePos {
                x: 33369,
                y: 32220,
                floor: -9
            }),
        "the actual source corpse must be usable through the ordinary Instance map projection"
    );
    let entitlements:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_boss_reward_deaths WHERE scope_kind=2 AND settlement_outcome=1),(SELECT count(*) FROM game_boss_reward_items),(SELECT count(*) FROM game_item_corpse_container_entries)").fetch_one(&h.pool).await?;
    assert_eq!(entitlements.0, 1);
    assert!(
        entitlements.1 > 0,
        "source personal rewards must be retained"
    );
    assert_eq!(
        entitlements.2, 0,
        "personal loot is never duplicated in the physical corpse"
    );
    {
        let mut runtime = owner.runtime.lock().await;
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        assert!(!runtime.commit_spell_batch(staged).map_err(debug)?.applied);
    }
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_boss_reward_deaths),(SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION')").fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1));
    Ok(())
}
