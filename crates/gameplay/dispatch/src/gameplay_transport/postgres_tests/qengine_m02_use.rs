//! Nomads window through ordinary USE and the committed native scene.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::*;

pub(super) const NAME: &str = "m02/nomads-land/window";
pub(super) const WINDOW: Position = Position {
    x: 33216,
    y: 32855,
    floor: -9,
};

#[test]
fn m02_nomads_native_use_observes_both_live_stacks_postgres() -> TestResult {
    scenario(NAME)
}
#[test]
fn m02_nomads_native_use_refuses_same_counts_in_wrong_order_postgres() -> TestResult {
    scenario("m02/nomads-land/window-wrong-order")
}
fn scenario(name: &'static str) -> TestResult {
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, WINDOW, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z09-x129-y128.b3"
            )),
            region: (9, 129, 128),
            bounds: (33214, 32853, 33219, 32857),
            start: (33217, 32854),
            expected: vec![
                ExpectedTile {
                    position: (33216, 32855),
                    ids: vec![7358, 1744],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33215, 32854),
                    ids: vec![7358],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33216, 32854),
                    ids: vec![7358],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33217, 32854),
                    ids: vec![7358],
                    counted_entry: None,
                },
            ],
        },
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    wrong_order: bool,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M02_NATIVE_USE_POSTGRES {version} wrong_order={wrong_order}");
    assert_eq!(callback.key, format!("oteryn:quest-callback/{NAME}"));
    assert!(!callback.has_personal_state());
    assert_nomads_catalogue(owner);
    let before = footprint(h).await?;
    assert_eq!(
        use_window(owner, admitted, 1).await?,
        UseDisposition::Rejected,
        "empty stacks"
    );
    for (pile, x) in [33215, 33216].into_iter().enumerate() {
        let items = if wrong_order && pile == 1 {
            [2471, 2904, 2471]
        } else {
            [2471, 2471, 2904]
        };
        for (offset, item) in items.into_iter().enumerate() {
            mint(owner, h, x, item, u32::try_from(1200 + pile * 3 + offset)?).await?;
        }
        if pile == 0 {
            assert_eq!(
                use_window(owner, admitted, 2).await?,
                UseDisposition::Rejected,
                "one pile is insufficient"
            );
        }
    }
    let items_before: Vec<(String,i64)> = sqlx::query_as("SELECT item_instance_id::text, quantity FROM game_item_instances ORDER BY item_instance_id").fetch_all(&h.pool).await?;
    assert_layers(owner, admitted, callback, wrong_order).await?;
    let expected = if wrong_order {
        UseDisposition::Rejected
    } else {
        UseDisposition::Committed
    };
    assert_eq!(
        use_window(owner, admitted, 3).await?,
        expected,
        "same quantities must still obey live stack order"
    );
    assert_eq!(
        use_window(owner, admitted, 4).await?,
        expected,
        "repeat remains a physical observation"
    );
    assert_eq!(
        footprint(h).await?,
        before,
        "no fabricated personal progress or quest receipt"
    );
    assert_eq!(items_before,sqlx::query_as::<_,(String,i64)>("SELECT item_instance_id::text, quantity FROM game_item_instances ORDER BY item_instance_id").fetch_all(&h.pool).await?,"observation cannot consume or mint");
    if !wrong_order {
        mint(owner, h, 33216, 2471, 1210).await?;
        assert_eq!(
            use_window(owner, admitted, 4).await?,
            UseDisposition::Committed,
            "retained terminal wire replay precedes fresh selection after layout drift"
        );
        assert_eq!(
            use_window(owner, admitted, 5).await?,
            UseDisposition::Rejected,
            "an extra live root invalidates the previously accepted layout"
        );
        assert_eq!(footprint(h).await?, before);
    }
    Ok(())
}
pub(super) fn assert_nomads_catalogue(owner: &ComposedFreshAdmission<'_, '_, '_>) {
    let mut keys: Vec<_> = owner
        .callback_content()
        .iter()
        .filter(|callback| {
            callback.selector.native.as_ref().is_some_and(|native| {
                native.position == WINDOW && native.ordinal == 1 && native.appearance == 1744
            })
        })
        .map(|callback| callback.key.as_str())
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "oteryn:quest-callback/engenc2/nomads-land-window",
            "oteryn:quest-callback/m02/nomads-land/window",
        ],
        "both actual callback contracts must reach the ordinary selector"
    );
}
async fn footprint(h: &Harness) -> Result<(i64, i64), sqlx::Error> {
    sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_tracks),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await
}
pub(super) async fn use_window(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("native map")?;
    let key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: WINDOW.x,
            y: WINDOW.y,
            floor: WINDOW.floor,
        },
        1,
    )
    .ok_or("window key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        encode_use_intent(&target).map_err(debug)?,
    )
    .await?;
    decode_use_result(&reply).map_err(debug)
}
pub(super) async fn mint(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    x: u16,
    item: u32,
    ordinal: u32,
) -> TestResult {
    let (generation, content, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.owner_fence().map_err(debug)?.generation(),
            format!(
                "sha256:{}",
                runtime
                    .content_pin()
                    .server_artifact_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            runtime.content_pin().frame_binding_digest().to_vec(),
        )
    };
    let map_revision =
        crate::map::overlay::map_revision(owner.bundle_map().ok_or("native map")?.overlay.base());
    let mut candidate = h
        .root
        .freeze_item_mint(
            owner.character,
            &h.node,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    owner.world_id,
                    owner.channel_id,
                    generation,
                    ordinal,
                    1,
                    TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:loot.m02-physical-stacks".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    "fixture:m02.window-drop".into(),
                    0,
                ),
                item: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: format!("oteryn:item.tibia.i{item}"),
                    revision_ref: "definition-r1".into(),
                },
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            x, 32854, -9,
                        ),
                    // These are loose drops, not loot still owned by an unmaterialized corpse.
                    // Use the existing custody category admitted by the native Ground reader.
                    corpse_ref: [
                        crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,
                        super::fixture::id(3).as_slice(),
                    ]
                    .concat(),
                    map_revision,
                    content_revision: content.clone(),
                    native_room_placement_context: frame,
                },
                content_revision: content,
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("Ground MINT lane")?;
    assert!(matches!(
        h.root
            .commit_item_mint(&permit, owner.character, &h.node, &mut candidate)
            .await
            .map_err(debug)?,
        ItemMintOutcome::Committed(_)
    ));
    // Fixture MINT stands in for a committed drop; the ordinary writer owns the same invalidation.
    owner.states.loop_ground.invalidate();
    Ok(())
}

async fn assert_layers(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    callback: &Callback,
    wrong_order: bool,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let context = {
        let runtime = owner.runtime.lock().await;
        let snapshot = runtime.read_actor_position(actor).map_err(debug)?;
        crate::durability::quest_world_callback::QuestWorldContext {
            map_revision: crate::map::overlay::map_revision(
                owner.bundle_map().ok_or("map")?.overlay.base(),
            ),
            content_revision: format!(
                "sha256:{}",
                runtime
                    .content_pin()
                    .server_artifact_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
            reset_epoch: owner.reset_epoch,
            actor_placement: actor.placement_identity(),
            movement_revision: snapshot.revision(),
            party: Vec::new(),
        }
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "current fence")?
        .ok_or("current fence")?;
    let permit = owner.spell_lane_permit().await.ok_or("observer lane")?;
    let observed = owner
        .root
        .observe_world_callback_gate(
            owner.character,
            owner.holder,
            &permit,
            fence,
            context,
            callback.key.clone(),
        )
        .await
        .map_err(debug)?;
    eprintln!("M02_NATIVE_ROOT_GATE observed={observed} wrong_order={wrong_order}");
    assert_eq!(
        observed, !wrong_order,
        "independent current Root observation"
    );
    drop(permit);
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let map = owner.bundle_map().ok_or("map")?;
    let ground =
        crate::gameplay_transport::loop_move::loop_ground(owner, map.overlay.base(), Some(index))
            .await
            .ok_or("live Ground projection")?;
    for x in [33215, 33216] {
        let at = crate::map::overlay::TilePos {
            x,
            y: 32854,
            floor: -9,
        };
        let tile = ground.overlay().tile(at).ok_or("Ground cell")?;
        let mut base = Vec::new();
        crate::map::view::compose_into(map.overlay.base(), None, &map.facts, at, &mut base)
            .map_err(debug)?;
        eprintln!(
            "M02_NATIVE_LOCAL x={x} added={} base={:?} crate_revision={:?} amphora_revision={:?}",
            tile.added().len(),
            base.iter().map(|e| e.source).collect::<Vec<_>>(),
            index.revision("oteryn:item.tibia.i2471"),
            index.revision("oteryn:item.tibia.i2904")
        );
        assert_eq!(
            tile.added().len(),
            3,
            "live projection retains every durable root"
        );
    }
    Ok(())
}
