use crate::bestiary_postgres_harness as postgres_fixture;

use super::super::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, SessionContinuity,
};
use super::*;
use crate::foundation::{ChannelId, ChannelRuntimeV1, NodeId, WorldId};
use postgres_fixture::{Harness, TestResult, configured_admin, debug, id, item_fence};
use tokio::sync::Mutex;

mod item_use_view_refresh_tests;
mod wire_dispatch;

// The transport fixture already has an independently fenced active session; it has no
// fresh-admission claim receipt. Supply its snapshot directly from the real item reader,
// and forward both USE forms and container observations to the production owner.
struct WireOwner<'a, 'b, 'f, 's>(&'a ComposedFreshAdmission<'b, 'f, 's>);

impl FreshAdmissionAuthority for WireOwner<'_, '_, '_, '_> {
    async fn admit(
        &self,
        _: super::super::connection::FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, super::super::connection::AdmissionRefusal> {
        Err(super::super::connection::AdmissionRefusal::Unavailable)
    }
    async fn observe(
        &self,
        actor: ExactActorRef,
    ) -> Option<super::super::world_spatial::WorldSpatialObservation> {
        self.0.observe(actor).await
    }
    async fn observe_character_inventory(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<super::super::item_view::InventoryItems> {
        let bag = self
            .0
            .root
            .read_character_backpack(self.0.character, item_fence().ok()?.character_id)
            .await
            .ok()?;
        let runtime = self.0.runtime.lock().await;
        let index =
            super::super::item_ref_admission::pinned_index(&runtime, self.0.active_generation)?;
        super::super::item_view::inventory_from_backpack(bag.as_ref(), |definition| {
            super::super::item_ref_admission::definition_ref(index, definition)
        })
    }
    async fn observe_container(
        &self,
        actor: ExactActorRef,
        fence: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
        target: ItemKey,
    ) -> Option<super::super::container_view::ContainerObservation> {
        self.0.observe_container(actor, fence, target).await
    }
    async fn use_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> UseOutcome {
        self.0.use_item(actor, command, item).await
    }
    async fn use_item_by_definition(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        definition: NonZeroU32,
    ) -> UseOutcome {
        self.0
            .use_item_by_definition(actor, command, definition)
            .await
    }
}

#[test]
fn nested_food_hotkey_and_potion_handle_commit_through_wire_dispatch_on_postgres() -> TestResult {
    run_scenario(None)
}

fn run_scenario(case: Option<item_use_view_refresh_tests::Case>) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            postgres_fixture::runtime()
                .map_err(|error| error.to_string())?
                .block_on(scenario(admin, case))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "item use e2e panicked")?
        .map_err(Into::into)
}

async fn scenario(admin: String, case: Option<item_use_view_refresh_tests::Case>) -> TestResult {
    use item_use_view_refresh_tests::Case;
    let tag = match case {
        None => "bags_use_wire",
        Some(Case::LastCharge) => "bags_use_last",
        Some(Case::PotionFlask) => "bags_use_flask",
        Some(Case::ClosedBag) => "bags_use_closed",
        Some(Case::WithoutContainerCapability) => "bags_use_no_views",
    };
    let harness = Harness::create(admin, tag, false).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let fence = item_fence()?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let channel = ChannelId::decode(&id(43)).map_err(debug)?;
    let session = GameSessionId::decode(&id(50)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let active = controller.active().expect("active generation");
    let native = active.native_gameplay().expect("native content");
    let index = native.item_index().expect("item index");
    let room = pin.qualified_room().clone();
    let food_key = "oteryn:item.tibia.i3577";
    let potion_key = "oteryn:item.tibia.i266";
    let bag_key = "oteryn:item.tibia.i2854";
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key, quantity) in [
        (70, bag_key, 1_i64),
        (80, bag_key, 1),
        (90, bag_key, 1),
        (100, food_key, 3),
        (110, potion_key, 2),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,$5,1,encode($6,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).bind(index.revision(key).expect("production revision"))
            .bind(quantity).bind(id(seed + 1).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(id(80).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(82).as_slice()).execute(&mut *tx).await?;
    for (seed, parent, ordinal) in [(90, 80, 1_i64), (100, 90, 1), (110, 90, 2)] {
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(parent).as_slice()).bind(ordinal).bind(id(seed + 2).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let channel_pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (0, 0, 0),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        channel_pin,
    )
    .map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(debug)?;
    let door_content = crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
        .map_err(debug)?;
    let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
        fence.runtime_scope,
        fence.scope_ownership_generation,
        door_content,
    );
    let door = crate::world_runtime::bind_native_entry_door(
        room.door(),
        &door_fence,
        fence.runtime_scope,
        fence.scope_ownership_generation,
    )
    .map_err(debug)?;
    let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
        "platform-test".into(), ("127.0.0.1".into(), 1), "source.test".into(), "source.test".into(),
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec())],
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec())],
        rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der").to_vec())?,
    ).map_err(debug)?;
    let evidence = crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
    let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
    let achievements =
        crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
    let charms =
        crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
    let runtime = Mutex::new(runtime);
    let door = Mutex::new(door);
    let mut owner = ComposedFreshAdmission {
        emit: |_, _| {},
        root: &harness.root,
        character: &authority,
        holder: &harness.node,
        evidence: &evidence,
        world_id: world,
        channel_id: channel,
        // Gameplay-only fixture: it never admits through this owner, so the ADMIT0 binding
        // declares nothing and only has to be a valid Channel boot binding.
        required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &[],
            crate::foundation::RuntimeScopeRefV1::channel(world, channel),
            1,
            "policy-1",
            crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
        )
        .expect("an empty declared set binds"),
        runtime: &runtime,
        movement_cells: room.movement_cells(),
        door: &door,
        chest: room.door(),
        spells: &spells,
        active_generation: Some(active),
        premium_coordinator: None,
        rooms: crate::gameplay_transport::SeamRooms::new(None, Some(&room)),
        achievements: &achievements,
        imported_charms: &charms,
        spell_states: Mutex::default(),
        world_gate: std::sync::Mutex::default(),
        attack: Mutex::default(),
        // In the past, so the owner clock already stands well after the swing clock's origin.
        clock_origin: std::time::Instant::now()
            .checked_sub(Duration::from_secs(60))
            .expect("a monotonic clock older than a minute"),
        lost: std::sync::Mutex::default(),
        revision_sequencer:
            crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new(),
        quest_catalogue: None,
        quest_log_content: None,
        quest_sessions: std::sync::Mutex::default(),
        chat: crate::gameplay_transport::chat_intent::ChatRuntime::default(),
        npc_conversations: Default::default(),
        wheel_ruleset: None,
        wheel_sessions: std::sync::Mutex::default(),
        progression_sessions: std::sync::Mutex::default(),
        last_positions: crate::gameplay_transport::last_position::LastPositions::default(),
        input_gate: Default::default(),
        premium: crate::premium::refresh::PremiumRefresher::new(
            Default::default(),
            harness.root.clone(),
            None,
        ),
        premium_sessions: std::sync::Mutex::default(),
        fence_holders: Default::default(),
        spell_lane: crate::durability::spell_owner_commit::SpellLane::new(world, channel),
        loop_ground: Default::default(),
    };
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            owner
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    actor,
                    session,
                    crate::gameplay_transport::actor_spell::tests::FACTS,
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
    }
    use oteryn_protocol_oteryn::world_object::{
        UseIntent, UseTarget, encode_item_use_intent, encode_use_item_intent,
    };
    let food_index = index
        .definition_ref(
            "Item",
            food_key,
            index.revision(food_key).expect("food revision"),
        )
        .expect("food index");
    let selected = super::super::capabilities::SelectedCapabilities::select(
        &[
            super::super::capabilities::OfferedCapability {
                id: 4,
                requires: &[],
            },
            super::super::capabilities::OfferedCapability {
                id: 14,
                requires: &[4],
            },
            super::super::capabilities::OfferedCapability {
                id: 15,
                requires: &[4],
            },
        ],
        if matches!(case, Some(Case::WithoutContainerCapability)) {
            &[4, 15]
        } else {
            &[4, 14, 15]
        },
    )
    .expect("selected capabilities");
    let admitted = AdmittedSession {
        game_session_id: session,
        world_id: world,
        channel_id: channel,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: selected,
            ..SessionContinuity::FRESH
        },
        item_fence: Some(fence),
    };
    if let Some(case) = case {
        item_use_view_refresh_tests::run(&owner, &harness, admitted, case).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        return harness.cleanup().await;
    }
    // Fresh domain 9 allocates main=1, outer=2. Opening outer allocates inner=3;
    // opening inner (display order, newest first) allocates potion=4 and food=5.
    let payloads = [
        encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(food_index),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?,
        encode_use_item_intent(std::num::NonZeroU64::new(2).expect("outer handle")),
        encode_use_item_intent(std::num::NonZeroU64::new(3).expect("inner handle")),
        encode_use_item_intent(std::num::NonZeroU64::new(4).expect("potion handle")),
    ];
    let output = wire_dispatch::exchange(&owner, admitted, &payloads).await?;
    let mut cursor = 0;
    let mut results = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let frame =
            crate::foundation::decode_wire_envelope(&output[cursor + 4..cursor + 4 + length])?;
        if frame.message_type() == crate::foundation::MessageType::CommandResult {
            let result = oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
            results.push((result.command_id, result.payload.to_vec()));
        }
        cursor += 4 + length;
    }
    assert_eq!(
        results,
        [
            (1, UseDisposition::Committed),
            (2, UseDisposition::Committed),
            (3, UseDisposition::Committed),
            (4, UseDisposition::Committed)
        ]
        .map(|(command, disposition)| (
            command,
            oteryn_protocol_oteryn::world_object::encode_use_result(disposition).expect("result")
        ))
    );
    assert_eq!(harness.count("game_item_use_records").await?, 2);
    for seed in [100, 110] {
        let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id = encode($1,'hex')::uuid")
            .bind(id(seed).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(quantity, if seed == 100 { 2 } else { 1 });
    }
    // A replay through the real owner shares the settled gate and never burns again.
    let outcome = owner
        .use_item(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: 4,
                item_fence: Some(fence),
            },
            ItemKey::Instance(id(110)),
        )
        .await;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    assert_eq!(harness.count("game_item_use_records").await?, 2);
    // Equipment-first refuses without falling through to the closed bag. A reserved
    // equipment match is skipped, then a reserved nested source is skipped as well.
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
        .bind(id(120).as_slice()).bind(id(42).as_slice()).bind(food_key).bind(index.revision(food_key).expect("food revision")).bind(id(121).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(120).as_slice()).bind(id(42).as_slice()).bind(id(122).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let command = |command_id| UseCommand {
        game_session_id: session,
        command_id,
        item_fence: Some(fence),
    };
    assert_eq!(
        owner
            .use_item_by_definition(actor, command(5), food_index)
            .await
            .disposition,
        UseDisposition::NothingToUse
    );
    assert_eq!(harness.count("game_item_use_records").await?, 2);
    {
        let runtime = owner.runtime.lock().await;
        owner
            .spell_states
            .lock()
            .await
            .item_uses
            .gate(&runtime, actor, session)
            .expect("gate")
            .unspendable
            .push(id(120));
    }
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(Duration::from_secs(120))
        .expect("clock");
    assert_eq!(
        owner
            .use_item_by_definition(actor, command(6), food_index)
            .await
            .disposition,
        UseDisposition::Committed
    );
    assert_eq!(harness.count("game_item_use_records").await?, 3);
    {
        let runtime = owner.runtime.lock().await;
        owner
            .spell_states
            .lock()
            .await
            .item_uses
            .gate(&runtime, actor, session)
            .expect("gate")
            .unspendable
            .push(id(100));
    }
    assert_eq!(
        owner
            .use_item_by_definition(actor, command(7), food_index)
            .await
            .disposition,
        UseDisposition::NothingToUse
    );
    assert_eq!(harness.count("game_item_use_records").await?, 3);
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await
}
