//! Shared owner assembly for MINING-1 real-wire PostgreSQL cases.
use super::*;

pub(super) async fn scenario(admin: String) -> TestResult {
    let harness = Harness::create(admin, "mining_wire", true).await?;
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
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(native.source_digest().as_slice())
        .bind(active.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
    let room = pin.qualified_room().clone();
    let food_key = "oteryn:item.tibia.i3456";
    let potion_key = "oteryn:item.tibia.i10310";
    let bag_key = "oteryn:item.tibia.i2854";
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key, quantity) in [
        (70, bag_key, 1_i64),
        (80, bag_key, 1),
        (90, bag_key, 1),
        (100, food_key, 1),
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
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec())],
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec())],
        rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("../../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der").to_vec())?,
    ).map_err(debug)?;
    let evidence = crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
    let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
    let achievements =
        crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
    let charms =
        crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
    let runtime = Mutex::new(runtime);
    let door = Mutex::new(door);
    let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
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
        room: &room,
        door: &door,
        spells: &spells,
        achievements: &achievements,
        imported_charms: &charms,
    }
    .build();
    owner.active_generation = Some(active);
    owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
    // In the past, so the owner clock already stands well after the swing clock's origin.
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(Duration::from_secs(60))
        .expect("a monotonic clock older than a minute");
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            owner
                .states
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
    super::restart::runtime_role(&harness.root).await?;
    run(&owner, &harness, actor, session, index).await?;
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await
}
