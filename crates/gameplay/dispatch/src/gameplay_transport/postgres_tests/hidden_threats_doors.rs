//! Source-pinned Hidden Threats retained-key door operations through actual framed item USE and PG.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[derive(Clone, Copy, Debug)]
enum Case {
    Works01,
    Works02,
    Works03,
    Unknown,
}
#[test]
fn hidden_threats_doors_works01_retains_key_and_unlocks_exact_source_door() -> TestResult {
    postgres(Case::Works01)
}
#[test]
fn hidden_threats_doors_works02_retains_key_and_unlocks_exact_source_door() -> TestResult {
    postgres(Case::Works02)
}
#[test]
fn hidden_threats_doors_works03_retains_key_and_unlocks_exact_source_door() -> TestResult {
    postgres(Case::Works03)
}
#[test]
fn hidden_threats_doors_unknown_saved_definition_stays_closed() -> TestResult {
    postgres(Case::Unknown)
}
fn postgres(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("hidden-threats-doors", move || {
        scenario(case)
    });
    println!(
        "POSTGRES_CASE_{} hidden_threats_doors case={case:?} duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}
fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "hidden_threats_doors", false, false).await?;
        let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(version, 180006);
        println!("POSTGRES_CASE_STARTED hidden_threats_doors case={case:?} version={version}");
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let at = case.position();
        let map = door_world(native, case)?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        assert_eq!(
            cells.scope().coordinate_frame.as_str(),
            "oteryn:frame/world-bundle-v3"
        );
        assert_eq!(
            cells.scope().map_revision.as_str(),
            crate::map::overlay::map_revision(map.base()).as_str()
        );
        assert_eq!(
            cells.scope().generation_digest,
            pin.identity().server_artifact_digest()
        );
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            map.map_revision_digest(),
            (i32::from(at.x - 1), i32::from(at.y), 12),
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
                MovementLocalPosition {
                    x: i32::from(at.x - 1),
                    y: i32::from(at.y),
                    floor: 12,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
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
        super::item_look::seed_main_backpack(&harness, &[]).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        door_steps(case, &mut owner, &mut admitted, &harness).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

impl Case {
    fn facts(self) -> (u16, u16, u16, u16) {
        match self {
            Self::Works01 | Self::Unknown => (33025, 32008, 45974, 1642),
            Self::Works02 => (33045, 32007, 45975, 1642),
            Self::Works03 => (33001, 32047, 45976, 1644),
        }
    }
    fn position(self) -> oteryn_gameplay_quests::quest::world_callbacks::Position {
        let (x, y, _, _) = self.facts();
        oteryn_gameplay_quests::quest::world_callbacks::Position { x, y, floor: -12 }
    }
    fn track(self) -> String {
        let suffix = match self {
            Self::Works01 | Self::Unknown => "01",
            Self::Works02 => "02",
            Self::Works03 => "03",
        };
        format!("oteryn:quest-progress/quest/u11_50/hidden_threats/corym_works_door{suffix}")
    }
}

fn door_world(
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    case: Case,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (x, y, action, appearance) = case.facts();
    let index = native.item_index().ok_or("index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let bag = "oteryn:item.tibia.i2854";
    let closed = format!("oteryn:item.tibia.i{appearance}");
    let opened = format!("oteryn:item.tibia.i{}", appearance + 1);
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_states(
        (bag, reference(bag).ok_or("bag")?.get() - 1),
        (&closed, reference(&closed).ok_or("closed door")?.get() - 1),
        &[(&opened, reference(&opened).ok_or("opened door")?.get() - 1)],
    );
    let mut data = bundle::read(&bytes)?;
    data.manifest.world = bundle::Extent {
        min_x: x - 1, min_y: y - 1, max_x: x + 2, max_y: y + 2, floors: vec![-12],
    };
    let palette = u32::try_from(
        data.manifest
            .palette
            .iter()
            .position(|p| p.key == closed)
            .ok_or("palette")?,
    )?;
    let mut sectors = std::collections::BTreeMap::<_, Vec<_>>::new();
    for ty in y - 1..=y + 1 {
        for tx in x - 1..=x + 1 {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            if (tx, ty) == (x, y) {
                let attrs = oteryn_world_bundle::sector::Attrs {
                    action: Some(action),
                    ..Default::default()
                };
                items.push(sector::Item {
                    palette,
                    depth: 0,
                    attrs,
                });
            }
            sectors
                .entry((-12, ty / 32, tx / 32))
                .or_default()
                .push(sector::Tile {
                    x: tx,
                    y: ty,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items,
                });
        }
    }
    let sectors = sectors
        .into_iter()
        .map(|((floor, sy, sx), tiles)| bundle::Sector {
            floor,
            sy,
            sx,
            tiles,
        })
        .collect::<Vec<_>>();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: x - 1,
                y,
                floor: -12,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| {
            let policy = native.item_policy(key, "definition-r1")?.record();
            let crate::content::ReferenceItemField::Known(physical) = &policy.semantics.physical
            else {
                return None;
            };
            Some(crate::map::facts::ItemDefinition {
                reference: reference(key)?,
                solid: policy.attributes.blocks_movement,
                blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                pickupable: physical.pickupable == crate::content::ReferenceItemField::Known(true),
                door: key == closed || key == opened,
            })
        },
    )
    .map_err(debug)
}

async fn door_steps(
    case: Case,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::map::view::MapFacts;
    let prior = owner.quest_catalogue.as_ref().and_then(|c|c.definition_hash("oteryn:quest.tibia_tales")).ok_or("Tibia Tales definition")?;
    println!("QUESTPC_ACTIVE_TIBIA_TALES_DEFINITION={}",prior.iter().map(|b|format!("{b:02x}")).collect::<String>());
    // Seed through the actual old-cohort owner: stored pins stay immutable.
    let old = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state_at_layer("content-1",0).map_err(debug)?;
    let old = old.catalogue();
    let seed_catalogue = if matches!(case,Case::Unknown) {
        let transition=old.transition("oteryn:quest-transition/quest/u11_50/hidden_threats/quest_line/npc_1").ok_or("old NPC transition")?.clone();
        let tracks=transition.effects.iter().map(|effect| old.track(&effect.track).cloned().ok_or("old track"))
            .collect::<Result<Vec<_>,_>>()?;
        oteryn_gameplay_quests::quest::QuestStateCatalogue::new("content-1",tracks,vec![transition]).map_err(debug)?
    } else { old.clone() };
    let seed_hash=seed_catalogue.definition_hash("oteryn:quest.tibia_tales").ok_or("seed hash")?;
    let saved_definition=seed_hash.iter().map(|b|format!("{b:02x}")).collect::<String>();
    if matches!(case,Case::Unknown) {
        assert!(oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(owner.quest_catalogue.as_ref().ok_or("current catalogue")?,"oteryn:quest.tibia_tales",&seed_hash).is_none());
    } else {
        assert_eq!(saved_definition,"8c6494650b3d22ccd66d6893c77f8b0ad16b06dfcd69725316150c1be49ee084");
    }
    let quest_fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "quest fence")?.ok_or("quest fence")?;
    let mut slot = owner.revision_sequencer.acquire(quest_fence.character_id).await;
    let seeded = slot.commit_quest_transition(owner.root, owner.character, owner.holder,
        quest_fence, crate::durability::quest_state::QuestTransitionRequest {
            transition_key: "oteryn:quest-transition/quest/u11_50/hidden_threats/quest_line/npc_1".into(),
            cause: crate::durability::quest_state::QuestCause::Command(crate::foundation::CommandRef::new(
                admitted.game_session_id, crate::foundation::CommandId::new(9000).map_err(debug)?)),
        }, std::sync::Arc::new(seed_catalogue)).await.map_err(debug)?;
    assert!(matches!(seeded,crate::durability::quest_state::QuestTransitionOutcome::Committed(_)));
    drop(slot);
    assert_eq!(sqlx::query_scalar::<_,String>("SELECT encode(definition_hash,'hex') FROM game_character_quest_states WHERE quest_key='oteryn:quest.tibia_tales'").fetch_one(&h.pool).await?,saved_definition);
    let at = case.position();
    let (x, y, _, appearance) = case.facts();
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(-12, x, y, 1).ok_or("placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = || WorldObjectTarget {
        placement: placement.clone(),
        expected_revision: map.facts.doors().revision(key),
    };
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let reference = |item: u16| {
        index.definition_ref(
            "Item",
            &format!("oteryn:item.tibia.i{item}"),
            "definition-r1",
        )
    };
    // This is a component fixture input, not a claim that the complete quest path is qualified.
    for (ordinal, item) in [(800, 27269), (801, 27268)] {
        super::quest_world_callbacks::input_item(
            owner,
            admitted.item_fence.ok_or("fence")?,
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: oteryn_gameplay_quests::quest::world_callbacks::Position {
                    x: x - 1,
                    ..at
                },
                item: format!("oteryn:item.tibia.i{item}"),
                count: 1,
            },
            ordinal,
            true,
        )
        .await?;
    }
    owner.states.chat.admit(admitted.game_session_id,"Corym",owner.owner_now());
    let original:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i27269'").fetch_one(&h.pool).await?;
    if matches!(case,Case::Unknown) {
        let payload=encode_item_use_intent(&UseIntent { target:UseTarget::Item(std::num::NonZeroU64::MIN),use_with:None,use_with_map_item:Some(MapUseTarget::Object(target())),use_with_position:None }).map_err(debug)?;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let command=admitted.continuity.next_command_id;
        let (next,result,_)=super::item_craft_carried_fluid::exchange_carried(owner,*admitted,command,&payload,reference(27269),None).await?;
        *admitted=next;
        assert_eq!(result,UseDisposition::Rejected,"an unrecognized saved definition cannot unlock");
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts").fetch_one(&h.pool).await?,1);
        assert_eq!(sqlx::query_scalar::<_,String>("SELECT encode(definition_hash,'hex') FROM game_character_quest_states WHERE quest_key='oteryn:quest.tibia_tales'").fetch_one(&h.pool).await?,saved_definition);
        let after:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i27269'").fetch_one(&h.pool).await?;
        assert_eq!(after,original);
        assert!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE track_key=$1").bind(case.track()).fetch_optional(&h.pool).await?.is_none());
        return Ok(());
    }
    let direct =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target()).map_err(debug)?;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let command = admitted.continuity.next_command_id;
    let (next, result, _) =
        super::item_craft::exchange(owner, *admitted, command, &direct, None).await?;
    *admitted = next;
    assert_eq!(
        result,
        UseDisposition::Locked,
        "carrying the key is not using it"
    );
    let intent = |target| UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent(target())).map_err(debug)?;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let command = admitted.continuity.next_command_id;
    let (next, result, _) = super::item_craft_carried_fluid::exchange_carried(
        owner,
        *admitted,
        command,
        &payload,
        reference(27268),
        None,
    )
    .await?;
    *admitted = next;
    assert_eq!(
        result,
        UseDisposition::NothingToUse,
        "wrong item while carrying the key cannot unlock"
    );
    let flag: Option<i64> =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(case.track())
            .fetch_optional(&h.pool)
            .await?;
    assert_eq!(flag, None);
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let command = admitted.continuity.next_command_id;
    let (next, result, actual) = super::item_craft_carried_fluid::exchange_carried(
        owner,
        *admitted,
        command,
        &payload,
        reference(27269),
        None,
    )
    .await?;
    *admitted = next;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "source forged key must unlock exact door"
    );
    let flag: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(case.track())
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(flag, 1);
    let after:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i27269'").fetch_one(&h.pool).await?;
    assert_eq!(
        after, original,
        "the exact key instance and its state remain unchanged"
    );
    assert!(owner.states.chat.drain(admitted.game_session_id).iter().any(|line|
        matches!(line,oteryn_protocol_oteryn::chat::ChatLine::Private{text,..} if text=="The forged key unlocks the door.")));
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    let (next, replay, _) = super::item_craft_carried_fluid::exchange_carried(
        owner, *admitted, command, &actual, None, None,
    )
    .await?;
    *admitted = next;
    assert_eq!(replay, UseDisposition::Committed);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
        receipts
    );
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let payload = encode_item_use_intent(&intent(target())).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (next, result, _) = super::item_craft_carried_fluid::exchange_carried(
        owner,
        *admitted,
        command,
        &payload,
        reference(27269),
        None,
    )
    .await?;
    *admitted = next;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "source handles an already unlocked flag without changing it"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
        receipts + 1
    );
    assert!(!owner.states.chat.drain(admitted.game_session_id).iter().any(|line|
        matches!(line,oteryn_protocol_oteryn::chat::ChatLine::Private{text,..} if text=="The forged key unlocks the door.")));
    let repeated_key:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i27269'").fetch_one(&h.pool).await?;
    assert_eq!(repeated_key,original);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE track_key=$1").bind(case.track()).fetch_one(&h.pool).await?,1);
    let direct =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target()).map_err(debug)?;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let command = admitted.continuity.next_command_id;
    let (next, result, _) =
        super::item_craft::exchange(owner, *admitted, command, &direct, None).await?;
    *admitted = next;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "existing quest-door owner now admits access"
    );
    assert!(!map.facts.doors().blocks(crate::map::overlay::TilePos { x, y, floor: -12 }));
    assert_eq!(
        map.facts
            .base_entry(
                crate::map::overlay::TilePos { x, y, floor: -12 },
                1,
                reference(appearance).ok_or("closed ref")?.get() - 1
            )
            .ok_or("door state")?
            .appearance_id,
        appearance + 1
    );
    Ok(())
}
