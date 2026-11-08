//! A synthetic walkable World at the donor coordinates, compiled with the repository
//! bundle writer. Creature and Item definitions are the unchanged admitted production set.
use super::super::fixture::{debug, id};
use super::super::{Dependencies, TestResult};
use crate::durability::arena_match::donor;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

pub(in super::super) fn bundle(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::writer::{Extent, Sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
        ("oteryn:item.tibia.i2854", 0),
        ("oteryn:item.tibia.i3031", 1),
    );
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    bundle.manifest.palette.truncate(3);
    let ground = "oteryn:item.tibia.i100";
    let reference = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", ground, "definition-r1"))
        .ok_or("ground reference")?;
    bundle.manifest.palette[0].key = ground.into();
    bundle.manifest.palette[0].family = oteryn_world_bundle::bundle::Family::Item;
    bundle.manifest.palette[0].id = reference.get() - 1;
    for source in [1841, 5022] {
        let key = format!("oteryn:item.tibia.i{source}");
        let reference = native.item_index()
            .and_then(|index| index.definition_ref("Item", &key, "definition-r1"))
            .ok_or("arena item reference")?;
        bundle.manifest.palette.push(oteryn_world_bundle::writer::PaletteEntry {
            key, family: oteryn_world_bundle::bundle::Family::Item,
            id: reference.get() - 1, terrain: None,
        });
    }
    bundle.manifest.world = Extent {
        min_x: 32160,
        min_y: 31045,
        max_x: 32260,
        max_y: 31105,
        floors: vec![-7, -6],
    };
    let mut tiles = std::collections::BTreeMap::new();
    for (x, y, z) in donor::CENTERS
        .iter()
        .map(|&(x, y)| (x, y, 7))
        .chain([donor::KICK, donor::REWARD_ROOM])
    {
        for dx in -6..=5 {
            for dy in -5..=5 {
                let at = ((x + dx) as u16, (y + dy) as u16, -z as i8);
                let mut tile = bundle.sectors[0].tiles[0].clone();
                tile.x = at.0;
                tile.y = at.1;
                tiles.insert(at, tile);
            }
        }
    }
    for &(x, y) in &donor::CENTERS {
        let tile = tiles.get_mut(&((x - 6) as u16, y as u16, -7))
            .ok_or("pillar tile")?;
        tile.items.push(oteryn_world_bundle::sector::Item {
            palette: 3, depth: 0, attrs: Default::default(),
        });
    }
    let mut sectors = std::collections::BTreeMap::<(i8, u16, u16), Sector>::new();
    for ((x, y, z), tile) in tiles {
        let key = (z, x / 32, y / 32);
        sectors
            .entry(key)
            .or_insert_with(|| Sector {
                floor: z,
                sx: key.1,
                sy: key.2,
                tiles: Vec::new(),
            })
            .tiles
            .push(tile);
    }
    let mut sectors: Vec<_> = sectors.into_values().collect();
    for sector in &mut sectors {
        sector.tiles.sort_by_key(|t| (t.y, t.x));
    }
    sectors.sort_by_key(|s| (s.floor, s.sy, s.sx));
    let bytes = oteryn_world_bundle::writer::write(&bundle.manifest, &sectors, &bundle.spawns)
        .map_err(debug)?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    Ok((bytes, pins))
}

pub(in super::super) fn runtime(
    deps: &mut Dependencies,
    active: &crate::content::ActiveGeneration,
    pin: &crate::content::NativeEntryContentPin,
    map: &crate::map::boot::BundleWorld,
) -> TestResult {
    runtime_at(
        deps,
        active,
        pin,
        map,
        crate::foundation::MovementLocalPosition {
            x: donor::KICK.0,
            y: donor::KICK.1,
            floor: donor::KICK.2,
        },
    )
}

pub(in super::super) fn runtime_at(
    deps: &mut Dependencies,
    active: &crate::content::ActiveGeneration,
    pin: &crate::content::NativeEntryContentPin,
    map: &crate::map::boot::BundleWorld,
    start: crate::foundation::MovementLocalPosition,
) -> TestResult {
    deps.room = pin.qualified_room().clone();
    let cells = map
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    deps.combat_room = Some(
        deps.room
            .bundle_view(
                cells,
                map.frame_binding_digest(deps.admitted.world_id),
                map.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: start.x,
                    y: start.y,
                    floor: start.floor,
                },
            )
            .ok_or("bundle combat view")?,
    );
    let content_pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        map.frame_binding_digest(deps.admitted.world_id),
        map.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        32,
        content_pin,
    )
    .map_err(debug)?;
    active
        .native_gameplay()
        .ok_or("native")?
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
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
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    deps.admitted.runtime_actor = Some(deps.actor);
    deps.runtime = tokio::sync::Mutex::new(runtime);
    Ok(())
}

pub(in super::super) async fn equip_sword(h: &super::Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3271','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(id(210).as_slice()).bind(id(42).as_slice()).bind(id(211).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,6,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(42).as_slice()).bind(id(212).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(in super::super) async fn fight(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    actor: crate::foundation::ExactActorRef,
    target: crate::foundation::ExactActorRef,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use oteryn_protocol_oteryn::attack::{
        AttackIntentDisposition, ChaseMode, FightMode, FightModes,
    };
    let id = admitted.continuity.next_command_id;
    let reference = {
        let runtime = owner.runtime.lock().await;
        let creature = runtime
            .visible_entities()
            .creatures
            .iter()
            .find(|creature| creature.actor == target)
            .copied()
            .ok_or("target")?;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: target.placement_identity(),
            generation: creature.generation,
        }
    };
    let (status, bytes) = super::super::guild_wire::exchange(
        owner,
        admitted,
        id,
        oteryn_protocol_oteryn::attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
        &oteryn_protocol_oteryn::attack::encode_attack_target_intent(Some(&reference))
            .map_err(debug)?,
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    assert_eq!(
        oteryn_protocol_oteryn::attack::decode_attack_intent_result(&bytes).map_err(debug)?,
        AttackIntentDisposition::Ok
    );
    assert_eq!(
        owner
            .fight_modes(
                actor,
                admitted.game_session_id,
                FightModes {
                    fight_mode: FightMode::Offensive,
                    chase: ChaseMode::Chase,
                    secure: true
                }
            )
            .await,
        AttackIntentDisposition::Ok
    );
    for swing in 0..512 {
        owner.clock_origin -= std::time::Duration::from_secs(3);
        let _ = owner.chase_step(actor, admitted.game_session_id).await;
        owner
            .refresh_attack_weapon(actor, admitted.game_session_id)
            .await;
        owner.drain_auto_attacks().await;
        if !owner.runtime.lock().await.contains_live_creature(target) {
            eprintln!(
                "SV15 COMBAT_PROOF: opponent defeated after {} normal attack turns",
                swing + 1
            );
            return Ok(());
        }
    }
    Err("arena opponent survived 512 normal attack turns".into())
}

pub(in super::super) async fn train_character(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
) -> TestResult {
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
    };
    let formula = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.training_formula())
        .ok_or("training formula")?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "current fence")?
        .ok_or("session")?;
    let before = owner
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await?;
    let chosen = convert_vocation(formula, &before, "knight").map_err(debug)?;
    owner
        .root
        .commit_character_build(
            owner.character,
            owner.holder,
            fence,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(214)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before,
                after: chosen.clone(),
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    let mut skills = chosen.skills();
    skills[2] = (100, 0);
    let trained = DurableBuildState::new("knight", chosen.magic(), skills).map_err(debug)?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "current fence")?
        .ok_or("session")?;
    owner
        .root
        .commit_character_build(
            owner.character,
            owner.holder,
            fence,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(215)).map_err(debug)?,
                cause: BuildCause::Training,
                before: chosen,
                after: trained,
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    Ok(())
}
