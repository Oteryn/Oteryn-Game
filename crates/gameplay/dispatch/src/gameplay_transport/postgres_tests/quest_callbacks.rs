//! Generic source chest -> literal USE -> existing atomic claim/MINT -> reconnect/replay.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{
    ChannelRuntimeV1, MessageType, MovementLocalPosition, decode_wire_envelope,
};
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

// Canary's Steel Helmet Quest coin chest, not an injected entry-room claim.
const CLAIM: &str = "oteryn:reward-claim.kv.steelhelmet1";
const CHEST: (u16, u16, i8) = (32460, 31951, -5);

fn source_world(
    unique: u16,
    appearance: u16,
    offset: u16,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.world = bundle::Extent {
        min_x: 32448,
        min_y: 31936,
        max_x: 32480,
        max_y: 31968,
        floors: vec![-5],
    };
    // A terrain carrier is legal for a placed chest (the binder also validates
    // Item carriers). Its appearance and unique id must both match the shard.
    data.manifest.palette.truncate(1);
    data.manifest.palette.push(bundle::PaletteEntry {
        key: format!("oteryn:terrain.tibia.i{appearance}"),
        family: bundle::Family::Terrain,
        id: 1,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
        }),
    });
    let ground = || sector::Item {
        palette: 0,
        depth: 0,
        attrs: sector::Attrs::default(),
    };
    let tiles = (CHEST.0 - 2..=CHEST.0 + offset)
        .map(|x| sector::Tile {
            x,
            y: CHEST.1,
            flags: 0,
            house: 0,
            zones: Vec::new(),
            items: if x == CHEST.0 + offset {
                vec![
                    ground(),
                    sector::Item {
                        palette: 1,
                        depth: 0,
                        attrs: sector::Attrs {
                            unique: Some(unique),
                            ..Default::default()
                        },
                    },
                ]
            } else {
                vec![ground()]
            },
        })
        .collect();
    let bytes = oteryn_world_bundle::writer::write(
        &data.manifest,
        &[bundle::Sector {
            floor: CHEST.2,
            sx: CHEST.0 / 32,
            sy: CHEST.1 / 32,
            tiles,
        }],
        &Default::default(),
    )?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: CHEST.0 - 1,
                y: CHEST.1,
                floor: CHEST.2,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

#[test]
fn quest_callbacks_binding_and_coverage_require_exact_source_witnesses() -> TestResult {
    use crate::content::world_reward_claims::bind_embedded_reward_claims;
    use std::collections::BTreeSet;
    let manifest: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/repository/QUEST_CALLBACK_COVERAGE.json"
    )))?;
    let supported: BTreeSet<_> = manifest["quests"]
        .as_array()
        .ok_or("quests")?
        .iter()
        .flat_map(|quest| quest["chests"].as_array().into_iter().flatten())
        .filter(|chest| chest["status"] != "not_playable")
        .map(|chest| chest["claim"].as_str().ok_or("claim"))
        .collect::<Result<_, _>>()?;
    for (unique, appearance, offset, bound) in [
        (6176, 2434, 0, true),
        (6177, 2434, 0, false),
        (6176, 2435, 0, false),
        (6176, 2434, 1, false),
    ] {
        let world = source_world(unique, appearance, offset)?;
        let report = bind_embedded_reward_claims(world.base().as_ref(), &|_| true)?;
        assert_eq!(
            report
                .served
                .iter()
                .any(|claim| claim.key.as_str() == CLAIM),
            bound
        );
        // The manifest's supported set must match the actual existing binder,
        // not a second interpretation of the authoring readiness fields.
        let candidates: BTreeSet<_> = report
            .served
            .iter()
            .map(|claim| claim.key.as_str())
            .chain(
                report
                    .unbound
                    .iter()
                    .map(|placement| placement.claim.as_str()),
            )
            .collect();
        assert_eq!(supported, candidates);
    }
    Ok(())
}

async fn wire_uses(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    commands: &[u64],
    target: &WorldObjectTarget,
) -> TestResult<Vec<UseDisposition>> {
    use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(target).map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let peer = async {
        let mut results = Vec::new();
        for &command_id in commands {
            let frame = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id,
                    command_type: 2,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            client
                .write_all(&u32::try_from(frame.len())?.to_be_bytes())
                .await?;
            client.write_all(&frame).await?;
            loop {
                let length = client.read_u32().await?;
                let mut bytes = vec![0; usize::try_from(length)?];
                client.read_exact(&mut bytes).await?;
                let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let value = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        .map_err(debug)?;
                    assert_eq!(value.command_id, command_id);
                    results.push(
                        oteryn_protocol_oteryn::world_object::decode_use_result(value.payload)
                            .map_err(debug)?,
                    );
                    break;
                }
            }
            // Respect the real 200ms world_action cooldown before another USE.
            tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(results)
    };
    let (_, results) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(
            serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS),
            peer,
        ),
    )
    .await?;
    results
}

#[test]
fn quest_callbacks_source_chest_use_once_replay_and_second_use_refused() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "quest callbacks e2e panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "quest_callbacks", false, false).await?;
        prepare_current_session(&harness).await?;
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
        let world = source_world(6176, 2434, 0)?;
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
                i32::from(CHEST.0) - 1,
                i32::from(CHEST.1),
                -i16::from(CHEST.2),
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
                    x: i32::from(CHEST.0) - 1,
                    y: i32::from(CHEST.1),
                    floor: -i16::from(CHEST.2),
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
        let key = oteryn_world_bundle::bundle::placement_key(CHEST.2, CHEST.0, CHEST.1, 1)
            .ok_or("key")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        // The second USE is a new command on the live connection. Rebuild the owner
        // for the replay so no Foundation command-result cache can satisfy it.
        for (commands, expected) in [
            (
                &[1, 2][..],
                &[UseDisposition::Committed, UseDisposition::NothingToUse][..],
            ),
            (&[1][..], &[UseDisposition::Committed][..]),
        ] {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = &cells;
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            assert_eq!(
                wire_uses(&owner, admitted, commands, &target).await?,
                expected
            );
            let bag = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("bag")?;
            assert_eq!(bag.entries.len(), 1);
            assert_eq!(bag.entries[0].item.quantity, 56);
            assert_eq!(
                bag.entries[0].item.definition.production_key,
                "oteryn:item.tibia.i3031"
            );
        }
        let claims: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid AND claim_production_key=$2), (SELECT count(*) FROM game_reward_claim_mint_receipts)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(CLAIM).fetch_one(&harness.pool).await?;
        assert_eq!(claims, (1, 1));
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
