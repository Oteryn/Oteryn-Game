//! Synthetic citizenship tile and two temples through the real bundle loader.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::ability::creature_bite::CreatureBiteVitals;
use crate::foundation::{CharacterLease, MovementLocalPosition};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

fn towns() -> oteryn_world_bundle::subarea::Table {
    use oteryn_world_bundle::subarea::{Area, Mask, Run, Table, Temple};
    let hash = "a".repeat(64);
    let area = |id, town, x| Area {
        id,
        key: format!("oteryn:content.area.fixture{id}"),
        name: format!("Town {id}"),
        parent: (id != 1).then_some(1),
        source_record_sha256: hash.clone(),
        town_id: town,
        hometown: town.is_some(),
        temple: town.map(|_| Temple {
            x,
            y: 11,
            floor: -7,
        }),
    };
    Table {
        default_town: Some(8),
        client_version: "15.30".into(),
        candidate_sha256: hash.clone(),
        // Town 11 (Edron) is the P20 premium-area home town; 8 and 6 are free-account towns.
        areas: vec![
            area(1, None, 0),
            area(2, Some(8), 2),
            area(3, Some(6), 8),
            area(4, Some(11), 14),
        ],
        masks: vec![Mask {
            subarea: 2,
            asset: format!("content/assets/files/subarea-0002-{hash}.bmp.lzma"),
            compressed_sha256: hash.clone(),
            bmp_sha256: hash.clone(),
            source_record_sha256: hash,
            source_tiles: 1,
            admitted_tiles: 1,
        }],
        runs: vec![Run {
            floor: -7,
            y: 11,
            begin: 2,
            end: 3,
            subarea: 2,
        }],
    }
}

#[test]
fn citizen_tile_persists_town_death_uses_temple_and_reconnect_keeps_it() -> TestResult {
    on_large_stack(None)
}

#[test]
fn p20_ended_premium_moves_a_premium_area_citizen_to_the_thais_temple() -> TestResult {
    on_large_stack(Some(PremiumEntry::Stored))
}

#[derive(Clone, Copy)]
enum PremiumEntry {
    Stored,
    Respawn,
    Arrival,
}

#[test]
fn p20_ended_premium_consumes_pending_respawn_with_relocation() -> TestResult {
    on_large_stack(Some(PremiumEntry::Respawn))
}

#[test]
fn p20_ended_premium_consumes_pending_arrival_with_relocation() -> TestResult {
    on_large_stack(Some(PremiumEntry::Arrival))
}

fn on_large_stack(p20: Option<PremiumEntry>) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(p20).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "home town PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario(p20: Option<PremiumEntry>) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "citizen_temple", false, false).await?;
        let request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = fence(1)?.runtime_scope.world_id();
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_source(
            world_id,
            "content-1",
            None,
        )?;
        let active = controller.active().ok_or("active generation")?;
        let bundle = world(world_id)?;
        let cells = bundle
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = pin
            .qualified_room()
            .bundle_view(
                cells.clone(),
                bundle.frame_binding_digest(world_id),
                bundle.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 2,
                    y: 11,
                    floor: 7,
                },
            )
            .ok_or("bundle room")?;
        let channel_pin = crate::foundation::ChannelContentPin::from_activation(
            world_id,
            1,
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (2, 11, 7),
        );
        if p20.is_some() {
            // Real first entry independently verifies this activation before P20 placement.
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                .bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(channel_pin.server_artifact_digest().as_slice())
                .bind(channel_pin.client_artifact_digest().as_slice())
                .bind(channel_pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
        }
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            active
                .native_gameplay()
                .ok_or("native")?
                .spell_book()
                .clone(),
        )?;
        let (consumer, coordinator) = premium_reads(&h);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        assert!(owner.progression_binding(session).is_some());
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
        }
        assert_eq!(owner.home_town(session, Some((actor, 8))).await, None);
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        deps.initialize(&owner).await;
        assert_eq!(owner.home_town(session, None).await, Some(8));
        assert_eq!(
            revisions(&h).await?,
            (1, 1),
            "materializing the default is not a semantic change"
        );
        let (outcome, _) = owner
            .ordinary_paced_step(actor, session, StepDirection::East, None, None)
            .await;
        assert_eq!(outcome.moved_to.ok_or("citizen step")?.actor_position.x, 8);
        let stored: i32 = sqlx::query_scalar("SELECT town_id FROM game_character_home_towns WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(stored, 6);
        assert_eq!(
            revisions(&h).await?,
            (2, 2),
            "citizenship advances the root and progression revision together"
        );
        assert_eq!(owner.home_town(session, Some((actor, 6))).await, Some(6));
        assert_eq!(
            revisions(&h).await?,
            (2, 2),
            "the same citizenship is idempotent"
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_home_town_receipts WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(receipts, 1, "one receipt for the actual town change");
        if let Some(entry) = p20 {
            premium_ended(
                &h,
                &mut owner,
                (&consumer, &coordinator),
                actor,
                session,
                (&deps, &request, entry),
            )
            .await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        assert!(sqlx::query(
            "UPDATE game_character_home_towns SET town_id=8 WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).execute(&h.pool).await.is_err(), "town writes require their receipt");
        assert!(sqlx::query(
            "UPDATE game_character_home_town_receipts SET town_after=8 WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).execute(&h.pool).await.is_err(), "receipts are immutable");
        let temple = MovementLocalPosition {
            x: 8,
            y: 11,
            floor: 7,
        };
        assert_eq!(owner.home_temple(session).await, Some(temple));
        let channel = owner.channel_id;
        owner.channel_id = crate::foundation::ChannelId::decode(&id(44)).map_err(debug)?;
        assert_eq!(owner.home_town(session, Some((actor, 8))).await, None);
        owner.channel_id = channel;
        // A stale generation cannot overwrite citizenship.
        let mut stale = fence(2)?;
        stale.character_lease_generation = 1;
        assert!(
            h.root
                .character_home_town(&authority, &h.node, stale, 8, Some(8), None)
                .await
                .is_err()
        );
        let death = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .place_respawned_player(actor, session, MovementLocalPosition { x: 9, ..temple })
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            states
                .apply_creature_damage(
                    &mut runtime,
                    actor,
                    session,
                    100_000,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                        owner.owner_now().get(),
                    ),
                )
                .ok_or("damage")?;
            states
                .player_death(&runtime, actor, session)
                .ok_or("death")?
        };
        owner
            .respawn_after_death(actor, session, death)
            .await
            .ok_or("temple respawn")?;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            temple
        );
        // Recreate the dispatch owner, as on reconnect; citizenship is loaded from PostgreSQL.
        drop(owner);
        let mut reconnected = deps.owner(&h, &authority);
        reconnected.active_generation = Some(active);
        reconnected.movement_cells = &cells;
        assert_eq!(reconnected.home_temple(session).await, Some(temple));
        let mut current = fence(3)?;
        current.character_lease_generation = 2;
        let saved = h
            .root
            .read_character_last_position(&authority, &h.node, current)
            .await
            .map_err(debug)?
            .ok_or("respawn saved position")?;
        assert_eq!(saved.record.cell.x, 8);
        assert_eq!(saved.record.cell.y, 11);
        assert_eq!(saved.record.cell.floor, 7);
        eprintln!(
            "C20d-W23_POSTGRES_EXECUTED: citizenship, temple death respawn, reconnect reload; PostgreSQL 18.6; migration 0534"
        );
        drop(reconnected);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

const P20_NOW: i64 = 1_790_812_800_000_000;

struct P20Clock(std::sync::Mutex<Option<crate::premium::TrustedNow>>);
impl crate::gameplay_transport::spell_premium_coordinator::clock_registration::Registered
    for P20Clock
{
}
impl crate::gameplay_transport::spell_premium_coordinator::RegisteredPremiumClock for P20Clock {
    fn read_current_window(&self) -> Option<crate::premium::TrustedNow> {
        *self.0.lock().expect("clock")
    }
}

/// P20: delivered Premium reads with a trusted clock and the durable latch.
fn premium_reads(
    h: &Harness,
) -> (
    std::sync::Arc<crate::premium::PremiumConsumer>,
    crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator,
) {
    use crate::gameplay_transport::spell_premium_coordinator::{
        PremiumActivation, SpellPremiumCoordinator,
    };
    let consumer = std::sync::Arc::new(crate::premium::PremiumConsumer::default());
    let coordinator = SpellPremiumCoordinator::new(
        consumer.clone(),
        Some(std::sync::Arc::new(P20Clock(std::sync::Mutex::new(
            crate::premium::TrustedNow::new(P20_NOW, 1_000_000),
        )))),
        Some(PremiumActivation {
            activation_id: "p20-test".to_owned(),
            switch_over_us: P20_NOW - 60_000_000,
        }),
        Some(h.root.clone()),
    );
    (consumer, coordinator)
}

/// P20: Crystal 201189f8 `data-global/scripts/creaturescripts/others/login.lua:8-15`.
async fn premium_ended<'a>(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'a, '_, '_>,
    (consumer, coordinator): (
        &crate::premium::PremiumConsumer,
        &'a crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator,
    ),
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    (deps, request, entry): (
        &Dependencies,
        &crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1,
        PremiumEntry,
    ),
) -> TestResult {
    use crate::durability::premium_fence::{EntitlementState, PremiumEvidence};
    let now = P20_NOW;
    let evidence = |state| PremiumEvidence {
        account_id: id(40),
        producer_revision: "1".repeat(40),
        producer_profile: crate::premium::PRODUCER_PROFILE.to_owned(),
        product_id: crate::premium::PRODUCT_ID.to_owned(),
        product_version: crate::premium::PRODUCT_VERSION,
        entitlement_id: Some("01926c1e-8a40-7c3b-9f2e-5a1d3c4b6e70".to_owned()),
        state,
        lifecycle_revision: 1,
        authority_revision: 7,
        effective_from_us: now - 60_000_000,
        effective_until_us: now + 3_600_000_000,
        authority_issued_at_us: now,
        authority_valid_until_us: now + 3_600_000_000,
        refresh_after_us: now + 1_800_000_000,
    };
    let town = async |h: &Harness| -> TestResult<i32> {
        Ok(sqlx::query_scalar("SELECT town_id FROM game_character_home_towns WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
    };
    assert_eq!(owner.home_town(session, Some((actor, 11))).await, Some(11));
    // A restart before the first admission position write must not restore this premium cell
    // after citizenship has already changed to Thais.
    let old_cell = MovementLocalPosition {
        x: 14,
        y: 11,
        floor: 7,
    };
    let saved = {
        let runtime = owner.runtime.lock().await;
        let mut saved = owner.placed_record(&runtime, old_cell);
        saved.record.vitals = crate::durability::character_last_position::LastVitals::new(40, 3);
        saved.record.rest = Some([1, 2, 3, 4]);
        saved
    };
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    assert_eq!(
        owner
            .root
            .write_character_last_position(
                owner.character,
                owner.holder,
                fence,
                saved.write_sequence,
                saved.record
            )
            .await?,
        crate::durability::character_last_position::LastPositionWrite::Written
    );
    // Without Premium reads the admission is unchanged.
    owner.premium_coordinator = None;
    assert_eq!(owner.premium_ended_temple(id(40), session).await, None);
    owner.premium_coordinator = Some(coordinator);
    // Current Premium keeps the premium-area town.
    consumer.prove_for_test(evidence(EntitlementState::Active));
    assert_eq!(owner.premium_ended_temple(id(40), session).await, None);
    assert_eq!(town(h).await?, 11);
    match entry {
        PremiumEntry::Stored => {}
        PremiumEntry::Respawn => {
            let death = {
                let mut runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                states
                    .apply_creature_damage(
                        &mut runtime,
                        actor,
                        session,
                        100_000,
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                            owner.owner_now().get(),
                        ),
                    )
                    .ok_or("damage")?;
                states
                    .player_death(&runtime, actor, session)
                    .ok_or("death")?
            };
            assert!(matches!(
                owner.settle_death(actor, session, death, false).await.0,
                crate::gameplay_transport::SessionDeath::Durable
            ));
        }
        PremiumEntry::Arrival => {
            // An interrupted paid trip has this same obligation; seed only its durable input.
            sqlx::query("INSERT INTO game_character_pending_arrivals(character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,14,11,7)")
                .bind(id(41).as_slice()).bind(id(79).as_slice())
                .bind(owner.world_id.as_bytes().as_slice()).execute(&h.pool).await?;
        }
    }
    // An ended entitlement moves the citizen to Thais (town 8) and enters at its temple.
    consumer.prove_for_test(PremiumEvidence {
        authority_revision: 8,
        ..evidence(EntitlementState::Expired)
    });
    let thais = MovementLocalPosition {
        x: 2,
        y: 11,
        floor: 7,
    };
    assert_eq!(
        owner.premium_ended_temple(id(40), session).await,
        Some(thais)
    );
    assert_eq!(
        town(h).await?,
        11,
        "a placement decision does not commit citizenship"
    );
    let pin = deps.runtime.lock().await.content_pin().clone();
    let relocated = MovementLocalPosition { x: 3, ..thais };
    // Interrupt after the runtime placement but before its position write. All durable parts
    // must roll back together, and a brand-new owner must relocate again on the next login.
    sqlx::raw_sql("CREATE FUNCTION p20_fail_position() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'P20 injected position failure'; END; $$; CREATE TRIGGER p20_fail_position BEFORE INSERT OR UPDATE ON game_character_last_positions FOR EACH ROW EXECUTE FUNCTION p20_fail_position()")
        .execute(&h.pool).await?;
    for attempt in 0..3 {
        let fresh =
            Dependencies::with_content(pin.clone(), deps.room.clone(), deps.spells.clone())?;
        // The nominal temple is occupied. Persist the owner's actual radius fallback cell.
        {
            let mut runtime = fresh.runtime.lock().await;
            let occupant_session =
                crate::foundation::GameSessionId::decode(&id(82)).map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(occupant_session)
                .map_err(debug)?;
            let occupant = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_first_entry_position(occupant)
                .map_err(debug)?;
        }
        let mut login = fresh.owner(h, owner.character);
        login.active_generation = owner.active_generation;
        login.movement_cells = owner.movement_cells;
        login.premium_coordinator = Some(coordinator);
        let (outcome, _, decided) = Box::pin(login.initialize_first_entry(
            request,
            session,
            crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?,
            fresh.actor,
        ))
        .await;
        if attempt < 2 {
            assert_eq!(
                outcome,
                crate::gameplay_transport::connection::FirstEntryOutcome::RefusedUnavailable
            );
            // The refusal must reach the injected write, not an unrelated admission preflight.
            assert_eq!(
                fresh
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(fresh.actor)
                    .map_err(debug)?
                    .position(),
                relocated
            );
            assert_eq!(town(h).await?, 11);
            let pending: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_character_pending_respawns) + (SELECT count(*) FROM game_character_pending_arrivals)")
                .fetch_one(&h.pool).await?;
            assert_eq!(pending, i64::from(!matches!(entry, PremiumEntry::Stored)));
            let restored = h
                .root
                .read_character_last_position(owner.character, owner.holder, fence)
                .await?
                .ok_or("old position after interruption")?;
            assert_eq!(restored.record.cell, saved.record.cell);
            if attempt == 0 {
                sqlx::raw_sql("DROP TRIGGER p20_fail_position ON game_character_last_positions; CREATE TRIGGER p20_fail_position BEFORE INSERT ON game_character_home_town_receipts FOR EACH ROW EXECUTE FUNCTION p20_fail_position()")
                    .execute(&h.pool).await?;
            } else {
                sqlx::raw_sql("DROP TRIGGER p20_fail_position ON game_character_home_town_receipts; DROP FUNCTION p20_fail_position()")
                    .execute(&h.pool).await?;
            }
        } else {
            assert_eq!(
                outcome,
                crate::gameplay_transport::connection::FirstEntryOutcome::Positioned
            );
            assert_eq!(
                fresh
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(fresh.actor)
                    .map_err(debug)?
                    .position(),
                relocated
            );
            let budget =
                crate::gameplay_transport::AdmissionRetryBudget::for_root(&session, &h.root);
            assert!(
                login
                    .consume_admitted_newhaven_arrival(
                        &budget,
                        session,
                        fresh.actor,
                        decided.as_ref()
                    )
                    .await
            );
        }
    }
    assert_eq!(town(h).await?, 8);
    assert_eq!(owner.home_temple(session).await, Some(thais));
    // No periodic writer has run: a restarted admission already reads the actual relocation.
    let restored = owner
        .root
        .read_character_last_position(owner.character, owner.holder, fence)
        .await?
        .ok_or("durable relocated position")?;
    assert_eq!(
        restored.record.cell,
        crate::durability::character_last_position::LastPositionCell {
            x: relocated.x,
            y: relocated.y,
            floor: relocated.floor,
        }
    );
    let (vitals, rest) = if matches!(entry, PremiumEntry::Respawn) {
        (None, None)
    } else {
        (saved.record.vitals, saved.record.rest)
    };
    assert_eq!(restored.record.vitals, vitals);
    assert_eq!(restored.record.rest, rest);
    let pending: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_character_pending_respawns) + (SELECT count(*) FROM game_character_pending_arrivals)")
        .fetch_one(&h.pool).await?;
    assert_eq!(pending, 0, "the relocation consumed its obligation");
    // A free-area town stays where it is on the next login.
    assert_eq!(owner.premium_ended_temple(id(40), session).await, None);
    let receipts: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT town_before, town_after FROM game_character_home_town_receipts WHERE character_id=encode($1,'hex')::uuid ORDER BY committed_character_revision",
    ).bind(id(41).as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(receipts, vec![(8, 6), (6, 11), (11, 8)]);
    eprintln!("P20_POSTGRES_EXECUTED: ended Premium, premium-area town 11 -> Thais temple");
    owner.premium_coordinator = None;
    Ok(())
}

async fn revisions(h: &Harness) -> TestResult<(u64, u64)> {
    let (root, progression): (String, String) = sqlx::query_as(
        "SELECT r.character_revision::text,p.character_revision::text \
         FROM game_character_roots r JOIN game_character_progression_state p USING(character_id) \
         WHERE r.character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    Ok((root.parse()?, progression.parse()?))
}
fn world(world: crate::foundation::WorldId) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
        TerrainKind,
    };
    let manifest = Manifest {
        subareas: towns(),
        format: writer::FORMAT.into(),
        min_reader_version: writer::VERSION,
        projection_class: "server".into(),
        compiler_version: "pvp-live-test".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Default::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-7],
        },
        palette: vec![PaletteEntry {
            family: Family::Terrain,
            key: "oteryn:item.tibia.i100".into(),
            id: 0,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
            }),
        }],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    // The P20 restart case saves a real walkable cell in the premium town, Edron (14, 11).
    let tiles = [(2, 0), (3, 0), (8, 0), (9, 0), (14, 0)]
        .into_iter()
        .map(|(x, flags)| Tile {
            x,
            y: 11,
            flags,
            house: 0,
            zones: vec![],
            items: vec![Item {
                palette: 0,
                depth: 0,
                attrs: Attrs {
                    unique: (x == 3).then_some(9056),
                    ..Attrs::default()
                },
            }],
        })
        .collect();
    let bytes = writer::write(
        &manifest,
        &[Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles,
        }],
        &Default::default(),
    )
    .map_err(debug)?;
    let pins = crate::map::BundlePins {
        digest: oteryn_world_bundle::bundle::read(&bytes)
            .map_err(debug)?
            .digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "rev-1".into(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 2,
                y: 11,
                floor: -7,
            },
        },
        world,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
