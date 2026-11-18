//! Two independent characters use one Channel poison counter and one native corpse.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use oteryn_gameplay_quests::quest::{
    world_callbacks::{Callback, Offering, Position},
    wrath_keeper::{CORPSE, CREATURE, POISON_USE, SPAWN, TAIL_HANDOFF},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};

pub(super) const NAME: &str = "qengine-rewards-wrath-keeper";
pub(super) const PLANT: Position = Position {
    y: SPAWN.y - 1,
    ..SPAWN
};
const FIRST: Position = Position {
    x: PLANT.x - 1,
    ..PLANT
};
const SECOND: Position = Position {
    x: PLANT.x + 1,
    ..PLANT
};

#[test]
fn wrathm1_keeper_shared_poison_and_two_character_corpse_handoff() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, PLANT, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == POISON_USE)
        .ok_or_else(|| "registered plant-poison callback".into())
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    let mut geometry = callback.clone();
    // Crystal items.xml:28302 names the corrupted plantation appearance.
    // This test viewport places the script's ActionID adjacent to its spawn.
    geometry.selector.appearance = Some(11373);
    super::quest_world_callbacks::world_with_cells(&geometry, PLANT, &[FIRST, SECOND, SPAWN])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    deps: &Dependencies,
    h: &Harness,
) -> TestResult {
    use super::quest_world_callbacks::{input_item, local, wire};
    let (_, mut second) =
        super::grizzly_boss::vacancy::second_player(owner, deps, h, local(SECOND)).await?;
    let caps = SelectedCapabilities::select(owner.offered_capabilities(), &[4, 11, 15])
        .ok_or("Keeper USE capabilities")?;
    admitted.continuity.selected_capabilities = caps.clone();
    second.continuity.selected_capabilities = caps;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    let definition = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i11364", "definition-r1"))
        .ok_or("plant poison definition")?;
    let offering = Offering {
        position: FIRST,
        item: "oteryn:item.tibia.i11364".into(),
        count: 1,
    };
    input_item(
        owner,
        admitted.item_fence.ok_or("item fence")?,
        &offering,
        71,
        true,
    )
    .await?;
    let map = owner.bundle_map().ok_or("Keeper test map")?;
    let key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: PLANT.x,
            y: PLANT.y,
            floor: PLANT.floor,
        },
        1,
    )
    .ok_or("plant placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let use_poison = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    for count in 1..=6 {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let result = wire(
            owner,
            admitted,
            admitted.continuity.next_command_id,
            2,
            use_poison.clone(),
        )
        .await?;
        assert_eq!(
            decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed,
            "poison use {count}"
        );
        assert_eq!(
            super::questpc7_outfit_items::quantity(h, 11364).await?,
            if count < 6 { 1 } else { 0 }
        );
    }
    let keeper = owner
        .runtime
        .lock()
        .await
        .positioned_melee_census()
        .map_err(debug)?
        .into_iter()
        .find(|(_, at, session)| session.is_none() && at.position() == local(SPAWN))
        .map(|(actor, _, _)| actor)
        .ok_or("the sixth use spawns the Keeper")?;
    super::questpc7_native_deaths::lethal_creature(owner, admitted, keeper, 7400, CREATURE).await?;
    let corpse: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT i.item_instance_id FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id)
         WHERE i.definition_production_key=$1 AND i.lifecycle=1 AND octet_length(g.corpse_ref)>0"
    ).bind(CORPSE).fetch_one(&h.pool).await?;
    for player in [&mut *admitted, &mut second] {
        claim_tail(owner, player, h, *corpse.as_bytes()).await?;
    }
    let grants: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT character_id) FROM game_character_quest_receipts WHERE transition_key=$1"
    ).bind(TAIL_HANDOFF).fetch_one(&h.pool).await?;
    assert_eq!(grants, 2);
    assert_eq!(super::questpc7_outfit_items::quantity(h, 11367).await?, 2);
    Ok(())
}

async fn claim_tail(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: &mut AdmittedSession,
    h: &Harness,
    corpse: [u8; 16],
) -> TestResult {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use crate::gameplay_transport::item_view::ItemKey;
    use oteryn_protocol_oteryn::item_view::ItemHandle;
    let fence = owner
        .current_quest_fence(player.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks
         (character_id,track_key,quest_key,value,committed_character_revision)
         SELECT character_id,$2,$3,7,character_revision FROM game_character_roots
         WHERE character_id=$1 ON CONFLICT (character_id,track_key) DO UPDATE
         SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision",
    )
    .bind(sqlx::types::Uuid::from_bytes(*fence.character_id.as_bytes()))
    .bind(oteryn_gameplay_quests::quest::wrath_sleeping_dragon::QUESTLINE)
    .bind(oteryn_gameplay_quests::quest::wrath_sleeping_dragon::QUEST)
    .execute(&h.pool).await?;
    owner
        .refresh_quest_session(player.game_session_id, false)
        .await;
    let actor = player.runtime_actor.ok_or("player actor")?;
    let outcome = owner
        .use_quest_tool(
            actor,
            UseCommand {
                game_session_id: player.game_session_id,
                command_id: 7500,
                item_fence: Some(fence.item_fence()),
            },
            UseIntent {
                target: UseTarget::Item(ItemHandle::new(1).ok_or("corpse handle")?),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            },
            Some(ItemKey::Instance(corpse)),
            None,
        )
        .await
        .map_err(|()| "corpse USE")?
        .ok_or("corpse route")?;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    Ok(())
}
