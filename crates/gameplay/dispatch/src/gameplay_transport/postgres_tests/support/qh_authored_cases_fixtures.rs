//! Lane qh: registered authored bindings driven by literal USE on an unmodified native map crop.
//! The quest catalogue is the production embedded one; nothing here supplies a callback.
use super::fixture::{Harness, debug, id};
use super::questpc8_authored_programmes_fixtures::use_target;
use super::{TestResult, quest_world_callbacks_fixtures};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) const SMALL_SAPPHIRE: &str = "authored-qh-small-sapphire";
const SMALL_SAPPHIRE_CHEST: TilePos = TilePos {
    x: 32962,
    y: 32429,
    floor: -7,
};
pub(super) const TOP_OF_THE_CITY: &str = "authored-qh-top-of-the-city";
const TOP_OF_THE_CITY_CRATE: TilePos = TilePos {
    x: 32807,
    y: 31252,
    floor: -8,
};

pub(super) const DIG: &str = "authored-qh-top-of-the-city-dig";
pub(super) const BARE: &str = "authored-qh-top-of-the-city-dig-bare";
pub(super) const WRONG_TOOL: &str = "authored-qh-top-of-the-city-dig-wrong-tool";
pub(super) const NO_PICK: &str = "authored-qh-top-of-the-city-dig-no-pick";
pub(super) const DIRT: Position = Position {
    x: 32803,
    y: 31259,
    floor: -8,
};
const BEFORE_DIG: Position = Position {
    x: 32804,
    y: 31258,
    floor: -8,
};
const TUNNEL: Position = Position { floor: -9, ..DIRT };
const DIG_CALLBACK: &str = "oteryn:quest-callback/authored-top-of-the-city-dig";
const CITY_STAGE: &str = "oteryn:quest-progress/authored/top-of-the-city/stage";
const PICK: &str = "oteryn:item.tibia.i3456";
const OBSIDIAN_KNIFE: &str = "oteryn:item.tibia.i5908";

pub(super) fn is_city(name: &str) -> bool {
    name == TOP_OF_THE_CITY || is_dig(name)
}

pub(super) fn is_dig(name: &str) -> bool {
    matches!(name, DIG | BARE | WRONG_TOOL | NO_PICK)
}

pub(super) const fn dig_start() -> Position {
    BEFORE_DIG
}

pub(super) fn is_case(name: &str) -> bool {
    name.starts_with("authored-qh-")
}

/// The registered production binding, parsed from the same committed document the server embeds.
pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let (document, key) = match name {
        SMALL_SAPPHIRE => (
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/quests/authored-bindings/small_sapphire_quest.json"
            )),
            "oteryn:quest-callback/authored-small-sapphire-chest",
        ),
        TOP_OF_THE_CITY | DIG | BARE | WRONG_TOOL | NO_PICK => (
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/quests/authored-bindings/top_of_the_city_quest.json"
            )),
            if is_dig(name) {
                DIG_CALLBACK
            } else {
                "oteryn:quest-callback/authored-top-of-the-city-crate"
            },
        ),
        _ => return Err(format!("unknown qh case {name}").into()),
    };
    Bindings::parse(document.as_bytes())?
        .world_callbacks()
        .find(|callback| callback.key == key)
        .cloned()
        .ok_or_else(|| "qh binding has no world callback".into())
}

pub(super) fn world(
    name: &str,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    if is_dig(name) {
        return super::source_world_crop::compiled_region(
            TilePos {
                x: DIRT.x - 1,
                y: DIRT.y - 1,
                floor: TUNNEL.floor,
            },
            TilePos {
                x: DIRT.x + 1,
                y: DIRT.y + 1,
                floor: DIRT.floor,
            },
            tile(BEFORE_DIG),
            native,
        );
    }
    let (minimum, maximum, start) = match name {
        SMALL_SAPPHIRE => (
            TilePos {
                x: 32958,
                y: 32426,
                floor: -7,
            },
            TilePos {
                x: 32966,
                y: 32433,
                floor: -7,
            },
            TilePos {
                x: 32962,
                y: 32430,
                floor: -7,
            },
        ),
        TOP_OF_THE_CITY => (
            TilePos {
                x: 32806,
                y: 31249,
                floor: -8,
            },
            TilePos {
                x: 32813,
                y: 31256,
                floor: -8,
            },
            TilePos {
                x: 32808,
                y: 31252,
                floor: -8,
            },
        ),
        _ => return Err(format!("unknown qh case {name}").into()),
    };
    super::source_world_crop::world(minimum, maximum, start, native)
}

pub(super) async fn exercise(
    name: &str,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    match name {
        SMALL_SAPPHIRE => small_sapphire(owner, admitted, h).await,
        TOP_OF_THE_CITY => top_of_the_city(owner, admitted, h).await,
        DIG | BARE | WRONG_TOOL | NO_PICK => dig(name, owner, admitted, h).await,
        _ => Err(format!("unknown qh case {name}").into()),
    }
}

const fn tile(position: Position) -> TilePos {
    TilePos {
        x: position.x,
        y: position.y,
        floor: position.floor,
    }
}

async fn dig(
    name: &str,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let npcs = prepare_city(owner, admitted).await?;
    let mut sequence = 2000_u64;
    start_city(owner, admitted, h, npcs, &mut sequence).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "dig fence")?
        .ok_or("dig fence")?
        .item_fence();
    for (index, item) in [(1, PICK), (2, OBSIDIAN_KNIFE)] {
        if (item == PICK && name != NO_PICK) || (item == OBSIDIAN_KNIFE && name == WRONG_TOOL) {
            quest_world_callbacks_fixtures::input_item(
                owner,
                fence,
                &Offering {
                    position: BEFORE_DIG,
                    item: item.into(),
                    count: 1,
                },
                950 + index,
                true,
            )
            .await?;
        }
    }
    let dirt = target(owner, tile(DIRT))?;
    let map = owner.bundle_map().ok_or("dig map")?;
    assert_eq!(
        super::engitem_seal_paint_fixtures::source_appearances(map, DIRT)?,
        [351, 8504]
    );
    assert_eq!(live(h, 3456).await?, i64::from(name != NO_PICK));

    let command = admitted.continuity.next_command_id;
    let result = if name == BARE {
        use_target(owner, admitted, command, &dirt).await?
    } else {
        let tool = if name == WRONG_TOOL {
            OBSIDIAN_KNIFE
        } else {
            PICK
        };
        let reference = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|n| n.item_index())
            .and_then(|index| index.definition_ref("Item", tool, "definition-r1"))
            .ok_or("dig tool definition")?;
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(reference),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(dirt)),
            use_with_position: None,
        })
        .map_err(debug)?;
        let response = quest_world_callbacks_fixtures::wire(owner, admitted, command, 2, payload).await?;
        decode_use_result(&response).map_err(debug)?
    };
    let expected = match name {
        DIG => UseDisposition::Committed,
        BARE => UseDisposition::Rejected,
        _ => UseDisposition::NothingToUse,
    };
    assert_eq!(result, expected, "{name} dispatch disposition");
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("dig actor")?)
        .map_err(debug)?
        .position();
    let expected_position = if name == DIG { TUNNEL } else { BEFORE_DIG };
    assert_eq!(
        position,
        crate::foundation::MovementLocalPosition {
            x: i32::from(expected_position.x),
            y: i32::from(expected_position.y),
            floor: -i16::from(expected_position.floor),
        }
    );
    assert_eq!(track(h, CITY_STAGE).await?, 2);
    let selected_tools: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_world_tool_reads r \
         JOIN game_item_instances i USING(item_instance_id) \
         WHERE r.transition_key=$1 AND i.definition_production_key=$2",
    )
    .bind(DIG_CALLBACK)
    .bind(PICK)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(selected_tools, i64::from(name == DIG), "fenced pick read");
    assert_eq!(live(h, 3456).await?, i64::from(name != NO_PICK));
    Ok(())
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    at: TilePos,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("map")?;
    let key = crate::map::view::placement_key(at, 1).ok_or("placement key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn live(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances \
         WHERE definition_production_key=$1 AND lifecycle=1",
    )
    .bind(format!("oteryn:item.tibia.i{item}"))
    .fetch_one(&h.pool)
    .await?)
}

async fn track(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE((SELECT value FROM game_character_quest_tracks \
         WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)",
    )
    .bind(id(41).as_slice())
    .bind(key)
    .fetch_one(&h.pool)
    .await?)
}

/// TibiaWiki curid 77574 (Small Sapphire Quest/Spoiler): premium, one chest on Drefia island.
async fn small_sapphire(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    const CHEST: &str = "oteryn:quest-progress/authored/small-sapphire/chest";
    let chest = target(owner, SMALL_SAPPHIRE_CHEST)?;
    let premium = owner.premium_coordinator.take();
    assert_eq!(
        use_target(owner, admitted, 1, &chest).await?,
        UseDisposition::Rejected,
        "free account cannot claim the premium chest"
    );
    assert_eq!(track(h, CHEST).await?, -1);
    assert_eq!(live(h, 3029).await?, 0);
    owner.premium_coordinator = premium;
    owner.admit_premium(id(40), admitted.game_session_id);
    assert_eq!(
        use_target(owner, admitted, 2, &chest).await?,
        UseDisposition::Committed
    );
    assert_eq!(track(h, CHEST).await?, 1);
    for (item, count) in [(2853, 1), (3029, 1), (3031, 50), (2822, 1), (637, 1)] {
        assert_eq!(live(h, item).await?, count, "reward item {item}");
    }
    let nested: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_nested_entries e \
         JOIN game_item_instances c ON c.item_instance_id=e.parent_item_instance_id \
         WHERE c.definition_production_key='oteryn:item.tibia.i2853' AND c.lifecycle=1",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(nested, 4, "all four rewards sit inside the one bag");
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        use_target(owner, admitted, 2, &chest).await?,
        UseDisposition::Committed,
        "replay of the same command returns the recorded result"
    );
    assert_eq!(
        use_target(owner, admitted, 3, &target(owner, SMALL_SAPPHIRE_CHEST)?).await?,
        UseDisposition::Rejected,
        "the chest is claimed once"
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(after, before, "no second reward is minted");
    Ok(())
}

async fn prepare_city(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<[EntityRef; 2]> {
    owner.admit_premium(id(40), admitted.game_session_id);
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let npcs =
        super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.iriana", "oteryn:npc.jimmy"])
            .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(npcs)
}

async fn start_city(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    [iriana, jimmy]: [EntityRef; 2],
    sequence: &mut u64,
) -> TestResult {
    for (npc, word, stage) in [(iriana, "quest", 1), (jimmy, "iriana", 2)] {
        assert_eq!(
            super::questpc7_outfit_items_fixtures::talk(owner, admitted, npc, sequence, word).await?,
            NpcIntentDisposition::Ok,
            "{word}"
        );
        assert_eq!(track(h, CITY_STAGE).await?, stage, "{word}");
    }
    Ok(())
}

/// TibiaWiki curid 31073 oldid 1050120 (Top of the City Quest/Spoiler): Iriana sends the player
/// to Jimmy, who wants a food crate from the tavern cellar in exchange for Iriana's Chest.
/// The roof route, the sewer dig and the cross-floor rope are walked outside this one-floor crop.
async fn top_of_the_city(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [iriana, jimmy] = prepare_city(owner, admitted).await?;
    let mut sequence = 2000_u64;
    let crate_target = target(owner, TOP_OF_THE_CITY_CRATE)?;
    assert_eq!(
        use_target(owner, admitted, 1, &crate_target).await?,
        UseDisposition::Rejected,
        "the cellar crate waits for Jimmy's mission"
    );
    assert_eq!(live(h, 8772).await?, 0);
    start_city(owner, admitted, h, [iriana, jimmy], &mut sequence).await?;
    assert_eq!(
        use_target(owner, admitted, 2, &target(owner, TOP_OF_THE_CITY_CRATE)?).await?,
        UseDisposition::Committed
    );
    assert_eq!(track(h, CITY_STAGE).await?, 3);
    assert_eq!(live(h, 8772).await?, 1, "one food crate");
    assert_eq!(
        use_target(owner, admitted, 3, &target(owner, TOP_OF_THE_CITY_CRATE)?).await?,
        UseDisposition::Rejected,
        "the crate is taken once"
    );
    assert_eq!(live(h, 8772).await?, 1);
    assert_eq!(
        super::questpc7_outfit_items_fixtures::talk(owner, admitted, jimmy, &mut sequence, "quest").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(track(h, CITY_STAGE).await?, 4);
    assert_eq!((live(h, 8772).await?, live(h, 8773).await?), (0, 1));
    assert_eq!(
        super::questpc7_outfit_items_fixtures::talk(owner, admitted, iriana, &mut sequence, "quest").await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(track(h, CITY_STAGE).await?, 5);
    assert_eq!((live(h, 8773).await?, live(h, 5921).await?), (0, 10));
    let completed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind("oteryn:quest.authored.top_of_the_city_quest")
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(completed, 1, "the quest completes once");
    super::questpc7_outfit_items_fixtures::talk(owner, admitted, iriana, &mut sequence, "quest").await?;
    assert_eq!(live(h, 5921).await?, 10, "no second reward");
    Ok(())
}
