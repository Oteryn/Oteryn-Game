use super::*;
use std::collections::BTreeMap;

pub(super) fn object(
    map: &crate::map::WorldBase,
    x: u16,
    y: u16,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let key =
        crate::map::view::placement_key(crate::map::overlay::TilePos { x, y, floor: -8 }, ordinal)
            .ok_or("map placement")?;
    let mut placement = map.digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
}

pub(super) async fn issue_content(h: &Harness, pin: &ChannelContentPin) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
        .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'soulpit-fixture',$2,$3,$4,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-10')")
        .bind(id(42).as_slice()).bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice())
        .execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(id(42).as_slice())
        .execute(&h.pool)
        .await?;
    Ok(())
}

pub(super) fn bundle(
    ground_ref: u32,
    obelisk_ref: u32,
) -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self as w, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
        TerrainKind,
    };
    let mut sectors = BTreeMap::<(u16, u16), Vec<Tile>>::new();
    for y in 31132..=31162_u16 {
        for x in 32362..=32390_u16 {
            let mut items = vec![Item {
                palette: 0,
                depth: 0,
                attrs: Attrs::default(),
            }];
            if (x, y) == (32375, 31157) {
                items.push(Item {
                    palette: 1,
                    depth: 0,
                    attrs: Attrs::default(),
                });
            }
            sectors.entry((y / 32, x / 32)).or_default().push(Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items,
            });
        }
    }
    let sectors = sectors
        .into_iter()
        .map(|((sy, sx), tiles)| Sector {
            floor: -8,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    let manifest = Manifest {
        subareas: Default::default(),
        format: w::FORMAT.into(),
        min_reader_version: w::VERSION,
        projection_class: "server".into(),
        compiler_version: "engineering".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "sv8-soulpit".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 32352,
            min_y: 31104,
            max_x: 32416,
            max_y: 31168,
            floors: vec![-8],
        },
        palette: vec![
            PaletteEntry {
                key: "oteryn:item.tibia.i103".into(),
                family: Family::Item,
                id: ground_ref,
                terrain: Some(Terrain {
                    kind: TerrainKind::Ground,
                    walkable: Some(true),
                    ground_speed: Some(110),
                }),
            },
            PaletteEntry {
                key: "oteryn:item.tibia.i47367".into(),
                family: Family::Item,
                id: obelisk_ref,
                terrain: None,
            },
        ],
        draft_areas: Vec::new(),
        skipped_provisional_keys: Vec::new(),
        dropped_teleports: Vec::new(),
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let bytes = w::write(&manifest, &sectors, &Default::default()).map_err(debug)?;
    let digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    Ok((
        bytes,
        crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "sv8-soulpit".into(),
            production: false,
        },
    ))
}

pub(super) async fn probe_entry_rollback(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    intent: &UseIntent,
) -> TestResult {
    sqlx::query("CREATE FUNCTION sv8_soulpit_entry_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'Soulpit entry rollback probe'; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER sv8_soulpit_entry_failure AFTER INSERT ON game_soulpit_encounters FOR EACH ROW EXECUTE FUNCTION sv8_soulpit_entry_failure()")
        .execute(&h.pool).await?;
    assert!(
        owner
            .use_soulpit_obelisk(actor, command, intent, None, None)
            .await
            .is_err()
    );
    let fence = command.item_fence.ok_or("source prune fence")?;
    owner
        .prune_refused_activity_attempts(
            crate::gameplay_transport::activity_runtime::ActivityRefusalPrune::BeforeCommand {
                session: command.game_session_id,
                connection_generation: fence.connection_generation.get(),
                command: command.command_id + 1,
            },
        )
        .await
        .map_err(|_| "prune refused after unknown SQL outcome")?;
    assert_eq!(
        owner
            .activity_runtimes
            .soulpit_entries
            .lock()
            .map_err(debug)?
            .len(),
        1,
        "uncertain native entry survives bounded refusal pruning for exact retry"
    );
    let core: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(71).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(core, 2, "entry rollback retains both physical cores");
    let groups: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        groups, 0,
        "entry rollback publishes no durable shared handoff"
    );
    sqlx::query("DROP TRIGGER sv8_soulpit_entry_failure ON game_soulpit_encounters")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP FUNCTION sv8_soulpit_entry_failure()")
        .execute(&h.pool)
        .await?;
    Ok(())
}

pub(super) async fn probe_return_uncertainty(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    destination: AdmittedSession,
) -> TestResult {
    let actor = destination.runtime_actor.ok_or("return actor")?;
    let map = owner.bundle_map().ok_or("real Instance map")?;
    {
        let _permit = owner
            .spell_lane_permit()
            .await
            .ok_or("exit movement lane")?;
        let mut runtime = owner.runtime.lock().await;
        let before = runtime.read_actor_position(actor).map_err(debug)?;
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells,
            actor,
            destination.game_session_id,
            before,
            MovementLocalPosition {
                x: 32375,
                y: 31152,
                floor: 8,
            },
            false,
            &closed,
        )
        .map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
    }
    let exit = object(map.overlay.base(), 32375, 31153, 0)?;
    let command = UseCommand {
        game_session_id: destination.game_session_id,
        command_id: 2,
        item_fence: destination.item_fence,
    };
    sqlx::query("CREATE SEQUENCE sv8_soulpit_return_failure_seen")
        .execute(&h.pool)
        .await?;
    sqlx::query("CREATE FUNCTION sv8_soulpit_return_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('sv8_soulpit_return_failure_seen'); RAISE EXCEPTION 'Soulpit return rollback probe'; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER sv8_soulpit_return_failure AFTER INSERT ON game_activity_runtime_returns FOR EACH ROW EXECUTE FUNCTION sv8_soulpit_return_failure()")
        .execute(&h.pool).await?;
    for attempt in 1..=2_i64 {
        super::super::maintain_ready(owner.root).await?;
        let failed = owner
            .use_soulpit_exit(actor, command, &exit)
            .await
            .ok_or("retained exit callback")?;
        assert_eq!(failed.disposition, UseDisposition::Rejected);
        assert_eq!(
            owner
                .activity_runtimes
                .soulpit_returns
                .lock()
                .map_err(debug)?
                .len(),
            1
        );
        assert_eq!(
            owner
                .activity_runtimes
                .current_connection(destination)
                .game_session_id,
            destination.game_session_id
        );
        let returns: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            returns, 0,
            "uncertain return publishes no SQL membership change"
        );
        let core: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(core, 1, "return replay never consumes another core");
        let seen: (i64, bool) =
            sqlx::query_as("SELECT last_value,is_called FROM sv8_soulpit_return_failure_seen")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            seen,
            (attempt, true),
            "each exact retry reaches the uncertain SQL boundary"
        );
        if attempt == 1 {
            let mut changed_target = exit.clone();
            changed_target.expected_revision += 1;
            let mut stale_fence = command.item_fence.ok_or("retained fence")?;
            stale_fence.scope_ownership_generation =
                crate::foundation::ScopeOwnershipGeneration::new(
                    stale_fence.scope_ownership_generation.get() + 1,
                )
                .map_err(debug)?;
            let stale_command = UseCommand {
                item_fence: Some(stale_fence),
                ..command
            };
            for (changed_command, target) in
                [(command, changed_target), (stale_command, exit.clone())]
            {
                let refused = owner
                    .use_soulpit_exit(actor, changed_command, &target)
                    .await
                    .ok_or("altered retained exit rejected")?;
                assert_eq!(refused.disposition, UseDisposition::Rejected);
                let unchanged: (i64, bool) = sqlx::query_as(
                    "SELECT last_value,is_called FROM sv8_soulpit_return_failure_seen",
                )
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(unchanged, (1, true), "altered identity never resumes SQL");
                assert_eq!(
                    owner
                        .activity_runtimes
                        .soulpit_returns
                        .lock()
                        .map_err(debug)?
                        .len(),
                    1
                );
            }
        }
    }
    let injected: (i64, bool) =
        sqlx::query_as("SELECT last_value,is_called FROM sv8_soulpit_return_failure_seen")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        injected,
        (2, true),
        "both authenticated exact retries reach SQL"
    );
    sqlx::query("DROP TRIGGER sv8_soulpit_return_failure ON game_activity_runtime_returns")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP FUNCTION sv8_soulpit_return_failure()")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP SEQUENCE sv8_soulpit_return_failure_seen")
        .execute(&h.pool)
        .await?;
    // The production service maintains semantic holders independently. Recreate
    // that ready holder after the deliberately aborted transaction before retry.
    super::super::maintain_ready(owner.root).await?;
    let committed = owner
        .use_soulpit_exit(actor, command, &exit)
        .await
        .ok_or("same command return retry")?;
    assert_eq!(committed.disposition, UseDisposition::Committed);
    assert!(
        owner
            .activity_runtimes
            .soulpit_returns
            .lock()
            .map_err(debug)?
            .is_empty()
    );
    let returns: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(returns, 1);
    let core: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(71).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(core, 1);
    Ok(())
}

/// Initialize with the active content's real policy, then reach the source
/// qualification through the fenced XP writer rather than historical seed rows.
pub(super) async fn qualify_entry_level(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
        convert_vocation,
    };
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let formula = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.training_formula())
        .ok_or("native class choice formula")?;
    let before = DurableBuildState::default();
    let after = convert_vocation(formula, &before, "monk").map_err(debug)?;
    let chosen = h
        .root
        .commit_character_build(
            owner.character,
            owner.holder,
            super::super::death_ml_loss_composed::current_fence(owner, admitted.game_session_id)
                .await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(119)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before,
                after,
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(chosen, BuildCommitOutcome::Committed(_)));
    let progression = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("real native progression admission")?;
    let required = progression
        .policy
        .thresholds
        .iter()
        .find(|threshold| threshold.level == crate::domain::soulpit::REQUIRED_LEVEL)
        .ok_or("Soulpit required level in active experience table")?
        .minimum_experience;
    let fence =
        super::super::death_ml_loss_composed::current_fence(owner, admitted.game_session_id)
            .await?;
    let award = h
        .root
        .commit_character_experience(
            owner.character,
            owner.holder,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(118)).map_err(debug)?,
                amount: required,
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    assert!(matches!(award, ExperienceCommitOutcome::Committed(_)));
    let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        u32::try_from(level)?,
        crate::domain::soulpit::REQUIRED_LEVEL
    );
    let crate::gameplay_transport::CastFactsLoad::Ready { facts, .. } =
        crate::gameplay_transport::load_character_cast_facts(
            owner.root,
            owner.character,
            fence.character_id,
        )
        .await
    else {
        return Err("official classed source facts".into());
    };
    let build = owner
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let actor = admitted.runtime_actor.ok_or("source build actor")?;
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    assert!(
        states
            .initialize(
                &runtime,
                actor,
                admitted.game_session_id,
                facts,
                (0, 0),
                owner.owner_now()
            )
            .is_some()
    );
    assert!(states.load_owned_build(&runtime, actor, admitted.game_session_id, &build));
    Ok(())
}

pub(super) async fn prove_equipment_entry_fences(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    origin: AdmittedSession,
    destination: AdmittedSession,
) -> TestResult {
    let mut fence = destination.item_fence.ok_or("Instance equipment fence")?;
    let digest = owner
        .runtime
        .lock()
        .await
        .content_pin()
        .server_artifact_digest();
    assert!(
        owner
            .root
            .read_activity_equipment(owner.character, owner.holder, &fence, [17; 32])
            .await
            .is_err(),
        "a different content pin cannot read the Instance custody graph"
    );
    let gameplay = owner
        .current_quest_fence(destination.game_session_id)
        .await
        .map_err(|_| "Instance lifecycle gameplay fence")?
        .ok_or("current Instance lifecycle fence")?;
    let lifecycle = owner
        .root
        .read_lifecycle_durable_facts(owner.character, owner.holder, &gameplay, digest)
        .await
        .map_err(debug)?;
    assert_eq!(lifecycle.fence, fence);
    assert!(
        lifecycle
            .equipment
            .items
            .iter()
            .any(|item| item.slot == 6
                && item.definition.production_key == "oteryn:item.tibia.i3264"),
        "actual Instance lifecycle reader sees native weapon custody"
    );
    assert_eq!(lifecycle.level, crate::domain::soulpit::REQUIRED_LEVEL);
    assert!(
        owner
            .root
            .read_lifecycle_durable_facts(owner.character, owner.holder, &gameplay, [17; 32])
            .await
            .is_err(),
        "Instance lifecycle facts reject a different content generation"
    );
    fence.game_session_id = origin.game_session_id;
    assert!(
        owner
            .root
            .read_activity_equipment(owner.character, owner.holder, &fence, digest)
            .await
            .is_err(),
        "the retired source session cannot read successor Instance equipment"
    );
    super::super::maintain_ready(owner.root).await?;
    Ok(())
}
