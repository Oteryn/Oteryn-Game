//! Crystal9f data-global/scripts/quests/fathers_burden/actions_corpse.lua:1-27,
//! sha256:ab1477d63aba338744198875b5746abc4c7092d90318331af3a2bad4732fa9a1.
//! Corpse rewards have no mission-start prerequisite and leave the corpse intact.
use super::fixture::{Harness, debug, run};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

fn case(name: &'static str, key: &str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    // Establish the missing-binding RED against a real migrated database too.
    run(async |admin| {
        let h = Harness::create_quest_reward(admin, "l2_world_reward_binding", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED {name} binding server_version={version}");
        let binding = oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .find(|c| c.key.ends_with(key));
        h.cleanup().await?;
        binding.ok_or("source corpse reward binding absent")?;
        Ok(())
    })?;
    super::qengine_rewards::scenario(name)
}

#[test]
fn questl2_world_corpse_scale_grants_once() -> TestResult {
    case("qengine-rewards-l2-father-scale", "l2-father-scale")
}
#[test]
fn questl2_world_corpse_sinew_grants_once() -> TestResult {
    case("qengine-rewards-l2-father-sinew", "l2-father-sinew")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let scale = name.ends_with("scale");
    let (suffix, corpse, output) = if scale {
        ("l2-father-scale", 11589, 11550)
    } else {
        ("l2-father-sinew", 11590, 11548)
    };
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with(suffix))
        .ok_or("corpse source")?;
    let track = &callback.storage[0].track;
    let stored: Option<i64> =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(track)
            .fetch_optional(&h.pool)
            .await?;
    assert_eq!(stored, None);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_tracks")
            .fetch_one(&h.pool)
            .await?,
        0,
        "no mission or corpse progress is seeded"
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(-15, 33294, 31677, 1).ok_or("corpse key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let result =
        super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(
        super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone()).await?,
        result
    );
    let quantity: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(format!("oteryn:item.tibia.i{output}")).fetch_one(&h.pool).await?;
    assert_eq!(quantity, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT value FROM game_character_quest_tracks WHERE track_key=$1"
        )
        .bind(track)
        .fetch_one(&h.pool)
        .await?,
        1
    );
    let map = owner.bundle_map().ok_or("map")?;
    let refreshed = WorldObjectTarget {
        placement: target.placement.clone(),
        expected_revision: map.facts.object_revision(key),
    };
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&refreshed).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let result = super::quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        UseDisposition::Rejected
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_quest_reward_sources")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(format!("oteryn:item.tibia.i{output}")).fetch_one(&h.pool).await?,1);
    let map = owner.bundle_map().ok_or("map")?;
    let tile = crate::map::view::compose_tile(
        &map.overlay,
        &map.facts,
        crate::map::overlay::TilePos {
            x: 33294,
            y: 31677,
            floor: -15,
        },
    )
    .map_err(debug)?
    .ok_or("corpse tile")?;
    assert!(
        tile.entries.iter().any(|e| e.facts.appearance_id == corpse),
        "source corpse remains after acquisition"
    );
    Ok(())
}
