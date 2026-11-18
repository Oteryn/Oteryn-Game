use super::*;

// Crystal 201189f8 actions_dwarf_disguise_kit.lua:1-13.
const BRIDGE_ROBBERY_SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_thieves_guild_quest/actions_dwarf_disguise_kit.lua:1-13";
const NURIK: &str = "oteryn:npc.nurik";
const NURIK_WELCOME: &str = "You must be the seller I'm expecting. What's your proposal?";
const NURIK_REFUSAL: &str = "I'm waiting for a different visitor.";
const READY_TO_SELL: &str = "oteryn:quest-callback/fixture/bridge-ready-to-sell";

fn nurik_catalogue() -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition, npc_rules::NpcRuleCatalog,
    };
    use serde_json::json;
    const CRYSTAL: &str =
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/nurik.lua:53-63";
    const GREETING: &str =
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/nurik.lua:105";
    const QUEST: &str = "oteryn:quest.the_thieves_guild_quest";
    const MISSION: &str = "oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest/mission04";
    let program = json!({
        "schema":1,"quest":QUEST,
        "tracks":[{"key":MISSION,"quest":QUEST,"initial":5,"min":-1,"max":7,"sources":[CRYSTAL]}],
        "rules":[
            {"key":"oteryn:quest-npc/thieves-guild/nurik/disguised","quest":QUEST,"npc":NURIK,
             "trigger":"greet","keywords":["hi","hello"],
             "guards":[{"kind":"storage","track":MISSION,"op":"eq","value":6},
                       {"kind":"temporary_outfit","look_type":66}],
             "reply":[NURIK_WELCOME],"sources":[CRYSTAL,GREETING]},
            {"key":"oteryn:quest-npc/thieves-guild/nurik/refusal","quest":QUEST,"npc":NURIK,
             "trigger":"greet","keywords":["hi","hello"],"reply":[NURIK_REFUSAL],
             "effects":[{"kind":"end_conversation","delay_ms":0}],"sources":[CRYSTAL]}
        ]
    }).to_string();
    let native = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: MISSION.into(), quest: QUEST.into(), initial: 5, min: -1, max: 7,
        }],
        vec![QuestTransition {
            key: READY_TO_SELL.into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: MISSION.into(),
                from: QuestComparison::Eq(5),
                effect: QuestEffectKind::Set(6),
            }],
            completes: false,
            experience: None,
            world_change: None,
        }],
    ).map_err(debug)?;
    Ok(NpcRuleCatalog::from_json_documents(&[&program])?.bind_fixture(native)?)
}

fn repin_fixture(pin: &mut crate::content::native_gameplay::PinnedGameplayBytes, bytes: Vec<u8>) {
    use sha2::{Digest, Sha256};
    pin.sha256 = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    pin.bytes = bytes;
}

fn qualify_disguise_kit(
    profiles: &mut crate::content::native_gameplay::PinnedGameplayBytes,
) -> TestResult {
    use serde_json::{Value, json};
    const KEY: &str = "oteryn:item.tibia.i7865";
    let mut document: Value = serde_json::from_slice(&profiles.bytes)?;
    let rows = document["records"].as_array_mut().ok_or("native item rows")?;
    if rows.iter().any(|row| row["production_definition"]["production_key"] == KEY) {
        return Ok(());
    }
    let canonical: Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/items/definitions/items-31500-31999.json",
    )))?;
    let definition = canonical["records"]
        .as_array()
        .and_then(|rows| rows.iter().map(|row| &row["definition"])
            .find(|item| item["identity"]["key"] == KEY))
        .ok_or("canonical disguise kit")?;
    let crystal: Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../imports/crystalserver/bindings/items.json",
    )))?;
    let binding = crystal["bindings"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["external_id"] == "7865"))
        .ok_or("Crystal disguise kit identity")?;
    rows.push(json!({
        "authoring":{"item":definition["identity"]},
        "semantics":definition["semantics"],
        "attributes":{},
        "admission":{"materializable":definition["materializable"],
            "stack_class":definition["stack_class"],
            "legal_destinations":["CharacterInventory","Ground"]},
        "production_definition":{"family":"Item","production_key":KEY,"revision_ref":"definition-r1"},
        "production_binding":binding,
        "production_binding_qualification":null
    }));
    repin_fixture(profiles, serde_json::to_vec(&document)?);
    Ok(())
}

fn disguise_input() -> TestResult<crate::content::native_gameplay::NativeGameplayInput> {
    let mut input = super::super::death_ml_loss_composed::production_input_with_source(
        "content-1", None,
    )?;
    qualify_disguise_kit(input.item_profiles.as_mut().ok_or("native item profiles")?)?;
    let appearances = input.spell_appearances.as_mut().ok_or("native spell appearances")?;
    let mut packet: serde_json::Value = serde_json::from_slice(&appearances.bytes)?;
    packet["item_costumes"] = serde_json::json!([{
        "item_id":7865,
        "creature":{"family":"Creature","key":"oteryn:creature.dwarf_geomancer","revision":"definition-r1"},
        "look_type":66,
        "duration_ms":300000,
        "source":BRIDGE_ROBBERY_SOURCE
    }]);
    repin_fixture(appearances, serde_json::to_vec(&packet)?);
    Ok(input)
}

pub(super) fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let key = case.key();
        let origin = case.origin();
        let ground_origin = MovementLocalPosition { x: origin.x - i32::from(case.feroxa()), ..origin };
        let harness =
            Harness::create_with_admission_seed(admin, "novelty_items", true, false).await?;
        super::super::reset();
        prepare_current_session(&harness).await?;
        if case.feroxa() {
            let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
                .fetch_one(&harness.pool).await?;
            eprintln!("QN5_FEROXA_TRAP_POSTGRES_READY version={version}");
        }

        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = if case == Case::BridgeRobberyDisguise {
            crate::content::native_gameplay::test_support::try_activate_input(
                deps.admitted.world_id, &disguise_input()?,
            ).map_err(debug)?
        } else {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            )
        };
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let oven = "oteryn:item.tibia.i2535";
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (mut bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
        );
        if case.feroxa() {
            let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
            let dx = u16::try_from(origin.x - 6)?;
            let dy = u16::try_from(origin.y)?;
            let floor = -i8::try_from(origin.floor)?;
            let mut grouped = std::collections::BTreeMap::new();
            for sector in data.sectors.drain(..) {
                for mut tile in sector.tiles {
                    tile.x += dx; tile.y += dy; tile.items.truncate(1);
                    grouped.entry((tile.y / 32, tile.x / 32)).or_insert_with(Vec::new).push(tile);
                }
            }
            data.manifest.world.min_x += dx; data.manifest.world.max_x += dx;
            data.manifest.world.min_y += dy; data.manifest.world.max_y += dy;
            data.manifest.world.floors = vec![floor];
            let sectors: Vec<_> = grouped.into_iter().map(|((sy, sx), tiles)|
                oteryn_world_bundle::bundle::Sector { floor, sx, sy, tiles }).collect();
            bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
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
        let bundle_room = super::super::rust_lottery::bundle_action_room(
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
            .initialize_pinned_test_position(
                deps.actor,
                origin,
            )
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
        if case.nested() {
            super::super::item_look::seed_main_backpack(&harness, &[(key, 1), (bag, 1)]).await?;
        } else {
            super::super::item_look::seed_main_backpack(&harness, &[(key, 1)]).await?;
        }
        let used = crate::gameplay_transport::item_look::typed_definition(
            index,
            reference(key).ok_or("source ref")?,
        )
        .ok_or("source definition")?;
        let seeded = id(71);
        let command_id = if case.nested() { 2 } else { 1 };
        let wanted_voodoo = match case {
            Case::VoodooSuccess | Case::GroundVoodoo => Some(true),
            Case::VoodooFailure => Some(false),
            _ => None,
        };
        let source = if case == Case::DiceStreak {
            (10_000_u32..20_000)
                .map(|nonce| {
                    let mut value = seeded;
                    value[12..].copy_from_slice(&nonce.to_be_bytes());
                    value
                })
                .find(|source| {
                    novelty_items::plan(
                        native,
                        &used,
                        *fence.game_session_id.as_bytes(),
                        command_id,
                        *source,
                    )
                    .is_some_and(|plan| plan.effects["dice_value"] == 6)
                })
                .ok_or("first six fixture")?
        } else if let Some(success) = wanted_voodoo {
            // Keep the first admitted command; choose a valid fixture identity to exercise
            // both exact source probability branches without replacing the real RNG.
            (1_u32..100_000)
                .map(|nonce| {
                    let mut value = seeded;
                    value[12..].copy_from_slice(&nonce.to_be_bytes());
                    value
                })
                .find(|source| {
                    novelty_items::plan(
                        native,
                        &used,
                        *fence.game_session_id.as_bytes(),
                        command_id,
                        *source,
                    )
                    .is_some_and(|plan| plan.effects["achievement"].is_string() == success)
                })
                .ok_or("source outcome")?
        } else {
            seeded
        };
        if source != seeded || case == Case::Hat || case.ground() || case.nested() {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            if source != seeded {
                sqlx::query("UPDATE game_item_instances SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid").bind(source.as_slice()).bind(seeded.as_slice()).execute(&mut *tx).await?;
                sqlx::query("UPDATE game_item_container_entries SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid").bind(source.as_slice()).bind(seeded.as_slice()).execute(&mut *tx).await?;
            }
            if case == Case::Hat {
                sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(fence.character_id.as_bytes().as_slice()).bind(source.as_slice()).bind(id(42).as_slice()).bind(id(79).as_slice()).execute(&mut *tx).await?;
            }
            if case.nested() {
                sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                    .bind(source.as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).bind(id(79).as_slice()).execute(&mut *tx).await?;
            }
            if case.ground() {
                let target = {
                    let runtime = owner.runtime.lock().await;
                    crate::durability::item_loop_move::LoopGround::from_native_owner(
                        &bundle_room,
                        &runtime,
                        ground_origin,
                        &crate::map::overlay::map_revision(&base),
                    )
                    .map_err(debug)?
                };
                sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).execute(&mut *tx).await?;
                let source_ref = [
                    crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,
                    id(79).as_slice(),
                ]
                .concat();
                sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,1)")
                    .bind(source.as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(&target.spatial_position).bind(source_ref)
                    .bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        let expected = if case == Case::BridgeRobberyDisguise {
            novelty_items::NoveltyPlan {
                recipe: "oteryn:novelty-item.i7865.v1".into(),
                outputs: vec![],
                consume_source: true,
                effects: serde_json::json!({
                    "kind":"costume","source_item_id":7865,"donor":BRIDGE_ROBBERY_SOURCE,
                    "cues":[],"private_speech":false,"require_head":false,
                    "require_player_target":false,"achievement_progress":[],
                    "costume":{"monster":"oteryn:creature.dwarf_geomancer","look_type":66,"duration_ms":300000}
                }),
            }
        } else if case.feroxa() {
            let output = &native.item_policy_for_source_id(22059).ok_or("open trap policy")?.record().production_definition;
            novelty_items::NoveltyPlan {
                recipe: "oteryn:feroxa-silvered-trap.v1".into(),
                outputs: vec![(crate::domain::item_definition::TypedDefinitionRef {
                    family: output.family.clone(), production_key: output.production_key.clone(),
                    revision_ref: output.revision_ref.clone(),
                }, 1)],
                consume_source: true,
                effects: serde_json::json!({
                    "kind":"transform", "source_item_id":22074,
                    "donor":"crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/grimvale/actions_closed_silvered_trap.lua",
                    "ground":{"x":ground_origin.x,"y":ground_origin.y,"floor":ground_origin.floor},
                    "cues":[{"binding":"canary.appearance:effect/poff","effect_id":3}],
                    "private_speech":false,"require_head":false,"require_player_target":false,
                    "achievement_progress":[],
                }),
            }
        } else if case.ground() {
            novelty_items::plan_ground(
                native,
                &used,
                *fence.game_session_id.as_bytes(),
                command_id,
                source,
                novelty_items::GroundEffect {
                    x: 6,
                    y: 0,
                    floor: 7,
                },
                false,
            )
            .ok_or("ground source admitted")?
        } else {
            novelty_items::plan(
                native,
                &used,
                *fence.game_session_id.as_bytes(),
                command_id,
                source,
            )
            .ok_or("novelty source admitted")?
        };
        let effects: NoveltyEffects = serde_json::from_value(expected.effects.clone())?;
        let before_vitals = {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            states
                .get(&runtime, deps.actor, fence.game_session_id)
                .ok_or("player")?
                .vitals()
        };
        let far_actor = if case == Case::VoodooFarTarget {
            let mut runtime = owner.runtime.lock().await;
            let session = crate::foundation::GameSessionId::decode(&id(67)).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let other = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    other,
                    MovementLocalPosition {
                        x: 10,
                        y: 0,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
            Some(other)
        } else {
            None
        };
        let target = if matches!(
            case,
            Case::VoodooSuccess | Case::VoodooFailure | Case::GroundVoodoo | Case::VoodooFarTarget
        ) {
            let runtime = owner.runtime.lock().await;
            let entities = runtime.visible_entities();
            let me = entities
                .players
                .iter()
                .find(|player| player.actor == far_actor.unwrap_or(deps.actor))
                .ok_or("target player")?;
            Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: me.actor.placement_identity(),
                generation: me.generation,
            })
        } else {
            None
        };
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let capabilities: &[u32] = if case.nested() {
            &[4, 6, 11, 12, 14, 15, 18, 350]
        } else {
            &[4, 6, 11, 12, 15, 18, 350]
        };
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), capabilities)
                .ok_or("offers")?;
        assert!(admitted.continuity.selected_capabilities.contains(350));
        assert!(!admitted.continuity.selected_capabilities.contains(23));
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
            use_with: target,
            use_with_map_item: None,
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let nurik = if case == Case::BridgeRobberyDisguise {
            owner.quest_catalogue = Some(std::sync::Arc::new(nurik_catalogue()?));
            owner.refresh_quest_session(admitted.game_session_id, false).await;
            let [npc] = super::super::classic_quests::install_with_offsets(
                &owner, &admitted, [NURIK], Some([(1, 0)]),
            ).await?;
            let reply = super::super::classic_quests::talk(&mut owner, &admitted, npc, 7865, "hi").await?;
            assert_eq!(reply, [NURIK_REFUSAL]);
            Some(npc)
        } else {
            None
        };
        if case == Case::PendingLifecycle {
            super::pending::exercise(&owner, &harness, deps.actor, fence, source, intent).await?;
            eprintln!("W19 POSTGRES_CASE_COMPLETED novelty_items case={case:?}");
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let (mut end, result, actual, wire_cues) =
            super::super::rust_lottery::exchange_with_visuals(
                &owner,
                admitted,
                command_id,
                &payload,
                reference(key),
            )
            .await?;
        let current: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&harness.pool).await?;
        if case.refused() {
            if case == Case::VoodooWithoutTarget {
                assert_eq!(result, UseDisposition::NoTarget);
            } else if case == Case::VoodooFarTarget {
                assert_eq!(result, UseDisposition::TooFar);
            } else {
                assert_ne!(result, UseDisposition::Committed);
            }
            assert_eq!(current, (1, 1));
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_item_consume_transform_records"
                )
                .fetch_one(&harness.pool)
                .await?,
                0
            );
        } else {
            assert_eq!(result, UseDisposition::Committed);
            assert_eq!(
                current,
                if expected.consume_source {
                    (0, 2)
                } else {
                    (1, 1)
                }
            );
            let (recipe, retained, hash_matches): (String, serde_json::Value, bool) = sqlx::query_as("SELECT r.recipe,r.effects,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_consume_transform_records r JOIN game_item_audit_outbox a USING(event_id)").fetch_one(&harness.pool).await?;
            assert_eq!(recipe, expected.recipe);
            assert_eq!(retained, expected.effects);
            assert!(hash_matches);
            let durable = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await?
                .ok_or("backpack")?;
            if case.ground() || case.nested() {
                let kind: i32 = sqlx::query_scalar(
                    "SELECT source_kind FROM game_item_consume_transform_records",
                )
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(kind, if case.ground() { 3 } else { 4 });
                let rows: Vec<(String, i64)> = if case.ground() {
                    sqlx::query_as("SELECT i.definition_production_key,i.quantity FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.lifecycle=1 ORDER BY g.stack_ordinal").fetch_all(&harness.pool).await?
                } else {
                    sqlx::query_as("SELECT i.definition_production_key,i.quantity FROM game_item_instances i JOIN game_item_nested_entries n USING(item_instance_id) WHERE i.lifecycle=1 ORDER BY n.placement_ordinal").fetch_all(&harness.pool).await?
                };
                if expected.consume_source {
                    assert_eq!(
                        rows,
                        expected
                            .outputs
                            .iter()
                            .map(|(definition, count)| (
                                definition.production_key.clone(),
                                i64::from(*count)
                            ))
                            .collect::<Vec<_>>()
                    );
                } else {
                    assert_eq!(rows, vec![(key.to_owned(), 1)]);
                }
                if case.ground() && !rows.is_empty() {
                    let ordinal: i64 =
                        sqlx::query_scalar("SELECT stack_ordinal FROM game_item_ground_locations")
                            .fetch_one(&harness.pool)
                            .await?;
                    assert_eq!(ordinal, 1);
                    assert_eq!(
                        harness
                            .root
                            .read_item_loop_ground(deps.admitted.world_id, deps.admitted.channel_id)
                            .await?
                            .len(),
                        rows.len()
                    );
                }
            } else if expected.consume_source {
                assert_eq!(durable.entries.len(), expected.outputs.len());
                for (entry, (definition, count)) in durable.entries.iter().zip(&expected.outputs) {
                    assert_eq!(&entry.item.definition, definition);
                    assert_eq!(entry.item.quantity, *count);
                }
            }
        }
        let (vitals, first_cues) = {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            assert!(states.pending_item_effects.is_empty());
            let state = states
                .get(&runtime, deps.actor, fence.game_session_id)
                .ok_or("player after")?;
            if let Some(costume) = &effects.costume {
                assert_eq!(
                    state.conditions.outfit_at(owner.owner_now().get()),
                    Some(costume.look_type)
                );
            }
            let vitals = state.vitals();
            let cues = match states.presentations.as_mut() {
                Some(publisher) => publisher
                    .take_for_current_publisher(&runtime)
                    .map_err(debug)?,
                None => Vec::new(),
            };
            (vitals, cues)
        };
        assert!(
            first_cues.is_empty(),
            "item cues use the live presentation journal"
        );
        let (visual_cursor, visuals) = owner
            .observe_sounds(deps.actor, fence.game_session_id, 0)
            .await
            .ok_or("item presentation")?;
        let actual_cues: Vec<_> = visuals
            .iter()
            .filter_map(|event| match event.kind {
                oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
                    effect_id,
                    position,
                    ..
                } => Some((effect_id, position)),
                _ => None,
            })
            .collect();
        let position = effects.ground.as_ref().map_or(
            oteryn_protocol_oteryn::world_spatial::ActorPosition {
                x: 6,
                y: 0,
                floor: 7,
            },
            |ground| oteryn_protocol_oteryn::world_spatial::ActorPosition {
                x: ground.x,
                y: ground.y,
                floor: ground.floor.abs(),
            },
        );
        let expected_cues: Vec<_> = effects
            .cues
            .iter()
            .filter(|_| !case.refused())
            .map(|cue| (u32::from(cue.effect_id), position))
            .collect();
        assert_eq!(
            actual_cues, expected_cues,
            "committed donor cues reach the live observer"
        );
        let wire_ids: Vec<_> = wire_cues
            .iter()
            .filter_map(|event| match event.kind {
                oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
                    effect_id,
                    position: at,
                    ..
                } => {
                    assert_eq!(at.x, position.x);
                    assert_eq!(at.y, position.y);
                    assert_eq!(at.floor, position.floor.abs());
                    Some(effect_id)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            wire_ids,
            effects
                .cues
                .iter()
                .filter(|_| !case.refused())
                .map(|cue| u32::from(cue.effect_id))
                .collect::<Vec<_>>(),
            "stationary USE sends the committed visuals over the real connection"
        );

        if case == Case::Rocket {
            assert_eq!(vitals.health, before_vitals.health - 10);
        } else {
            assert_eq!(vitals.health, before_vitals.health);
        }
        if !case.refused() {
            let original = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                end.continuity.selected_capabilities.as_slice(),
                &actual,
            )
            .map_err(debug)?;
            let replay = owner
                .use_event_item(
                    deps.actor,
                    UseCommand {
                        game_session_id: fence.game_session_id,
                        command_id,
                        item_fence: Some(fence),
                    },
                    original,
                    Some(ItemKey::Instance(source)),
                    true,
                )
                .await
                .map_err(|_| "replay unknown")?
                .ok_or("receipt replay")?;
            assert_eq!(replay.disposition, UseDisposition::Committed);
            let (replayed_end, replayed, _) =
                super::super::rust_lottery::exchange(&owner, end, command_id, &actual, None, None)
                    .await?;
            end = replayed_end;
            assert_eq!(replayed, UseDisposition::Committed);
            assert!(
                owner
                    .observe_sounds(deps.actor, fence.game_session_id, visual_cursor)
                    .await
                    .ok_or("replay visuals")?
                    .1
                    .iter().all(|event| !matches!(event.kind, oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect { .. })),
                "replay emits no item visual twice"
            );
            {
                let runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                assert_eq!(
                    states
                        .get(&runtime, deps.actor, fence.game_session_id)
                        .ok_or("replay player")?
                        .vitals(),
                    vitals
                );
                if let Some(publisher) = states.presentations.as_mut() {
                    assert!(
                        publisher
                            .take_for_current_publisher(&runtime)
                            .map_err(debug)?
                            .is_empty(),
                        "historical item replay never publishes deferred presentation"
                    );
                }
                if case == Case::BridgeRobberyDisguise {
                    use crate::domain::condition::ConflictKey;
                    use crate::foundation::condition::TemporaryDisplayedAppearance;
                    let conditions = states
                        .get(&runtime, deps.actor, fence.game_session_id)
                        .ok_or("disguised player")?
                        .owned_conditions();
                    let outfit = conditions.get(ConflictKey::Outfit).ok_or("outfit")?;
                    let expiry = outfit.started_at() + 300_000_000;
                    assert_eq!(outfit.remaining_duration_ms(outfit.started_at()), Some(300_000));
                    assert_eq!(conditions.outfit_at(expiry - 1), Some(66));
                    assert!(matches!(
                        conditions.displayed_temporary_appearance_at(expiry - 1),
                        Some(TemporaryDisplayedAppearance::Outfit(selection))
                            if selection.outfit_key == "oteryn:outfit.source.canary.look66"
                    ));
                    assert!(conditions.displayed_temporary_appearance_at(expiry).is_none());
                }
            }
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_item_consume_transform_records"
                )
                .fetch_one(&harness.pool)
                .await?,
                1
            );
        }
        if let Some(npc) = nurik {
            let talk = super::super::classic_quests::talk;
            assert_eq!(talk(&mut owner, &end, npc, 7866, "hi").await?, [NURIK_REFUSAL]);
            super::super::qengine_outfit_stories::commit_fixture(&owner, &end, READY_TO_SELL, 7867).await?;
            owner.refresh_quest_session(end.game_session_id, false).await;
            assert_eq!(talk(&mut owner, &end, npc, 7868, "hi").await?, [NURIK_WELCOME]);
            talk(&mut owner, &end, npc, 7869, "bye").await?;
            owner.clock_origin = owner.clock_origin
                .checked_sub(std::time::Duration::from_secs(300))
                .ok_or("disguise clock")?;
            assert_eq!(talk(&mut owner, &end, npc, 7870, "hi").await?, [NURIK_REFUSAL]);
        }
        if case.trumpet() {
            let (duration, delta): (i64, i64) = sqlx::query_as("SELECT p.duration_millis,p.expires_at_unix_ms-r.occurred_at FROM game_spell_item_temporal_schedules p JOIN game_item_consume_transform_records r ON r.transaction_id=p.source_transaction_id").fetch_one(&harness.pool).await?;
            assert_eq!((duration, delta), (4000, 4000));
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    owner.drain_source_item_deadlines().await;
                    if sqlx::query_scalar::<_, i64>(
                        "SELECT count(*) FROM game_spell_item_temporal_receipts",
                    )
                    .fetch_one(&harness.pool)
                    .await?
                        == 1
                    {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                Ok::<_, sqlx::Error>(())
            })
            .await??;
            let key: String = sqlx::query_scalar("SELECT i.definition_production_key FROM game_spell_item_temporal_schedules p JOIN game_item_instances i USING(item_instance_id)").fetch_one(&harness.pool).await?;
            assert_eq!(key, "oteryn:item.tibia.i6572");
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_spell_item_temporal_receipts"
                )
                .fetch_one(&harness.pool)
                .await?,
                1
            );
        }
        if case == Case::DiceStreak {
            let mut admitted = end;
            let earned = || {
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/number_of_the_beast'",
                )
            };
            assert_eq!(earned().fetch_one(&harness.pool).await?, 0);
            for (command, face) in [(2_u64, 6_u64), (3, 2), (4, 6), (5, 6), (6, 6)] {
                let start = u32::try_from(command)? * 10_000;
                let source = (start..start + 10_000)
                    .map(|nonce| {
                        let mut value = seeded;
                        value[12..].copy_from_slice(&nonce.to_be_bytes());
                        value
                    })
                    .find(|source| {
                        novelty_items::plan(
                            native,
                            &used,
                            *fence.game_session_id.as_bytes(),
                            command,
                            *source,
                        )
                        .is_some_and(|plan| plan.effects["dice_value"] == face)
                    })
                    .ok_or("source dice face fixture")?;
                let mut tx = harness.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i5792','definition-r1',1,1,game_character_uuid_v7())")
                    .bind(source.as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,coalesce(max(placement_ordinal),0)+1,game_character_uuid_v7() FROM game_item_container_entries WHERE parent_item_instance_id=encode($4,'hex')::uuid")
                    .bind(source.as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                let (next, outcome, actual) = super::super::rust_lottery::exchange_latest_source(
                    &owner,
                    admitted,
                    command,
                    &payload,
                    reference(key),
                )
                .await?;
                assert_eq!(outcome, UseDisposition::Committed);
                admitted = next;
                let roll: i64 = sqlx::query_scalar("SELECT (effects->>'dice_value')::bigint FROM game_item_consume_transform_records WHERE command_id=$1::bigint")
                    .bind(i64::try_from(command)?).fetch_one(&harness.pool).await?;
                assert_eq!(roll, i64::try_from(face)?);
                assert_eq!(
                    earned().fetch_one(&harness.pool).await?,
                    i64::from(command == 6)
                );
                if command == 6 {
                    let (_, outcome, _) = super::super::rust_lottery::exchange(
                        &owner, admitted, command, &actual, None, None,
                    )
                    .await?;
                    assert_eq!(outcome, UseDisposition::Committed);
                }
            }
            assert_eq!(earned().fetch_one(&harness.pool).await?, 1);
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_item_consume_transform_records"
                )
                .fetch_one(&harness.pool)
                .await?,
                6
            );
            let history: Vec<i64> = sqlx::query_scalar("SELECT (effects->>'dice_value')::bigint FROM game_item_consume_transform_records ORDER BY receipt_sequence")
                .fetch_all(&harness.pool).await?;
            assert_eq!(history, [6, 6, 2, 6, 6, 6]);
        }
        eprintln!("W19 POSTGRES_CASE_COMPLETED novelty_items case={case:?}");
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
