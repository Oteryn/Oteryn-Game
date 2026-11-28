//! Outdoor bars use the wire's base-object target and the Channel map owner.

use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{CharacterId, CharacterLease, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, placement_key};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use std::time::Duration;

#[test]
fn shipped_wooden_bars_2292_break_and_restore() -> TestResult {
    cut_shipped_bar((32809, 32580), (32809, 32581), 2292, 3146)
}

#[test]
fn shipped_wooden_bars_2295_break_and_restore() -> TestResult {
    cut_shipped_bar((33124, 32151), (33123, 32151), 2295, 3145)
}

#[test]
fn shipped_wooden_bars_2296_break_and_restore() -> TestResult {
    cut_shipped_bar((33124, 32151), (33124, 32150), 2296, 3145)
}

fn cut_shipped_bar((start_x, start_y): (u16, u16), (x, y): (u16, u16), appearance: u16, debris: u16) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "wooden_bars", true, false).await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_DATABASE_READY wooden_bars_{appearance} version={version}");
        super::reset();
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let start = TilePos {
            x: start_x,
            y: start_y,
            floor: -8,
        };
        let target = TilePos {
            x,
            y,
            floor: start.floor,
        };
        let boot = || super::source_world_crop::world(
            TilePos { x: x.min(start_x), y: y.min(start_y), ..start },
            TilePos { x: x.max(start_x), y: y.max(start_y), ..start },
            start,
            native,
        );
        let map = boot()?;
        super::arena_match_fixtures::fixture::runtime_at(
            &mut deps,
            active,
            &pin,
            &map,
            MovementLocalPosition {
                x: i32::from(start.x),
                y: i32::from(start.y),
                floor: 8,
            },
        )?;
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;

        let sword = "oteryn:item.tibia.i3264";
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        super::item_look_fixtures::seed_main_backpack(&h, &[(sword, 1)]).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(map.frame_binding_digest(deps.admitted.world_id).as_slice())
            .execute(&h.pool).await?;

        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("use and map capabilities")?;
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                CharacterLease::new(
                    CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;

        let key = placement_key(target, 1).ok_or("bar placement")?;
        let id = *map
            .base()
            .tile(x, y, target.floor)
            .ok_or("bar tile")?
            .ids()
            .get(1)
            .ok_or("bar item")?;
        let current = || {
            map.facts()
                .base_entry(target, 1, id)
                .map(|entry| entry.appearance_id)
        };
        let spell_blocked = || {
            cells.spell_tiles().lookup(
                cells.scope(),
                crate::content::LogicalCell {
                    x: i32::from(x),
                    y: i32::from(y),
                    z: -i32::from(target.floor),
                },
            ).map(|tile| tile.flags().block_solid).map_err(debug)
        };
        assert_eq!(current(), Some(appearance));
        assert!(!map.enterable(target));
        assert!(spell_blocked()?);
        let mut placement = map.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: 0,
            })),
            use_with_position: None,
        })
        .map_err(debug)?;
        let sword_ref = native
            .item_index()
            .and_then(|index| index.definition_ref("Item", sword, "definition-r1"))
            .ok_or("sword definition")?;
        // Advance the owner clock instead of sleeping between weapon rolls.
        let mut last_attempt_started = 0;
        for _ in 0..80 {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_millis(200))
                .ok_or("clock")?;
            let command = admitted.continuity.next_command_id;
            last_attempt_started = owner.owner_now().get();
            let (next, result, _) = super::item_craft_fixtures::exchange(
                &owner,
                admitted,
                command,
                &payload,
                Some(sword_ref),
            )
            .await?;
            assert_eq!(
                result,
                UseDisposition::Committed,
                "live sword USE_WITH on bar {appearance}"
            );
            admitted = next;
            if map.facts().object_revision(key) != 0 {
                break;
            }
        }
        assert_eq!(current(), Some(debris));
        assert_eq!(map.facts().object_revision(key), 1);
        assert!(map.enterable(target));
        assert!(!spell_blocked()?, "spell endpoint should open with the bar");
        let due = map
            .facts()
            .next_movement_decay_time()
            .ok_or("bar decay deadline")?;
        let decay = 120_000_000;
        assert!((last_attempt_started + decay..=owner.owner_now().get() + decay).contains(&due));
        let command = admitted.continuity.next_command_id;
        let (_, stale, _) = super::item_craft_fixtures::exchange(
            &owner, admitted, command, &payload, Some(sword_ref),
        ).await?;
        assert_eq!(stale, UseDisposition::StaleState);
        assert_eq!(map.facts().object_revision(key), 1);
        assert_eq!(map.facts().expire_movement_items(due - 1), 0);
        assert!(map.enterable(target));
        let restarted = boot()?;
        assert_eq!(
            restarted
                .facts()
                .base_entry(target, 1, id)
                .map(|entry| entry.appearance_id),
            Some(appearance)
        );
        assert_eq!(restarted.facts().next_movement_decay_time(), None);
        assert!(!restarted.enterable(target));
        assert_eq!(map.facts().expire_movement_items(due), 1);
        assert_eq!(current(), Some(appearance));
        assert_eq!(map.facts().object_revision(key), 2);
        assert!(!map.enterable(target));
        assert!(spell_blocked()?);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
