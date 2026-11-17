//! Lane qh: registered authored bindings driven by literal USE on an unmodified native map crop.
//! The quest catalogue is the production embedded one; nothing here supplies a callback.
use super::fixture::{Harness, id};
use super::questpc8_authored_programmes::use_target;
use super::{TestResult, quest_world_callbacks};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const SMALL_SAPPHIRE: &str = "authored-qh-small-sapphire";
const SMALL_SAPPHIRE_CHEST: TilePos = TilePos {
    x: 32962,
    y: 32429,
    floor: -7,
};

#[test]
fn qh_small_sapphire_premium_chest_mints_bag_once_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("qh-small-sapphire", || {
        // The scenario starts one tile west of this position, beside the chest.
        quest_world_callbacks::scenario(
            SMALL_SAPPHIRE,
            Position {
                x: 32963,
                y: 32430,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qh_small_sapphire_premium_chest_mints_bag_once_postgres duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn is_case(name: &str) -> bool {
    name.starts_with("authored-qh-")
}

/// The registered production binding, parsed from the same committed document the server embeds.
pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let document = match name {
        SMALL_SAPPHIRE => include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/quests/authored-bindings/small_sapphire_quest.json"
        )),
        _ => return Err(format!("unknown qh case {name}").into()),
    };
    Bindings::parse(document.as_bytes())?
        .world_callbacks()
        .next()
        .cloned()
        .ok_or_else(|| "qh binding has no world callback".into())
}

pub(super) fn world(
    name: &str,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
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
        _ => Err(format!("unknown qh case {name}").into()),
    }
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
