use crate::bestiary_postgres_harness::{Harness, TestResult, debug, runtime, scope};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
};
use crate::durability::world_reset::BundleHouseCatalogue;
use crate::map::boot::{self, BootPins, CheckedBundle};
use crate::node::config::{NodeConfig, WorldBundleConfig};
use sqlx::types::Json;

#[test]
fn p04_full_bundle_monday_reset_verifies_house_catalogue_and_advances_week() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            runtime()
                .map_err(debug)?
                .block_on(async {
                    let h = Harness::create_with_admission_seed(
                        admin,
                        "p04_bundle_monday_reset",
                        false,
                        false,
                    )
                    .await?;
                    let result = scenario(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "P04 Monday reset thread failed")?
        .map_err(Into::into)
}

fn full_bundle(pins: &WorldBundleConfig) -> TestResult<CheckedBundle> {
    use oteryn_world_bundle_compiler::{compile, project, source};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let world = source::project(&root)?;
    let (registry, palette) = source::registry(&root, &world)?;
    let identity = serde_json::from_slice(include_bytes!(
        "../../../../../content/world/pins/oteryn.identity.json"
    ))?;
    let input = compile::Input {
        regions: &world.regions,
        palette: &palette,
        identity,
        world: source::world_extent(&root)?,
        build_class: oteryn_world_bundle::bundle::BuildClass::NonProduction,
        draft_areas: project::draft_areas(&world.index)?,
        families: &world.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    assert_eq!(
        compiled.digest,
        pins.digest_bytes().ok_or("checked-in digest")?,
    );
    Ok(boot::check(
        compiled.bytes,
        BootPins {
            bundle: crate::map::BundlePins {
                digest: compiled.digest,
                project_format_version: pins.project_format_version.clone(),
                world_schema_version: pins.world_schema_version.clone(),
                content_revision: pins.content_revision.clone(),
                production: pins.production,
            },
            map_revision: format!("sha256:{}", pins.digest),
            start: crate::map::overlay::TilePos {
                x: pins.start_x,
                y: pins.start_y,
                floor: -pins.start_floor,
            },
        },
    )?)
}

async fn scenario(h: &Harness) -> TestResult {
    let scope = scope()?;
    let world = scope.world_id();
    let channel = scope.channel_id().ok_or("Channel scope")?;
    let pin: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../content/world/pins/oteryn.json"
    ))?;
    let start = &pin["entry_start"];
    let text = |name: &str| pin[name].as_str().map(str::to_owned).ok_or("World pin");
    let pins = WorldBundleConfig {
        path: "/unused/test-world.otwb".into(),
        digest: text("digest")?,
        project_format_version: text("project_format_version")?,
        world_schema_version: text("world_schema_version")?,
        content_revision: text("content_revision")?,
        production: false,
        start_x: u16::try_from(start["x"].as_u64().ok_or("entry x")?)?,
        start_y: u16::try_from(start["y"].as_u64().ok_or("entry y")?)?,
        start_floor: i8::try_from(start["floor"].as_u64().ok_or("entry floor")?)?,
    };
    let checked = full_bundle(&pins)?;
    let (bytes, digest) = checked.encoded_bundle();
    let catalogue = BundleHouseCatalogue::decode(bytes, digest)?;
    let tiles = catalogue.tiles();
    assert_eq!(tiles.len(), 109_744);
    let houses: std::collections::BTreeSet<_> = tiles.iter().map(|tile| tile.3).collect();
    assert_eq!(houses.len(), 995);
    let map = format!("sha256:{}", pins.digest);

    // Match the tuple used by node_boot/world_epoch.sql: the bundle owns
    // the map, while the independently issued native content owns its digests.
    let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
    let server = room.compiled().server_digest();
    let client = room.compiled().client_digest();
    let frame = room.frame_binding().digest();
    sqlx::raw_sql(
        "CREATE FUNCTION test_p04_clock() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             NEW.activated_at := CASE WHEN NEW.reset_epoch = 0
                 THEN TIMESTAMPTZ '2026-11-23 08:59:59+00'
                 ELSE TIMESTAMPTZ '2026-11-23 09:00:00+00' END;
             RETURN NEW;
         END $$;
         CREATE TRIGGER test_p04_clock BEFORE INSERT ON game_world_reset_epochs
             FOR EACH ROW EXECUTE FUNCTION test_p04_clock()",
    )
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .execute(&h.pool).await?;
    assert!(sqlx::query_scalar::<_, bool>("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice())
        .fetch_one(&h.pool).await?);
    assert!(sqlx::query_scalar::<_, bool>("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
        .bind(world.as_bytes().as_slice()).bind(digest.as_slice()).bind(&map)
        .bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice())
        .fetch_one(&h.pool).await?);
    let period = || {
        sqlx::query_scalar::<_, i64>("SELECT game_task_board_period(encode($1,'hex')::uuid)")
            .bind(world.as_bytes().as_slice())
            .fetch_one(&h.pool)
    };
    assert_eq!(period().await?, 20773);

    let publish = async |tiles: &[(i8, u16, u16, u32)]| {
        sqlx::query_scalar::<_, bool>(
            "SELECT game_world_reset_publish(encode($1,'hex')::uuid,0,$2,$3,$4,$5,$6,$7::jsonb,1)",
        )
        .bind(world.as_bytes().as_slice())
        .bind(digest.as_slice())
        .bind(&map)
        .bind(server.as_slice())
        .bind(client.as_slice())
        .bind(frame.as_slice())
        .bind(Json(tiles))
        .fetch_one(&h.pool)
        .await
    };
    assert!(publish(tiles).await.is_err(), "missing owner catalogue");
    sqlx::query("INSERT INTO game_world_reset_bundle_house_catalogues VALUES ($1,$2::jsonb)")
        .bind(digest.as_slice())
        .bind(Json(tiles))
        .execute(&h.pool)
        .await?;
    assert!(
        publish(&[]).await.is_err(),
        "nonempty proof cannot be replaced with []"
    );
    assert!(
        publish(&tiles[1..]).await.is_err(),
        "one omitted House tile"
    );
    assert!(publish(tiles).await?);
    assert!(!publish(tiles).await?, "identical publication replays");

    // Independently exercise the final SQL guard, even when an owner has
    // changed the frozen job after publication. Roll back the injected fault.
    let mut fault = h.pool.begin().await?;
    sqlx::query("UPDATE game_world_reset_jobs SET target_house_tiles=$2::jsonb WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0")
        .bind(world.as_bytes().as_slice()).bind(Json(&tiles[1..]))
        .execute(&mut *fault).await?;
    let node = h.node.fact();
    let refused = sqlx::query_scalar::<_, String>(
        "SELECT game_world_reset_activate(world_id,reset_epoch,retirement_binding,target_digest,
         target_map_revision,target_server_digest,target_client_digest,target_frame_digest,
         encode($2,'hex')::uuid,$3::text::numeric)
         FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0",
    )
    .bind(world.as_bytes().as_slice())
    .bind(node.node_id().as_bytes().as_slice())
    .bind(node.registration_revision().to_string())
    .fetch_one(&mut *fault)
    .await;
    let Err(sqlx::Error::Database(error)) = refused else {
        return Err("activation accepted drifted House evidence".into());
    };
    assert_eq!(error.code().as_deref(), Some("23514"));
    assert_eq!(
        error.message(),
        "World reset House qualification unavailable"
    );
    fault.rollback().await?;

    assert_eq!(
        period().await?,
        20773,
        "RETIRING leaves the previous week active"
    );

    h.root.await_ready().await?;
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("reset predecessor")?;
    assert!(matches!(
        h.writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([9; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope,
                    predecessor,
                    target: h.node.fact()
                },
            })
            .await
            .map_err(debug)?,
        AssignmentOutcome::Committed(_)
    ));

    h.root.await_ready().await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut config = NodeConfig::parse(crate::node::config::tests::NODE.as_bytes())?;
    config.world_bundle = Some(pins);
    h.root.await_ready().await?;
    assert_eq!(
        crate::node::serve::recover_retiring(
            &h.root,
            &config,
            world,
            &authority,
            &h.node,
            Some(&checked),
        )
        .await
        .map_err(debug)?,
        1,
    );
    assert_eq!(
        period().await?,
        20780,
        "Monday Server Save advances Weekly Tasks"
    );
    let activated: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs j
         JOIN game_world_reset_active a ON a.world_id = j.world_id
         JOIN game_content_activations c ON c.world_id = j.world_id
         WHERE j.world_id = encode($1,'hex')::uuid AND j.reset_epoch = 0
           AND j.phase = 'ACTIVATED' AND a.reset_epoch = 1
           AND j.target_house_tiles = $2::jsonb
           AND c.channel_id = encode($3,'hex')::uuid AND c.activation_sequence = 2)",
    )
    .bind(world.as_bytes().as_slice())
    .bind(Json(tiles))
    .bind(channel.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert!(activated);
    Ok(())
}
