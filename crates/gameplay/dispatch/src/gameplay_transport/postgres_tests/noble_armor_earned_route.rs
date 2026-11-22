//! Skjaar's paid quiz and the map USE path for the earned crypt key.
use super::classic_quests_fixtures::talk;
use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks_fixtures::wire;
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::foundation::{ChannelContentPin, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleTrigger;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

const NPC: &str = "oteryn:npc.skjaar";
const GATE: TilePos = TilePos {
    x: 32448,
    y: 32042,
    floor: -8,
};
const DOOR: TilePos = TilePos {
    x: 32450,
    y: 32044,
    floor: -8,
};
const START: TilePos = TilePos {
    x: 32448,
    y: 32041,
    floor: -8,
};
const ARMOR: TilePos = TilePos {
    x: 32451,
    y: 32048,
    floor: -8,
};
const HELMET: TilePos = TilePos {
    x: 32455,
    y: 32048,
    floor: -8,
};

#[test]
fn noble_armor_postgres_skjaar_quiz_earns_key_and_opens_both_claims() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("noble-armor-earned", || scenario(false))
}

#[test]
fn noble_armor_postgres_wrong_answer_capacity_and_replay_preserve_payment() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("noble-armor-refusals", || scenario(true))
}

fn scenario(refusals: bool) -> TestResult {
    run(async |admin| {
        super::reset();
        let h =
            Harness::create_with_admission_seed(admin, "noble_armor_earned", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, activation) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active content")?;
        let native = generation.native_gameplay().ok_or("native content")?;
        let world = crypt(native)?;
        let room = activation.qualified_room().clone();
        let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            world.frame_binding_digest(world_id),
            world.map_revision_digest(),
            (i32::from(START.x), i32::from(START.y), 8),
        );
        let deps = Dependencies::with_content(
            pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut admitted = deps.admitted;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: i32::from(START.x),
                    y: i32::from(START.y),
                    floor: 8,
                },
            )
            .map_err(debug)?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(world.frame_binding_digest(world_id).as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let mut entries = vec![("oteryn:item.tibia.i3035", 9)];
        if refusals {
            let bag = crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            };
            let capacity =
                crate::gameplay_transport::item_ref_admission::generation_item_facts(native, &bag)
                    .map_err(debug)?
                    .container_capacity
                    .ok_or("backpack capacity")?;
            entries.resize(usize::try_from(capacity)?, ("oteryn:item.tibia.i3385", 1));
        }
        super::item_look_fixtures::seed_main_backpack(&h, &entries).await?;
        sqlx::query(
            "INSERT INTO game_type2_audit_activation VALUES(1,now()) ON CONFLICT DO NOTHING",
        )
        .execute(&h.pool)
        .await?;
        if !refusals {
            seed_wallet(&h).await?;
        }
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[4, 6, 11, 15, 18],
            )
            .ok_or("item USE capabilities")?;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        deps.initialize(&owner).await;
        let [npc] = super::classic_quests_fixtures::install_with_offsets(
            &owner,
            &admitted,
            [NPC],
            Some([(-1, 0)]),
        )
        .await?;
        let mut command = 1000;
        let mut say = async |owner: &mut ComposedFreshAdmission<'_, '_, '_>,
                             input,
                             expected: &str|
               -> TestResult {
            command += 1;
            let lines = talk(owner, &admitted, npc, command, input).await?;
            assert!(
                lines.iter().any(|line| line.contains(expected)),
                "{input}: {lines:?}"
            );
            Ok(())
        };
        say(&mut owner, "hi", "Why are you disturbing me?").await?;
        say(&mut owner, "key", "Would you like me to test you?").await?;
        say(&mut owner, "yes", "Are you willing to pay 1000 gold coins").await?;
        assert_eq!(payment(&h).await?, (0, 9, if refusals { 0 } else { 150 }));
        if refusals {
            say(&mut owner, "yes", "You don't have enough money").await?;
            assert_eq!(payment(&h).await?, (0, 9, 0));
            assert_eq!(
                super::npc_program_inventory_fixtures::topic(&owner, &admitted),
                Some(0)
            );
            // Adjust the same seeded coin stack without opening a new backpack slot.
            let mut fixture = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *fixture)
                .await?;
            sqlx::query("UPDATE game_item_instances SET quantity=11 WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).execute(&mut *fixture).await?;
            fixture.commit().await?;
            say(&mut owner, "key", "Would you like me to test you?").await?;
            say(&mut owner, "yes", "Are you willing to pay 1000 gold coins").await?;
        }
        say(&mut owner, "yes", "Dago's favourite pet").await?;
        let paid = if refusals { (1, 1, 0) } else { (1, 0, 50) };
        assert_eq!(payment(&h).await?, paid);
        let pay_id = command;
        let answer = talk(&mut owner, &admitted, npc, pay_id + 1, "spider").await?;
        assert!(answer.iter().all(|line| !line.contains("how many fingers")));
        assert_eq!(
            super::npc_program_inventory_fixtures::topic(&owner, &admitted),
            Some(3)
        );
        for (offset, input, expected) in [
            (2, "redips", "how many fingers"),
            (3, "7", "colour of the deamons"),
            (4, "black", "Do you want the key"),
        ] {
            let lines = talk(&mut owner, &admitted, npc, pay_id + offset, input).await?;
            assert!(
                lines.iter().any(|line| line.contains(expected)),
                "{lines:?}"
            );
        }
        let grant_id = pay_id + 5;
        let lines = talk(&mut owner, &admitted, npc, grant_id, "yes").await?;
        if refusals {
            assert!(lines.iter().all(|line| line != "Here you are"));
        } else {
            assert!(lines.iter().any(|line| line == "Here you are"));
        }
        let _ = talk(&mut owner, &admitted, npc, grant_id + 1, "yes").await?;
        for (id, topic, rule, expected) in [
            (pay_id, 2, "oteryn:quest-reward/skjaar-donation", true),
            (
                grant_id,
                6,
                "oteryn:quest-npc/noble-armor/skjaar/grant",
                !refusals,
            ),
        ] {
            let receipt = h
                .root
                .reconcile_npc_program(
                    &authority,
                    fence.character_id,
                    NpcProgramRequest {
                        command: CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(id).map_err(debug)?,
                        ),
                        npc: NPC.into(),
                        trigger: NpcRuleTrigger::Say,
                        message: "yes".into(),
                        topic,
                        transition_key: rule.into(),
                    },
                )
                .await
                .map_err(debug)?;
            if expected {
                let replay = receipt.ok_or("retained reply")?;
                assert!(matches!(
                    replay.transition,
                    QuestTransitionOutcome::AlreadyCommitted(_)
                ));
                assert!(replay.inventory_changed);
                if topic == 2 {
                    assert_eq!(replay.next_topic, 3);
                    assert!(
                        replay
                            .reply
                            .iter()
                            .any(|line| line.contains("Dago's favourite pet"))
                    );
                } else {
                    assert_eq!(replay.next_topic, 6);
                    assert_eq!(replay.reply, ["Here you are"]);
                }
            } else {
                assert!(receipt.is_none());
            }
        }
        assert_eq!(payment(&h).await?, paid);
        let carried = h
            .root
            .read_item_use_inventory(&authority, &h.node, fence)
            .await
            .map_err(debug)?;
        if refusals {
            assert!(carried.keys.is_empty());
        } else {
            assert!(matches!(carried.keys.as_slice(), [(_, 3142)]));
            enter_and_claim(&mut owner, &mut admitted, native, &authority).await?;
            let rewards: Vec<(String, i64)> = sqlx::query_as("SELECT definition_production_key,quantity FROM game_item_instances WHERE definition_production_key IN ('oteryn:item.tibia.i3380','oteryn:item.tibia.i3385') AND lifecycle=1 ORDER BY definition_production_key")
                .fetch_all(&h.pool).await?;
            assert_eq!(
                rewards,
                [
                    ("oteryn:item.tibia.i3380".into(), 1),
                    ("oteryn:item.tibia.i3385".into(), 1)
                ]
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn payment(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_item_fee_burns WHERE purchase_service_key='oteryn:quest-reward/skjaar-donation' AND fee_gold_units=1000), (SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3035' AND lifecycle=1), (SELECT coalesce(sum(balance),0)::bigint FROM game_account_bank_balances)")
        .fetch_one(&h.pool).await?)
}

async fn seed_wallet(h: &Harness) -> TestResult {
    // Retain a historical deposit prefix. The quiz must use the live BURN and bank debit owners.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_account_bank_operations
           (operation_occurrence_id,transaction_id,event_id,kind,request_binding,
            acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,
            amount,outcome,occurred_at,envelope_sha256,committed_at,
            planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,sha256('seed'),
            encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,
            150,0,1,sha256('seed'),0,encode($8,'hex')::uuid,encode($9,'hex')::uuid,encode($10,'hex')::uuid)",
    ).bind(id(230).as_slice()).bind(id(231).as_slice()).bind(id(232).as_slice())
     .bind(id(41).as_slice()).bind(id(40).as_slice()).bind(id(42).as_slice())
     .bind(id(43).as_slice()).bind(id(233).as_slice()).bind(id(234).as_slice())
     .bind(id(70).as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_account_bank_entries
           (entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,
                encode($4,'hex')::uuid,1,150,0,150,encode($5,'hex')::uuid)",
    ).bind(id(236).as_slice()).bind(id(231).as_slice()).bind(id(40).as_slice())
     .bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,150,encode($3,'hex')::uuid)",
    )
    .bind(id(40).as_slice())
    .bind(id(42).as_slice())
    .bind(id(236).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn enter_and_claim(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    native: &crate::content::native_gameplay::NativeGameplayState,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseIntent, UseTarget, encode_item_use_intent,
    };
    super::qengine_outfit_stories_fixtures::earn_level(owner, admitted, authority, 35).await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    {
        let runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .initialize(
                &runtime,
                actor,
                admitted.game_session_id,
                crate::spell::cast::CharacterCastFacts {
                    level: 35,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                (3, 7_000_000),
                owner.owner_now(),
            )
            .ok_or("level 35 projection")?;
    }
    assert_eq!(
        use_object(owner, admitted, GATE).await?,
        UseDisposition::Committed
    );
    walk_to(
        owner,
        admitted,
        TilePos {
            x: DOOR.x,
            y: DOOR.y - 1,
            floor: DOOR.floor,
        },
    )
    .await?;
    assert_eq!(
        use_object(owner, admitted, DOOR).await?,
        UseDisposition::Locked
    );
    let target = object(owner, DOOR)?;
    let reference = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i2970", "definition-r1"))
        .ok_or("earned steel key definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, result, _) = super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(reference),
    )
    .await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert!(!owner.bundle_map().ok_or("map")?.facts.doors().blocks(DOOR));
    for chest in [ARMOR, HELMET] {
        walk_to(
            owner,
            admitted,
            TilePos {
                y: chest.y - 1,
                ..chest
            },
        )
        .await?;
        assert_eq!(
            use_object(owner, admitted, chest).await?,
            UseDisposition::Committed
        );
        assert_eq!(
            use_object(owner, admitted, chest).await?,
            UseDisposition::NothingToUse
        );
    }
    Ok(())
}

async fn walk_to(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    position: TilePos,
) -> TestResult {
    super::desert_earned_route_fixtures::walk_to(
        owner,
        admitted,
        oteryn_gameplay_quests::quest::world_callbacks::Position {
            x: position.x,
            y: position.y,
            floor: position.floor,
        },
    )
    .await?;
    Ok(())
}

fn object(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    tile: TilePos,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("crypt map")?;
    let key = crate::map::view::placement_key(tile, 1).ok_or("crypt placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn use_object(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    tile: TilePos,
) -> TestResult<UseDisposition> {
    let command = admitted.continuity.next_command_id;
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&object(owner, tile)?)
        .map_err(debug)?;
    let reply = wire(owner, admitted, command, 2, payload).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}

fn crypt(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    let index = native.item_index().ok_or("item index")?;
    data.manifest.palette.truncate(1);
    for appearance in [6264, 6248, 2472] {
        let key = format!("oteryn:item.tibia.i{appearance}");
        let reference = index
            .definition_ref("Item", &key, "definition-r1")
            .ok_or("source map item")?;
        data.manifest.palette.push(bundle::PaletteEntry {
            key,
            family: bundle::Family::Item,
            id: reference.get() - 1,
            terrain: None,
        });
    }
    data.manifest.world = bundle::Extent {
        min_x: START.x - 1,
        min_y: START.y,
        max_x: HELMET.x,
        max_y: HELMET.y,
        floors: vec![-8],
    };
    let mut sectors = std::collections::BTreeMap::<_, Vec<_>>::new();
    // Keep a single approach through both source doors and a corridor to the two chests.
    let route = [
        (START.x - 1, START.y),
        (START.x, START.y),
        (GATE.x, GATE.y),
        (GATE.x, GATE.y + 1),
        (GATE.x + 1, GATE.y + 1),
    ]
    .into_iter()
    .chain((DOOR.y - 1..=ARMOR.y - 1).map(|y| (DOOR.x, y)))
    .chain((ARMOR.x..=HELMET.x).map(|x| (x, ARMOR.y - 1)))
    .chain([(ARMOR.x, ARMOR.y), (HELMET.x, HELMET.y)]);
    for (x, y) in route {
        let tile = TilePos { x, y, floor: -8 };
        let mut items = vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: Default::default(),
        }];
        // Crystal door_level.lua:65-74, door_key.lua:202-205 and chest.lua:1603-1617.
        let binding = match tile {
            GATE => Some((1, None, Some(1035))),
            DOOR => Some((2, None, Some(3142))),
            ARMOR => Some((3, Some(6132), None)),
            HELMET => Some((3, Some(6133), None)),
            _ => None,
        };
        if let Some((palette, unique, action)) = binding {
            items.push(sector::Item {
                palette,
                depth: 0,
                attrs: sector::Attrs {
                    unique,
                    action,
                    ..Default::default()
                },
            });
        }
        sectors
            .entry((x / 32, y / 32))
            .or_default()
            .push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items,
            });
    }
    let sectors: Vec<_> = sectors
        .into_iter()
        .map(|((sx, sy), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            bundle::Sector {
                floor: -8,
                sx,
                sy,
                tiles,
            }
        })
        .collect();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: START,
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| {
            use crate::content::{ReferenceItemField, ReferenceItemType};
            let policy = native.item_policy(key, "definition-r1")?.record();
            Some(crate::map::facts::ItemDefinition {
                reference: index.definition_ref("Item", key, "definition-r1")?,
                solid: policy.attributes.blocks_movement,
                blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                pickupable: false,
                door: matches!(&policy.semantics.classification, ReferenceItemField::Known(c)
                    if c.item_type == ReferenceItemField::Known(ReferenceItemType::Door)),
            })
        },
    )
    .map_err(debug)
}
