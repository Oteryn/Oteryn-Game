//! Visibility reads the same authoritative vitals for Channel and native House players.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e::scenario_thread::run_async as run;
use super::{Dependencies, TestResult};
use crate::foundation::{
    CharacterId, CharacterLease, GameSessionId, LocalPosition, RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
};
use crate::gameplay_transport::actor_spell::tests::FACTS;
use crate::spell::cast::PlayerSpellState;
use oteryn_gameplay_entities::carrier::house_custody::HouseActorState;

#[test]
fn visibility_reads_injured_player_health_in_channel_and_house_postgres() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "visibility_player_health", false, false)
                .await?;
        let result: TestResult = async {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            {
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                let mut states = owner.states.spell_states.lock().await;
                let state = states
                    .get_mut(&runtime, deps.actor, deps.admitted.game_session_id)
                    .ok_or("channel state")?;
                state.restore_saved_vitals(37, state.vitals().mana);
            }
            let view = owner
                .observe_channel_entities(deps.actor)
                .await
                .ok_or("channel view")?;
            assert_eq!(
                view.entities
                    .iter()
                    .find(|e| e.identity == deps.actor.placement_identity())
                    .ok_or("player")?
                    .health_percent,
                20
            );
            let session = GameSessionId::decode(&id(71)).map_err(debug)?;
            let house_actor = {
                let mut runtime = owner.runtime.lock().await;
                let lease = CharacterLease::new(CharacterId::decode(&id(72)).map_err(debug)?, 1)
                    .map_err(debug)?;
                let position = LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                };
                let house = HouseActorState {
                    house: "house:health".into(),
                    scope: RuntimeScopeRefV1::Instance {
                        world_id: owner.world_id,
                        instance_id: id(73),
                    },
                    generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
                    position,
                    lease,
                    origin_position: position,
                };
                let actor = runtime
                    .restore_transfer_actor(
                        GameSessionId::decode(&id(74)).map_err(debug)?,
                        session,
                        lease,
                        position,
                        Some(house),
                    )
                    .map_err(debug)?;
                let mut state = PlayerSpellState::new(FACTS, 0, 0).ok_or("house spell state")?;
                state.restore_saved_vitals(37, state.vitals().mana);
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .actors
                    .push((actor, session, state));
                actor
            };
            let view = owner
                .observe_channel_entities(house_actor)
                .await
                .ok_or("house view")?;
            assert_eq!(
                view.entities
                    .iter()
                    .find(|e| e.identity == house_actor.placement_identity())
                    .ok_or("resident")?
                    .health_percent,
                20
            );
            Ok(())
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
