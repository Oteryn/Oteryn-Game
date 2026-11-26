//! Direct Post reconciles a committed receipt after the same actor steps out of reach.

use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use crate::gameplay_transport::{
    capabilities::SelectedCapabilities, connection::FreshAdmissionAuthority, item_view::ItemKey,
};
use crate::map::{
    boot::{BootPins, boot, tests as bundle_fixture},
    facts::ItemDefinition,
    floor::{BundleDoors, prepare_bundle_step},
    overlay::TilePos,
};
use oteryn_protocol_oteryn::mail::{CAPABILITY_MAIL_V1, MailIntent, MailOutcome, encode_intent};

const MAILBOX: &str = "oteryn:item.tibia.i3501";
const LETTER: &str = "oteryn:item.tibia.i3505";
const COIN: &str = "oteryn:item.tibia.i3031";
const START: TilePos = TilePos {
    x: 7,
    y: 0,
    floor: -7,
};

#[test]
fn mail_post_retry_after_step_away_returns_receipt_without_second_delivery() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(scenario(admin))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "mail Post replay scenario panicked")??;
    Ok(())
}

async fn scenario(admin: String) -> TestResult {
    let harness =
        Harness::create_with_admission_seed(admin, "mail_post_replay", true, false).await?;
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let initial = Dependencies::new()?;
    let (controller, activation) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            initial.admitted.world_id,
            "content-1",
        )?;
    let generation = controller.active().ok_or("active generation")?;
    let index = generation
        .native_gameplay()
        .and_then(|native| native.item_index())
        .ok_or("item index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let (bytes, mut pins) = bundle_fixture::bundle_with_item_ids(
        (
            MAILBOX,
            reference(MAILBOX).ok_or("mailbox reference")?.get() - 1,
        ),
        (COIN, reference(COIN).ok_or("coin reference")?.get() - 1),
    );
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let sector = bundle.sectors.first_mut().ok_or("mail sector")?;
    let mut step_tile = sector
        .tiles
        .iter()
        .find(|tile| tile.x == START.x)
        .ok_or("start tile")?
        .clone();
    step_tile.x += 1;
    step_tile.items.truncate(1);
    sector.tiles.push(step_tile);
    let bytes =
        oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
            .map_err(debug)?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let map = boot(
        &bytes,
        &BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: START,
        },
        initial.admitted.world_id,
        initial.admitted.channel_id,
        |key| {
            Some(ItemDefinition {
                reference: reference(key)?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: key == COIN,
                door: false,
            })
        },
    )
    .map_err(debug)?;
    let room = activation.qualified_room().clone();
    let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
    let identity = generation.identity();
    let start = MovementLocalPosition {
        x: i32::from(START.x),
        y: i32::from(START.y),
        floor: 7,
    };
    let pin = ChannelContentPin::from_activation(
        initial.admitted.world_id,
        activation.activation_sequence(),
        identity.server_artifact_digest(),
        identity.client_artifact_digest(),
        map.frame_binding_digest(initial.admitted.world_id),
        map.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let deps = Dependencies::with_content(pin, room, initial.spells)?;
    let actor = deps.actor;
    deps.runtime
        .lock()
        .await
        .initialize_pinned_test_position(actor, start)
        .map_err(debug)?;
    let mut owner = deps.owner(&harness, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    let current = FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(admitted.game_session_id)
        .await?
        .0;
    admitted.item_fence =
        Some(crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?);
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[4, 6, u32::from(CAPABILITY_MAIL_V1)],
    )
    .ok_or("mail capabilities")?;
    super::item_look_fixtures::seed_main_backpack(&harness, &[(LETTER, 1), (LETTER, 1)]).await?;
    address_letters(&harness).await?;

    let mailbox_position = TilePos {
        x: START.x - 1,
        ..START
    };
    let map_item = |position| -> TestResult<ItemKey> {
        Ok(ItemKey::MapBase {
            bundle_digest: map.base().digest(),
            placement_key: crate::map::view::placement_key(position, 1).ok_or("map placement")?,
            reset_epoch: 0,
        })
    };
    let mailbox = map_item(mailbox_position)?;
    let first = encode_intent(&MailIntent::Post {
        item_handle: 1,
        mailbox_handle: 2,
    })
    .map_err(debug)?;
    let second = encode_intent(&MailIntent::Post {
        item_handle: 3,
        mailbox_handle: 2,
    })
    .map_err(debug)?;
    let post = async |command_id, payload: &[u8], item, target| {
        owner
            .mail_command(
                actor,
                admitted,
                command_id,
                payload,
                Some(ItemKey::Instance(item)),
                Some(target),
            )
            .await
    };
    assert_eq!(
        post(500, &first, id(71), mailbox).await,
        Some(MailOutcome::Posted)
    );

    {
        let mut runtime = deps.runtime.lock().await;
        let before = runtime.read_actor_position(actor).map_err(debug)?;
        let doors = BundleDoors(map.facts().doors());
        let proof = prepare_bundle_step(
            &runtime,
            owner.movement_cells(),
            actor,
            admitted.game_session_id,
            before,
            crate::movement::CardinalStep::East,
            false,
            &doors,
        )
        .map_err(debug)?;
        let landed = runtime.commit_source_step(proof).map_err(debug)?;
        assert_eq!(landed.position().x, i32::from(mailbox_position.x) + 2);
    }

    sqlx::query("ALTER TABLE game_mail_operations RENAME TO fixture_mail_operations_unavailable")
        .execute(&harness.pool)
        .await?;
    let unavailable = post(500, &first, id(71), mailbox).await;
    sqlx::query("ALTER TABLE fixture_mail_operations_unavailable RENAME TO game_mail_operations")
        .execute(&harness.pool)
        .await?;
    assert_eq!(unavailable, None, "receipt read failure leaves the outcome unknown");
    assert_eq!(
        post(500, &first, id(71), mailbox).await,
        Some(MailOutcome::Posted),
        "exact retry after stepping away"
    );
    assert_eq!(
        post(500, &second, id(72), mailbox).await,
        Some(MailOutcome::Rejected),
        "same command with a different letter"
    );
    let other_target = encode_intent(&MailIntent::Post {
        item_handle: 1,
        mailbox_handle: 4,
    })
    .map_err(debug)?;
    assert_eq!(
        post(500, &other_target, id(71), map_item(START)?).await,
        Some(MailOutcome::Rejected),
        "same command with a different placement"
    );
    assert_eq!(
        post(501, &second, id(72), mailbox).await,
        Some(MailOutcome::Rejected),
        "fresh post out of reach"
    );
    let posted: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_mail_operations), \
                (SELECT count(*) FROM game_character_inbox_deliveries), \
                (SELECT count(*) FROM game_item_character_inbox_locations \
                 WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid)",
    ).bind(id(71).as_slice()).bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(posted, (1, 1, 1));
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await
}

async fn address_letters(harness: &Harness) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for letter in [id(71), id(72)] {
        sqlx::query("INSERT INTO game_item_text_revisions VALUES (encode($1,'hex')::uuid,1)")
            .bind(letter.as_slice())
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO game_item_texts(item_instance_id,text,written_at,text_revision) \
             SELECT encode($1,'hex')::uuid,name,now(),1 FROM game_character_roots \
             WHERE character_id=encode($2,'hex')::uuid",
        )
        .bind(letter.as_slice())
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
