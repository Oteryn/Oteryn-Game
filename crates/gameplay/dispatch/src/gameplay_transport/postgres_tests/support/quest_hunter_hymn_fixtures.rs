use super::fixture::{debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) const PIANO: Position = Position {
    x: 32406,
    y: 32794,
    floor: -9,
};

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        world_id,
        "content-1",
    )?;
    let native = controller
        .active()
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native Item provider")?;
    use oteryn_gameplay_domain::domain::loot_data::LootDefinitionRef;
    for (creature, sheet) in [
        ("novice_of_the_cult", 6087),
        ("acolyte_of_the_cult", 6088),
        ("adept_of_the_cult", 6089),
        ("enlightened_of_the_cult", 6090),
    ] {
        let reward = native
            .reward_table()
            .row(&format!("oteryn:creature.{creature}"))
            .map_err(debug)?;
        let item = format!("oteryn:item.tibia.i{sheet}");
        let sheet = LootDefinitionRef::new("Item", &item, "definition-r1");
        assert!(
            reward
                .loot_table
                .entries
                .iter()
                .any(|entry| entry.item == sheet)
        );
    }
    // The return teleport between the piano and landing is a separate map action.
    super::questpc8_native_slice::world(
        native,
        &[
            (32402, 32794, 32402, 32794, -9),
            (32406, 32794, 32407, 32794, -9),
        ],
        &[],
        TilePos {
            x: PIANO.x + 1,
            y: PIANO.y,
            floor: PIANO.floor,
        },
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &super::fixture::Harness,
) -> TestResult {
    use super::questpc8_authored_programmes_fixtures::{object_target, use_target};

    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("native Item capabilities")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    for (item, count) in [(6087_u16, 1_u16), (6088, 1), (6089, 1), (6090, 2)] {
        for copy in 0..count {
            super::quest_world_callbacks_fixtures::input_item(
                owner,
                fence.item_fence(),
                &Offering {
                    position: PIANO,
                    item: format!("oteryn:item.tibia.i{item}"),
                    count: 1,
                },
                u64::from(item) * 2 + u64::from(copy),
                true,
            )
            .await?;
        }
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let tile = TilePos {
        x: PIANO.x,
        y: PIANO.y,
        floor: PIANO.floor,
    };
    for (item, expected) in [(6090, 1_i64), (6088, 0), (6087, 0), (6089, 0)] {
        let target = object_target(owner, tile, 1)?;
        let command = admitted.continuity.next_command_id;
        let result = use_target(owner, admitted, command, &target).await?;
        assert_eq!(
            result,
            UseDisposition::Rejected,
            "the piano requires every verse"
        );
        super::qengine_voucher_stories_fixtures::use_native(
            owner,
            admitted,
            item,
            UseDisposition::Committed,
        )
        .await?;
        assert_eq!(quantity(h, item).await?, expected);
    }
    super::qengine_voucher_stories_fixtures::use_native(
        owner,
        admitted,
        6090,
        UseDisposition::Rejected,
    )
    .await?;
    assert_eq!(
        quantity(h, 6090).await?,
        1,
        "a known verse does not burn another sheet"
    );

    let target = object_target(owner, tile, 1)?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!((position.x, position.y, position.floor), (32402, 32794, 9));
    Ok(())
}

async fn quantity(h: &super::fixture::Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(format!("oteryn:item.tibia.i{item}"))
        .fetch_one(&h.pool).await?)
}
