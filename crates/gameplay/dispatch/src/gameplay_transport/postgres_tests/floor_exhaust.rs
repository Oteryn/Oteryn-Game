//! FLOOR-EXHAUST-1: actual session, paced STEP and spell dispatch with routed PostgreSQL.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::qualify_native_entry_room;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::boot::tests::{floor_bundle, floor_items, floor_pins};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_spell_rules::cast::STAIRHOP_COOLDOWN_KEY;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use std::num::NonZeroU32;

#[test]
fn floor_exhaust_postgres_real_session_step_then_aggressive_cast() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "floor_exhaust", true, false).await?;
        eprintln!("POSTGRES_STARTED FLOOR-EXHAUST-1");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let initial = Dependencies::new()?;
        let world = initial.admitted.world_id;
        let channel = initial.admitted.channel_id;
        let room = qualify_native_entry_room(world).map_err(debug)?;
        let (bytes, load) = floor_bundle();
        let bundle = crate::map::boot::boot(&bytes, &floor_pins(load), world, channel, floor_items)
            .map_err(debug)?;
        let cells = bundle
            .movement_cells(room.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            (0, 0, 8),
        );
        let spell = oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
            &serde_json::from_str(include_str!("../../../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-energy_strike/spell.json"))?,
            &serde_json::from_str(include_str!("../../../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-energy_strike/dependencies.json"))?,
        ).map_err(debug)?;
        assert!(spell.aggressive);
        let deps = Dependencies::with_content(
            pin,
            room,
            crate::spell::SpellBook::new(vec![spell]).map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = &cells;
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            states
                .initialize(
                    &runtime,
                    deps.actor,
                    deps.admitted.game_session_id,
                    crate::spell::cast::CharacterCastFacts {
                        level: 100,
                        magic_level: 50,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("spell state")?;
        }
        let session = deps.admitted.game_session_id;
        let intent = SpellCastIntent {
            spell: NonZeroU32::new(1).ok_or("index")?,
            target: SpellTarget::None,
            aim_at_target: false,
        };
        // This gameplay fixture has no activated combat route. Record its ordinary dispatcher
        // refusal so the post-window assertion tests the same production entry point.
        let baseline = owner
            .cast_spell_completion(deps.actor, session, 1, intent)
            .await
            .ok_or("baseline cast completion")?;
        assert_eq!(baseline.disposition, SpellCastDisposition::Rejected);
        let before = owner.observe_vitals(deps.actor, session).await;
        let (step, _) = owner
            .paced_step(deps.actor, session, StepDirection::East)
            .await;
        assert_eq!(step.disposition, StepDisposition::Moved);
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 2,
                y: 0,
                floor: 7
            }
        );
        assert_eq!(
            owner
                .cast_spell_completion(deps.actor, session, 2, intent)
                .await
                .ok_or("cast completion")?
                .disposition,
            SpellCastDisposition::CoolingDown
        );
        assert_eq!(
            owner.observe_vitals(deps.actor, session).await,
            before,
            "refusal spends nothing"
        );
        // Advance the actual owner's monotonic origin, without wall-clock sleeping or editing
        // the cooldown. The same production dispatch now returns its ordinary fixture refusal.
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        assert_eq!(
            owner
                .cast_spell_completion(deps.actor, session, 3, intent)
                .await
                .ok_or("post-window cast completion")?
                .disposition,
            baseline.disposition
        );
        // The legacy cast owner also reaches its ordinary target requirement after expiry.
        assert_eq!(
            owner
                .cast_spell(deps.actor, session, 4, intent)
                .await
                .disposition,
            SpellCastDisposition::TargetRequired
        );
        {
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .get(&runtime, deps.actor, session)
                    .ok_or("state")?
                    .cooldowns
                    .group_ready_at(STAIRHOP_COOLDOWN_KEY)
                    .is_some_and(|until| until <= owner.owner_now())
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
