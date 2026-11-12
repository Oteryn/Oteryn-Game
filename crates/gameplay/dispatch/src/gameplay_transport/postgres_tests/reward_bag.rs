//! REWARD-CHEST-WIRE-1: USE on a carried reward bag → real composed owner → one atomic
//! consume/mint → durable rows; replay of the consumed bag and a capacity refusal.
use super::fixture::{Harness, debug, id, run};
use super::item_craft::exchange;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_items::reward_bag::{reward_bag_conversion, reward_bag_draw};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget, encode_item_use_intent};

#[test]
fn reward_bag_use_wire_atomic_replay_and_capacity_refusal() -> TestResult {
    run_scenario("oteryn:item.tibia.i43895", None)
}

#[test]
fn reward_bag_desire_use_wire_atomic_replay_and_stale_generation() -> TestResult {
    run_scenario("oteryn:item.tibia.i34109", None)
}

#[test]
fn reward_bag_primal_quiver_use_wire_atomic_replay_and_stale_generation() -> TestResult {
    run_scenario("oteryn:item.tibia.i39546", Some(("oteryn:item.tibia.i39150", 1)))
}

#[test]
fn surprise_bag_use_wire_atomic_replay_and_stale_generation() -> TestResult {
    run_scenario("oteryn:item.tibia.i6570", None)
}

#[test]
fn surprise_bag_quantity_endpoints_and_special_rewards_use_wire() -> TestResult {
    run_scenarios(vec![
        ("oteryn:item.tibia.i6570", Some(("oteryn:item.tibia.i3598", 1))),
        ("oteryn:item.tibia.i6570", Some(("oteryn:item.tibia.i3598", 10))),
        ("oteryn:item.tibia.i6507", Some(("oteryn:item.tibia.i6505", 1))),
        ("oteryn:item.tibia.i12045", Some(("oteryn:item.tibia.i6529", 1))),
        ("oteryn:item.tibia.i21203", Some(("oteryn:item.tibia.i21295", 1))),
        ("oteryn:item.tibia.i8853", Some(("oteryn:item.tibia.i7377", 1))),
    ])
}

#[test]
fn anniversary_box_outcomes_use_wire_and_replay() -> TestResult {
    run_scenarios([
        "oteryn:item.tibia.i39704", "oteryn:item.tibia.i39705",
        "oteryn:item.tibia.i39706", "oteryn:item.tibia.i39710",
    ].into_iter().map(|key| (key, None)).collect())
}

#[test]
fn event_present_and_box_outcomes_use_wire_and_replay() -> TestResult {
    run_scenarios([
        "oteryn:item.tibia.i6506", "oteryn:item.tibia.i6507", "oteryn:item.tibia.i6508",
        "oteryn:item.tibia.i6571", "oteryn:item.tibia.i8853", "oteryn:item.tibia.i12045",
        "oteryn:item.tibia.i14751", "oteryn:item.tibia.i14759", "oteryn:item.tibia.i21203",
        "oteryn:item.tibia.i37457", "oteryn:item.tibia.i37461", "oteryn:item.tibia.i37462",
        "oteryn:item.tibia.i37463", "oteryn:item.tibia.i37465", "oteryn:item.tibia.i37466",
        "oteryn:item.tibia.i37467", "oteryn:item.tibia.i37468", "oteryn:item.tibia.i37469",
        "oteryn:item.tibia.i37614",
    ].into_iter().map(|key| (key, None)).collect())
}

type RewardBagCase = (&'static str, Option<(&'static str, u32)>);
struct RewardBagContent {
    world: crate::foundation::WorldId,
    room: crate::content::QualifiedNativeEntryRoom,
    spells: crate::spell::SpellBook,
    controller: crate::content::ContentActivationController,
    pin: crate::content::NativeEntryContentPin,
}

fn run_scenario(bag_key: &'static str, forced_reward: Option<(&'static str, u32)>) -> TestResult {
    run_scenarios(vec![(bag_key, forced_reward)])
}

fn run_scenarios(cases: Vec<RewardBagCase>) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            // Reuse only immutable content. Every case still creates its own
            // runtime, owner, admission and database, with the same assertions.
            let Dependencies { admitted, room, spells, .. } =
                Dependencies::new().map_err(|error| error.to_string())?;
            let world = admitted.world_id;
            let (controller, pin) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
            let content = RewardBagContent { world, room, spells, controller, pin };
            for (bag_key, forced_reward) in cases {
                scenario(bag_key, forced_reward, &content).map_err(|error| error.to_string())?;
            }
            Ok::<_, String>(())
        })?
        .join()
        .map_err(|_| "reward bag e2e panic")??;
    Ok(())
}

async fn with_case_deadline(
    bag_key: &str,
    case: impl std::future::Future<Output = TestResult>,
) -> TestResult {
    // Each case has two exchanges, whose existing individual limits remain 30s.
    // Also bound setup, direct owner replay and cleanup, which had no deadline.
    tokio::time::timeout(std::time::Duration::from_secs(60), case)
        .await
        .map_err(|_| format!("reward bag {bag_key}: setup, exchange, replay or cleanup did not finish within 60 seconds"))?
        .map_err(|error| format!("reward bag {bag_key}: {error}").into())
}

#[test]
fn reward_bag_unreachable_case_reports_its_deadline() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let started = tokio::time::Instant::now();
            let error = with_case_deadline("unreachable-case", std::future::pending())
                .await
                .err()
                .ok_or("unreachable case unexpectedly completed")?;
            assert!(error.to_string().contains("unreachable-case"));
            assert!(error.to_string().contains("60 seconds"));
            assert_eq!(started.elapsed(), std::time::Duration::from_secs(60));
            Ok(())
        })
}

fn scenario(
    bag_key: &str,
    forced_reward: Option<(&str, u32)>,
    content: &RewardBagContent,
) -> TestResult {
    run(async |admin| with_case_deadline(bag_key, async {
        eprintln!("SV11 PG starting: {bag_key}");
        let harness = Harness::create_with_admission_seed(admin, "reward_bag", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::with_content(
            ChannelContentPin::test(content.world),
            content.room.clone(),
            content.spells.clone(),
        )?;
        let (controller, pin) = (&content.controller, &content.pin);
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let oven = "oteryn:item.tibia.i2535";
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) = &policy.semantics.physical else { return None; };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (6, 0, 7),
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
            content_pin,
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
                    x: 6,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        super::item_look::seed_main_backpack(&harness, &[(bag_key, 1)]).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("reward bag offer selectable")?;
        let bag_definition = crate::gameplay_transport::item_look::typed_definition(index, reference(bag_key).ok_or("bag reference")?).ok_or("bag definition")?;
        let command_id = 1;
        let source = (0..=u16::MAX).find_map(|nonce| {
            let mut source = id(71);
            source[14..].copy_from_slice(&nonce.to_be_bytes());
            if forced_reward.is_none() { return Some(source); }
            let draw = reward_bag_draw(*fence.game_session_id.as_bytes(), command_id, source);
            reward_bag_conversion(native, &bag_definition, draw).and_then(|conversion| {
                forced_reward.is_none_or(|(key, count)| conversion.outputs[0].0.production_key == key && conversion.outputs[0].1 == count)
                    .then_some(source)
            })
        }).ok_or("reachable reward draw")?;
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_item_instances SET item_instance_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice()).bind(source.as_slice()).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_item_container_entries SET item_instance_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice()).bind(source.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let draw = reward_bag_draw(*fence.game_session_id.as_bytes(), command_id, source);
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("source")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let craft_count = async || sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records").fetch_one(&harness.pool).await;
        // A stale session generation cannot consume the still-carried bag.
        let mut stale = fence;
        stale.connection_generation = crate::foundation::ConnectionGeneration::new(
            fence.connection_generation.get() + 1,
        ).map_err(debug)?;
        let refused = owner.use_item_completion(deps.actor, UseCommand {
            game_session_id: deps.admitted.game_session_id,
            command_id,
            item_fence: Some(stale),
        }, ItemKey::Instance(source)).await.ok_or("stale outcome")?;
        assert_eq!(refused.disposition, UseDisposition::Rejected);
        assert_eq!(craft_count().await?, 0);
        let still_carried: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(source.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(still_carried, (1, 1));
        admitted.continuity.next_command_id = command_id;
        let (end, result, _) = exchange(&owner, admitted, command_id, &payload, reference(bag_key)).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(craft_count().await?, 1);
        let expected = reward_bag_conversion(native, &bag_definition, draw).ok_or("admitted reward bag")?;
        let reward = expected.outputs[0].0.production_key.clone();
        // The used bag is retired and its slot holds exactly the drawn reward.
        let retired: (i64, i16, i64) = sqlx::query_as("SELECT quantity,lifecycle,(SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(retired, (0, 2, 0));
        let durable = harness.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("durable backpack")?;
        let keys: Vec<_> = durable.entries.iter().map(|e| (e.item.definition.production_key.as_str(), e.item.quantity)).collect();
        let mut actual = keys;
        actual.sort_unstable();
        let mut expected_keys: Vec<_> = expected.outputs.iter()
            .map(|(definition, count)| (definition.production_key.as_str(), *count)).collect();
        expected_keys.sort_unstable();
        assert_eq!(actual, expected_keys);
        let (recipe, outputs, hash_matches): (String, serde_json::Value, bool) = sqlx::query_as(
            "SELECT r.recipe,r.outputs,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_craft_records r JOIN game_item_audit_outbox a USING(event_id)",
        ).fetch_one(&harness.pool).await?;
        let constant = oteryn_gameplay_items::reward_bag::reward_bags()
            .ok_or("bags")?.iter().find(|b| b.bag == bag_key).ok_or("bag")?;
        assert_eq!(recipe, format!("oteryn:reward-bag.{}", constant.constant.to_ascii_lowercase()));
        assert_eq!(outputs.as_array().ok_or("receipt outputs")?.len(), expected.outputs.len());
        for (output, (definition, count)) in outputs.as_array().ok_or("outputs")?.iter().zip(&expected.outputs) {
            assert_eq!(output["key"], definition.production_key);
            assert_eq!(output["count"], *count);
        }
        assert!(hash_matches);
        // Over capacity: a second bag stays carried and nothing is minted or recorded.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        let levelled = sqlx::query("UPDATE game_character_progression_state SET level=1 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&mut *tx).await?;
        assert_eq!(levelled.rows_affected(), 1);
        let mut seeded = Vec::new();
        // Keep the refusal fixture within the parent's twenty slots, including
        // a large anniversary grant, so the second command reaches the owner.
        let added = 8.min(20 - expected.outputs.len());
        for (n, key) in std::iter::once(bag_key).chain(std::iter::repeat_n("oteryn:item.tibia.i43872", added - 1)).enumerate() {
            seeded.push(sqlx::query_scalar::<_, String>("WITH i AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,'definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id) INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,game_character_uuid_v7() FROM i RETURNING item_instance_id::text")
                .bind(id(42).as_slice()).bind(key).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(100_i64 + i64::try_from(n)?).fetch_one(&mut *tx).await?);
        }
        tx.commit().await?;
        let mut next = end;
        next.continuity.next_command_id = command_id + 1;
        let bag_two: [u8; 16] = sqlx::query_scalar("SELECT uuid_send($1::uuid)").bind(&seeded[0]).fetch_one(&harness.pool).await?;
        let second = reward_bag_conversion(native, &bag_definition,
            reward_bag_draw(*fence.game_session_id.as_bytes(), command_id + 1, bag_two)).ok_or("second conversion")?;
        let capacity_refusal = if expected.outputs.len() + added + second.outputs.len() - 1 > 20 {
            UseDisposition::NoRoom
        } else {
            UseDisposition::NotPossible
        };
        let (_, result, _) = exchange(&owner, next, command_id + 1, &payload, reference(bag_key)).await?;
        assert_eq!(result, capacity_refusal);
        assert_eq!(craft_count().await?, 1);
        let kept: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=$1::uuid").bind(&seeded[0]).fetch_one(&harness.pool).await?;
        assert_eq!(kept, (1, 1));
        let bag_two: [u8; 16] = sqlx::query_scalar("SELECT uuid_send($1::uuid)").bind(&seeded[0]).fetch_one(&harness.pool).await?;
        // A restarted owner replays the consumed bag's receipt without redrawing or minting.
        drop(owner);
        let mut restarted = deps.owner(&harness, &authority);
        restarted.active_generation = Some(active);
        restarted.movement_cells = (&cells).into();
        deps.initialize(&restarted).await;
        let replayed = restarted
            .use_item_completion(deps.actor, UseCommand { game_session_id: deps.admitted.game_session_id, command_id, item_fence: Some(fence) }, ItemKey::Instance(source))
            .await
            .ok_or("replay outcome")?;
        assert_eq!(replayed.disposition, UseDisposition::Committed);
        let refused = restarted
            .use_item_completion(deps.actor, UseCommand { game_session_id: deps.admitted.game_session_id, command_id: command_id + 1, item_fence: Some(fence) }, ItemKey::Instance(bag_two))
            .await
            .ok_or("refused outcome")?;
        assert_eq!(refused.disposition, capacity_refusal);
        assert_eq!(craft_count().await?, 1);
        let minted: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND item_instance_id::text<>ALL($2)").bind(&reward).bind(&seeded).fetch_one(&harness.pool).await?;
        assert_eq!(minted, i64::try_from(expected.outputs.iter().filter(|(definition, _)| definition.production_key == reward).count())?);
        if let Some((key, count)) = forced_reward {
            assert_eq!(expected.outputs[0].1, count);
            assert_eq!(reward, key);
            let children: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_item_nested_entries e JOIN game_item_instances i ON i.item_instance_id=e.parent_item_instance_id WHERE i.definition_production_key=$1) + (SELECT count(*) FROM game_item_container_entries e JOIN game_item_instances i ON i.item_instance_id=e.parent_item_instance_id WHERE i.definition_production_key=$1)")
                .bind(key).fetch_one(&harness.pool).await?;
            assert_eq!(children, 0, "minted reward remains empty after replay");
        }
        drop(restarted);
        drop(authority);
        drop(seal);
        eprintln!("SV11 PG executed: {bag_key}; {} durable outputs; stale, refusal and replay verified", expected.outputs.len());
        harness.cleanup().await
    }).await)
}
