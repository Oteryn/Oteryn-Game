//! Crystal 201189f8 data-global/scripts/quests/roshamuul_quest/actions_trough.lua:3-16
//! and data-global/npc/sandomo.lua:53-59,97-165.
use std::num::NonZeroU64;
use std::sync::Arc;

use super::archivist_vote::{FixtureItem, install_fixture_item_profiles};
use super::classic_quests::install_above;
use super::contribution_redemption::{
    CLUSTER, FRAZZLEMAWS, GOLD, ITEM, MORTAR, NPC, SILENCERS, sandomo_catalogue,
};
use super::fixture::{Harness, debug, id, run};
use super::hireling_lifecycle::cook::{Client, drive};
use super::item_craft::exchange;
use super::quest_world_callbacks::{input_item, world_with_objects};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    authored_bindings::Bindings,
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use serde_json::json;

const QUEST: &str = "oteryn:quest.roshamuul_quest";
const FILLED: &str = "oteryn:item.tibia.i20170";
const EMPTY: &str = "oteryn:item.tibia.i2873";
// Fixture placement. The live binding uses the pin-qualified source trough placement.
const TROUGH: Position = Position {
    x: 33500,
    y: 32300,
    floor: -7,
};
const OTHER: Position = Position {
    x: 33499,
    y: 32301,
    ..TROUGH
};

#[test]
fn roshamuul_mortar_use_returns_selected_bucket_and_funds_one_sandomo_cluster() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h =
                    Harness::create_with_admission_seed(admin, "roshamuul_mortar", false, false)
                        .await?;
                let result = Box::pin(exercise(&h)).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Roshamuul mortar case panicked")??;
    Ok(())
}

async fn exercise(h: &Harness) -> TestResult {
    let source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/roshamuul_quest/actions_trough.lua:3-16";
    let callback_json = json!({
        "key":"oteryn:quest-callback/roshamuul-mortar-trough","quest":QUEST,"trigger":"use",
        "selector":{"appearance":20216,"native":{"position":TROUGH,"ordinal":1,"appearance":20216}},
        "requires_world_admission":true,"reversible":false,"passages":[],"storage":[],"offerings":[],
        "item_on_target":{"item":FILLED,"count":1,"replacement":EMPTY,"selected_instance":true},
        "state_ops":[{"track":MORTAR,"from":{"kind":"any"},"effect":{"kind":"add_at_least","value":1,"minimum":1}}],
        "effects":{"magic":{"position":TROUGH,"effect":10}},
        "return_on_failure":false,"sources":[source]
    });
    let callback: Callback = serde_json::from_value(callback_json.clone())?;
    let bindings = Bindings::parse(json!({
        "schema":1,"quests":[{
            "quest":QUEST,
            "recipe_sha256":<sha2::Sha256 as sha2::Digest>::digest(source)
                .iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            "tracks":[{"key":MORTAR,"initial":-1,"minimum":-1,"maximum":i64::MAX}],
            "stages":[{"key":"throw-mortar","completes":false,"trigger":{"kind":"map_use","callback":callback_json}}],
            "sources":[source]
        }]
    }).to_string().as_bytes())?;
    // The production loader attaches map bindings after extending the NPC rules.
    // Leave all four tracks at Crystal's absent value until real commands write them.
    let catalogue = bindings.attach(&sandomo_catalogue(-1, [-1; 3])?)?;
    let world = world_with_objects(&callback, TROUGH, &[(OTHER, 100)], &[])?;

    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let mut input = super::death_ml_loss_composed::production_input_with_source("content-1", None)?;
    install_fixture_item_profiles(
        &mut input,
        &[
            FixtureItem {
                id: 20170,
                name: "bucket of mortar",
                weight: 200,
                stackable: false,
            },
            CLUSTER,
        ],
    )?;
    let (controller, activated) =
        crate::content::native_gameplay::test_support::try_activate_input(world_id, &input)
            .map_err(debug)?;
    let generation = controller.active().ok_or("active content")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let room = activated.qualified_room();
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    let digest = generation.identity().server_artifact_digest();
    let client_digest = generation.identity().client_artifact_digest();
    let frame = activated.frame_binding().digest();
    let map = world.map_revision_digest();
    let start = crate::content::NativeEntryStart {
        x: i32::from(TROUGH.x) - 1,
        y: i32::from(TROUGH.y),
        floor: -i16::from(TROUGH.floor),
    };
    let pin = ChannelContentPin::from_activation(
        world_id,
        1,
        digest,
        client_digest,
        frame,
        map,
        (start.x, start.y, start.floor),
    );
    let mut deps = Dependencies::with_content(
        pin,
        room.clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    deps.combat_room = Some(
        room.bundle_view(cells.clone(), frame, map, start)
            .ok_or("mortar bundle")?,
    );
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(catalogue));
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
            .ok_or("item map USE capabilities")?;
    super::qd_world::select_talk_capabilities(&owner, &mut admitted, &[3, 6])?;
    let [npc] = install_above(&owner, &admitted, [NPC]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    admitted.item_fence = Some(fence.item_fence());
    deps.runtime
        .lock()
        .await
        .bind_attacker_lease(
            deps.actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(digest.as_slice())
        .bind(client_digest.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            digest,
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let template = crate::durability::starter_grant::StarterTemplate {
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        ..crate::durability::starter_grant::StarterTemplate::main()
    };
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            &template,
        )
        .await
        .map_err(debug)?;
    deps.initialize(&owner).await;
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_quest_session(&admitted).await;
    input_item(
        &owner,
        fence.item_fence(),
        &Offering {
            position: TROUGH,
            item: FILLED.into(),
            count: 1,
        },
        1,
        true,
    )
    .await?;
    let instance: String = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(FILLED).fetch_one(&h.pool).await?;
    let read = || async {
        sqlx::query_as::<_, (String, i64, Option<i16>, Option<i64>, i64)>(
            "SELECT i.definition_production_key, i.quantity, f.fluid_subtype, \
               (SELECT value FROM game_character_quest_tracks \
                WHERE character_id=encode($2,'hex')::uuid AND track_key=$3), \
               (SELECT count(*) FROM game_quest_callback_consumptions c \
                WHERE c.item_instance_id=i.item_instance_id AND c.transition_key=$4) \
             FROM game_item_instances i \
             JOIN game_item_container_entries e USING(item_instance_id) \
             LEFT JOIN game_item_fluid_states f USING(item_instance_id) \
             WHERE i.item_instance_id=$1::uuid AND i.lifecycle=1 \
             AND e.character_id=encode($2,'hex')::uuid",
        )
        .bind(&instance)
        .bind(id(41).as_slice())
        .bind(MORTAR)
        .bind(&callback.key)
        .fetch_one(&h.pool)
        .await
    };
    let map = owner.bundle_map().ok_or("map")?;
    let item_index = native.item_index().ok_or("item index")?;
    let reference = item_index
        .definition_ref("Item", FILLED, "definition-r1")
        .ok_or("filled bucket definition")?;
    let use_on = |target: Position| -> TestResult<Vec<u8>> {
        let key = oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, 1)
            .ok_or("fixture placement")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        Ok(encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            })),
            use_with_position: None,
        })
        .map_err(debug)?)
    };
    let before = read().await?;
    assert_eq!(before, (FILLED.into(), 1, None, None, 0));
    let (admitted, wrong, _) = exchange(
        &owner,
        admitted,
        admitted.continuity.next_command_id,
        &use_on(OTHER)?,
        Some(reference),
    )
    .await?;
    assert_ne!(wrong, UseDisposition::Committed);
    assert_eq!(read().await?, before);
    let command = admitted.continuity.next_command_id;
    let (admitted, result, actual) =
        exchange(&owner, admitted, command, &use_on(TROUGH)?, Some(reference)).await?;
    assert_eq!(result, UseDisposition::Committed);
    let committed = (EMPTY.into(), 1, None, Some(1), 1);
    assert_eq!(read().await?, committed);
    let effects = map.facts.movement_effects_since(0);
    let trough_effects: Vec<_> = effects.iter().filter(|cue| cue.2 == 10).collect();
    assert_eq!(trough_effects.len(), 1);
    assert_eq!(
        trough_effects[0].1,
        crate::map::overlay::TilePos {
            x: TROUGH.x,
            y: TROUGH.y,
            floor: TROUGH.floor
        },
    );

    let (admitted, replayed, _) = exchange(&owner, admitted, command, &actual, None).await?;
    assert_eq!(replayed, UseDisposition::Committed);
    assert_eq!(read().await?, committed);
    assert_eq!(map.facts.movement_effects_since(0), effects);
    redeem_mortar(h, &owner, admitted, npc).await
}

async fn redeem_mortar(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    let amount = |track: &'static str| async move {
        sqlx::query_scalar::<_, i64>(
            "SELECT value FROM game_character_quest_tracks
             WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
        )
        .bind(id(41).as_slice())
        .bind(track)
        .fetch_optional(&h.pool)
        .await
    };
    let clusters = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_instances i
             JOIN game_item_container_entries e USING(item_instance_id)
             WHERE i.definition_production_key=$2 AND i.lifecycle=1
             AND e.character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .bind(ITEM)
        .fetch_one(&h.pool)
        .await
    };
    assert_eq!(amount(GOLD).await?, None);
    assert_eq!(clusters().await?, 0);
    let (mut server, peer) = tokio::io::duplex(1 << 20);
    drive(owner, admitted, &mut server, async {
        let mut client = Client::new(peer, admitted);
        for word in ["hi", "gratitude", "repairs", "books", "yes"] {
            client.talk(npc, word).await?;
        }
        assert_eq!(amount(GOLD).await?, Some(100));
        for track in [MORTAR, FRAZZLEMAWS, SILENCERS] {
            assert_eq!(amount(track).await?, Some(0));
        }
        client.talk(npc, "trade").await?;
        assert_eq!(
            client.talk(npc, "2").await?.lines,
            ["Your balance cannot cover that quantity."],
        );
        assert_eq!(amount(GOLD).await?, Some(100));
        assert_eq!(clusters().await?, 0);
        let command = admitted.continuity.next_command_id + 7;
        assert_eq!(client.talk(npc, "1").await?.lines, ["Clusters delivered."]);
        assert_eq!(amount(GOLD).await?, Some(0));
        assert_eq!(clusters().await?, 1);
        assert_eq!(
            client.talk_command(npc, "1", command).await?.lines,
            ["Clusters delivered."],
        );
        assert_eq!(amount(GOLD).await?, Some(0));
        assert_eq!(clusters().await?, 1);
        client.close().await
    })
    .await?;
    Ok(())
}
