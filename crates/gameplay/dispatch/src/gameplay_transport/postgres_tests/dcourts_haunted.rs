//! Dream Courts actions through PostgreSQL admission and real framed USE.
use super::{
    Dependencies, TestResult,
    fixture::{Harness, debug, id, run},
    prepare_current_session,
};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::{
    capabilities::SelectedCapabilities,
    connection::{FirstEntryOutcome, FreshAdmissionAuthority},
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Egg,
    Disguise,
    TombCurse,
    TombKey,
}

impl Case {
    fn key(self) -> &'static str {
        match self {
            Self::Egg => "oteryn:item.tibia.i29310",
            Self::Disguise => "oteryn:item.tibia.i29309",
            Self::TombCurse | Self::TombKey => "oteryn:item.tibia.i29349",
        }
    }

    fn tomb(self) -> bool {
        matches!(self, Self::TombCurse | Self::TombKey)
    }
}

#[test]
fn dcourts_haunted_cellar_egg_use_starts_bucket_decay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    run_case(Case::Egg)
}

#[test]
fn dcourts_haunted_cellar_full_bucket_use_on_self_commits_disguise() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    run_case(Case::Disguise)
}

#[test]
fn dcourts_haunted_tomb_curse_spawns_one_mummy_and_locks_sarcophagus() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    run_case(Case::TombCurse)
}

#[test]
fn dcourts_haunted_tomb_key_has_thirty_minute_absolute_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    run_case(Case::TombKey)
}

fn run_case(case: Case) -> TestResult {
    let start = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {case:?}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Haunted House scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {case:?} duration={:?} success={}",
        start.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let key = case.key();
        let harness =
            Harness::create_with_admission_seed(admin, "dcourts_haunted", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let origin = if case.tomb() {
            tomb_origin(deps.admitted.game_session_id, case == Case::TombKey)?
        } else {
            MovementLocalPosition {
                x: 6,
                y: 0,
                floor: 7,
            }
        };
        let (controller, pin) = haunted_fixture(deps.admitted.world_id)?;
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let oven = if case == Case::Egg {
            "oteryn:item.tibia.i29305"
        } else if case.tomb() {
            key
        } else {
            "oteryn:item.tibia.i2535"
        };
        let bag = "oteryn:item.tibia.i2854";
        let normal_egg = "oteryn:item.tibia.i233";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let states = if case == Case::Egg {
            vec![(
                normal_egg,
                reference(normal_egg).ok_or("normal egg")?.get() - 1,
            )]
        } else {
            Vec::new()
        };
        let (mut bytes, mut pins) = crate::map::boot::tests::bundle_with_item_states(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
            &states,
        );
        if case.tomb() {
            bytes = tomb_bundle(&bytes, origin)?;
            pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
        }
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: u16::try_from(origin.x)?,
                    y: u16::try_from(origin.y)?,
                    floor: -i8::try_from(origin.floor)?,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let bundle_room = super::rust_lottery::bundle_action_room(
            pin.qualified_room(),
            &map,
            deps.admitted.world_id,
            &cells,
        )?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            bundle_room.frame_binding().digest(),
            bundle_room.map_revision_digest(),
            (origin.x, origin.y, origin.floor),
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
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&bundle_room));
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
        if case.tomb() {
            super::item_look::seed_main_backpack(&harness, &[]).await?;
        } else {
            super::item_look::seed_main_backpack(&harness, &[(key, 1)]).await?;
        }
        {
            let mut seed = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *seed)
                .await?;
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
                SELECT character_id,$2,'oteryn:quest.the_dream_courts_quest',1,greatest(character_revision,2)
                FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice())
                .bind(oteryn_gameplay_items::haunted_cellar::QUESTLINE)
                .execute(&mut *seed)
                .await?;
            seed.commit().await?;
        }
        match case {
            Case::Egg => {
                bucket_on_egg(
                    &owner,
                    &harness,
                    &deps,
                    fence,
                    owner.bundle_map().ok_or("egg bundle map")?,
                    reference(key),
                    reference(oven),
                )
                .await?
            }
            Case::Disguise => {
                bucket_on_self(&owner, &harness, &deps, fence, reference(key)).await?
            }
            Case::TombCurse | Case::TombKey => {
                sarcophagus_use(
                    &owner,
                    &harness,
                    &deps,
                    fence,
                    origin,
                    case == Case::TombKey,
                )
                .await?;
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn bucket_on_self(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    deps: &Dependencies,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    source: Option<std::num::NonZeroU32>,
) -> TestResult {
    let target = {
        let runtime = owner.runtime.lock().await;
        let visible = runtime.visible_entities();
        let me = visible
            .players
            .iter()
            .find(|p| p.actor == deps.actor)
            .ok_or("self target")?;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: me.actor.placement_identity(),
            generation: me.generation,
        }
    };
    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 12, 15, 18, 350])
            .ok_or("offers")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("bucket handle")?),
        use_with: Some(target),
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (_, disposition, _, _) =
        super::rust_lottery::exchange_with_visuals(owner, admitted, 1, &payload, source).await?;
    assert_eq!(disposition, UseDisposition::Committed);
    let (key, duration): (String, i64) = sqlx::query_as(
        "SELECT o->>'key',(r.effects->'costume'->>'duration_ms')::bigint
           FROM game_item_consume_transform_records r,LATERAL jsonb_array_elements(r.outputs) o
           WHERE r.recipe='oteryn:haunted-cellar-disguise.v1'",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(
        (key.as_str(), duration),
        ("oteryn:item.tibia.i29310", 120_000)
    );
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states
        .get(&runtime, deps.actor, fence.game_session_id)
        .ok_or("player spell state")?;
    assert_eq!(
        state.conditions.outfit_at(owner.owner_now().get()),
        Some(19)
    );
    Ok(())
}

fn haunted_fixture(
    world: crate::foundation::WorldId,
) -> TestResult<(
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    use crate::content::native_gameplay::{NativeGameplayInput, test_support::try_activate_input};
    use sha2::{Digest, Sha256};

    let mut input = NativeGameplayInput::from_manifest(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json"),
    )?;
    let profiles = input.item_profiles.as_mut().ok_or("native item profiles")?;
    let mut document: serde_json::Value = serde_json::from_slice(&profiles.bytes)?;
    let records = document["records"]
        .as_array_mut()
        .ok_or("native item records")?;
    let template = records
        .iter()
        .find(|record| {
            record["production_definition"]["production_key"] == "oteryn:item.tibia.i653"
        })
        .ok_or("nonstackable inventory fixture template")?
        .clone();
    for id in [233, 29305, 29307, 29308, 29309, 29310, 29349, 29351] {
        let key = format!("oteryn:item.tibia.i{id}");
        if records
            .iter()
            .any(|record| record["production_definition"]["production_key"] == key)
        {
            continue;
        }
        let mut record = template.clone();
        record["authoring"]["item"]["key"] = format!("candidate:item/{id}").into();
        record["production_binding"]["external_id"] = id.to_string().into();
        record["production_binding"]["target"]["key"] = key.clone().into();
        record["production_definition"]["production_key"] = key.into();
        let duration = if id == oteryn_gameplay_items::haunted_tomb::KEY {
            Some(oteryn_gameplay_items::haunted_tomb::KEY_LIFETIME_MS)
        } else {
            oteryn_gameplay_items::haunted_cellar::bucket_duration_ms(id)
        };
        if let Some(duration) = duration {
            record["semantics"]["temporal"] = serde_json::json!({
                "state": "KNOWN",
                "value": {
                    "consumption_mode": {"state": "KNOWN", "value": "DURABLE_ABSOLUTE_DEADLINE"},
                    "decay_target": {"state": "NOT_APPLICABLE"},
                    "duration": {"state": "KNOWN", "value": duration},
                    "stop_duration": {"state": "KNOWN", "value": false}
                }
            });
        }
        records.push(record);
    }
    records.sort_by(|a, b| {
        a["production_definition"]["production_key"]
            .as_str()
            .cmp(&b["production_definition"]["production_key"].as_str())
    });
    profiles.bytes = serde_json::to_vec(&document)?;
    profiles.sha256 = Sha256::digest(&profiles.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    try_activate_input(world, &input).map_err(Into::into)
}

async fn bucket_on_egg(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    deps: &Dependencies,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    map: &crate::map::boot::BundleMap,
    bucket: Option<std::num::NonZeroU32>,
    egg: Option<std::num::NonZeroU32>,
) -> TestResult {
    use crate::map::{
        overlay::TilePos,
        view::{MapFacts, placement_key},
    };
    use oteryn_protocol_oteryn::world_object::{MapUseTarget, WorldObjectTarget};
    let at = TilePos {
        x: 7,
        y: 0,
        floor: -7,
    };
    let key = placement_key(at, 1).ok_or("egg placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 12, 15, 18, 350])
            .ok_or("offers")?;
    let source = (10_000_u32..20_000)
        .map(|nonce| {
            let mut source = id(71);
            source[12..].copy_from_slice(&nonce.to_be_bytes());
            source
        })
        .find(|source| {
            oteryn_gameplay_items::reward_bag::reward_bag_bounded_draw(
                *fence.game_session_id.as_bytes(),
                1,
                *source,
                47,
                11,
            )
            .is_some_and(|roll| roll >= 5)
        })
        .ok_or("successful deterministic fill fixture")?;
    let mut seed = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *seed)
        .await?;
    sqlx::query("UPDATE game_item_instances SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid")
        .bind(source.as_slice()).bind(id(71).as_slice()).execute(&mut *seed).await?;
    sqlx::query("UPDATE game_item_container_entries SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid")
        .bind(source.as_slice()).bind(id(71).as_slice()).execute(&mut *seed).await?;
    seed.commit().await?;
    let intent = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("bucket handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent).map_err(debug)?;
    let (_, result, _, _) =
        super::rust_lottery::exchange_with_visuals(owner, admitted, 1, &payload, bucket).await?;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(map.facts.object_revision(key), 1);
    let base = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("egg tile")?;
    let compact = *base.ids().get(1).ok_or("egg stack")?;
    assert_ne!(
        map.facts
            .base_entry(at, 1, compact)
            .ok_or("normal egg")?
            .definition,
        oteryn_protocol_oteryn::world_map::MapDefinition::Item(egg.ok_or("egg ref")?)
    );
    let (duration, delta, output): (i64, i64, String) = sqlx::query_as(
        "SELECT p.duration_millis,p.expires_at_unix_ms-r.occurred_at,i.definition_production_key
         FROM game_spell_item_temporal_schedules p JOIN game_item_consume_transform_records r
         ON r.transaction_id=p.source_transaction_id JOIN game_item_instances i USING(item_instance_id)",
    ).fetch_one(&harness.pool).await?;
    assert_eq!(
        (duration, delta, output.as_str()),
        (180_000, 180_000, "oteryn:item.tibia.i29307")
    );
    assert_eq!(map.facts.expire_movement_items(u64::MAX), 1);
    assert_eq!(
        map.facts
            .base_entry(at, 1, compact)
            .ok_or("restored egg")?
            .definition,
        oteryn_protocol_oteryn::world_map::MapDefinition::Item(egg.ok_or("egg ref")?)
    );
    Ok(())
}

fn tomb_origin(
    session: crate::foundation::GameSessionId,
    reward: bool,
) -> TestResult<MovementLocalPosition> {
    use crate::map::{overlay::TilePos, view::placement_key};
    use oteryn_gameplay_items::{haunted_tomb as rules, reward_bag::reward_bag_bounded_draw};
    for x in 33051..=33103 {
        for y in 32297..=32356 {
            if !rules::is_source(rules::SARCOPHAGUS, Some(rules::ACTION_ID), x, y, -8) {
                continue;
            }
            let key = placement_key(TilePos { x, y, floor: -8 }, 1).ok_or("sarcophagus key")?;
            let mut seed = [0; 16];
            seed[..8].copy_from_slice(&key.to_be_bytes());
            let roll = reward_bag_bounded_draw(*session.as_bytes(), 1, seed, 48, 10)
                .ok_or("fixture roll")?
                + 1;
            if (roll >= 7) == reward {
                return Ok(MovementLocalPosition {
                    x: i32::from(x) - 1,
                    y: i32::from(y),
                    floor: 8,
                });
            }
        }
    }
    Err("no canonical sarcophagus for the requested deterministic test branch".into())
}

fn tomb_bundle(bytes: &[u8], origin: MovementLocalPosition) -> TestResult<Vec<u8>> {
    let mut data = oteryn_world_bundle::bundle::read(bytes)?;
    let dx = u16::try_from(origin.x - 6)?;
    let dy = u16::try_from(origin.y)?;
    let mut grouped = std::collections::BTreeMap::new();
    for sector in data.sectors.drain(..) {
        for mut tile in sector.tiles {
            if tile.x == 7 {
                tile.items
                    .get_mut(1)
                    .ok_or("sarcophagus placement")?
                    .attrs
                    .action = Some(oteryn_gameplay_items::haunted_tomb::ACTION_ID);
                let mut right = tile.clone();
                right.items.truncate(1);
                right.x += dx + 1;
                right.y += dy;
                grouped
                    .entry((right.y / 32, right.x / 32))
                    .or_insert_with(Vec::new)
                    .push(right);
            }
            tile.x += dx;
            tile.y += dy;
            grouped
                .entry((tile.y / 32, tile.x / 32))
                .or_insert_with(Vec::new)
                .push(tile);
        }
    }
    data.manifest.world.min_x += dx;
    data.manifest.world.max_x += dx;
    data.manifest.world.min_y += dy;
    data.manifest.world.max_y += dy;
    data.manifest.world.floors = vec![-8];
    let sectors: Vec<_> = grouped
        .into_iter()
        .map(|((sy, sx), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            oteryn_world_bundle::bundle::Sector {
                floor: -8,
                sx,
                sy,
                tiles,
            }
        })
        .collect();
    Ok(oteryn_world_bundle::writer::write(
        &data.manifest,
        &sectors,
        &Default::default(),
    )?)
}

async fn sarcophagus_use(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    deps: &Dependencies,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    origin: MovementLocalPosition,
    reward: bool,
) -> TestResult {
    use crate::map::{overlay::TilePos, view::placement_key};
    use oteryn_protocol_oteryn::world_object::WorldObjectTarget;
    let map = owner.bundle_map().ok_or("Haunted Tomb bundle")?;
    let pos = TilePos {
        x: u16::try_from(origin.x + 1)?,
        y: u16::try_from(origin.y)?,
        floor: -8,
    };
    let key = placement_key(pos, 1).ok_or("sarcophagus placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 12, 15, 18, 350])
            .ok_or("offers")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::WorldObject(WorldObjectTarget {
            placement,
            expected_revision: 0,
        }),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, result, _, _) =
        super::rust_lottery::exchange_with_visuals(owner, admitted, 1, &payload, None).await?;
    assert_eq!(result, UseDisposition::Committed);
    let (_, cooldown, _, _) =
        super::rust_lottery::exchange_with_visuals(owner, next, 2, &payload, None).await?;
    assert_eq!(cooldown, UseDisposition::Committed);
    let records: Vec<(i16, i64, bool)> = sqlx::query_as(
        "SELECT roll,expires_at-occurred_at,reward_transaction_id IS NOT NULL FROM game_haunted_tomb_uses",
    ).fetch_all(&harness.pool).await?;
    let [(roll, duration, granted)] = records.as_slice() else {
        return Err(format!("expected one committed sarcophagus roll, found {records:?}").into());
    };
    assert_eq!((*roll >= 7, *duration, *granted), (reward, 30_000, reward));
    let schedules: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT s.definition_production_key,s.duration_millis,s.expires_at_unix_ms-r.occurred_at
         FROM game_spell_item_temporal_schedules s JOIN game_reward_claim_mint_receipts r
         ON r.transaction_id=s.source_transaction_id",
    )
    .fetch_all(&harness.pool)
    .await?;
    let runtime = owner.runtime.lock().await;
    let creatures = runtime.current_live_creature_census().map_err(debug)?;
    if reward {
        assert_eq!(
            schedules,
            vec![("oteryn:item.tibia.i29351".into(), 1_800_000, 1_800_000)]
        );
        assert!(creatures.is_empty());
    } else {
        assert!(schedules.is_empty());
        let [mummy] = creatures.as_slice() else {
            return Err(format!("expected one live Mummy, found {creatures:?}").into());
        };
        let facts = runtime.creature_combat_facts(*mummy, 0).map_err(debug)?;
        let policy = runtime
            .companion_policy(oteryn_gameplay_items::haunted_tomb::MUMMY)
            .map_err(debug)?;
        assert_eq!(facts.health, policy.maximum_health);
        let landing = runtime
            .read_actor_position(*mummy)
            .map_err(debug)?
            .position();
        assert_eq!((landing.y, landing.floor), (origin.y, origin.floor));
        assert!((origin.x + 1..=origin.x + 2).contains(&landing.x));
    }
    Ok(())
}
