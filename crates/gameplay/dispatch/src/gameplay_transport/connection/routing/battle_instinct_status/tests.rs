use super::*;
use crate::foundation::{ChannelId, WorldId};
use crate::gameplay_transport::actor_spell::{
    ChannelSpellStates,
    tests::{FACTS, runtime_with_capacity},
};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity,
};
use oteryn_protocol_oteryn::battle_instinct_status::{
    BattleInstinctStatus, COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY,
};
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::cell::Cell;
use std::error::Error;

struct Authority {
    runtime: crate::foundation::ChannelRuntimeV1,
    states: ChannelSpellStates,
    calls: Cell<usize>,
    now: Cell<u64>,
}

impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn battle_instinct_status_query(
        &self,
        admitted: AdmittedSession,
        _: u64,
    ) -> Option<Vec<u8>> {
        self.calls.set(self.calls.get() + 1);
        if !crate::gameplay_transport::battle_instinct_status::actor_is_current(
            &self.runtime,
            admitted,
        ) {
            return None;
        }
        crate::gameplay_transport::battle_instinct_status::project(
            &self.runtime,
            &self.states,
            admitted.runtime_actor?,
            admitted.game_session_id,
            self.now.get(),
        )?
        .encode()
    }
}

#[test]
fn battle_instinct_route_reads_the_current_actor_and_rejects_stale_and_unselected_sessions()
-> Result<(), Box<dyn Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Box<dyn Error + Send + Sync>> {
            tokio::runtime::Builder::new_current_thread()
                .build()?
                .block_on(async {
                    let (mut runtime, actor, session) = runtime_with_capacity(0x72, 10);
                    runtime.initialize_pinned_test_position(
                        actor,
                        crate::foundation::MovementLocalPosition {
                            x: 10,
                            y: 10,
                            floor: 7,
                        },
                    )?;
                    use oteryn_gameplay_content::content::creature_policy::{
                        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
                    };
                    let content_digest = runtime.content_pin().server_artifact_digest();
                    runtime.install_companion_policies(
                        CompiledCreaturePolicies::from_active_artifact(
                            content_digest,
                            vec![CompiledCreaturePolicy {
                                definition_key: "fixture:battle-instinct-monster".into(),
                                definition_revision: "fixture:1".into(),
                                display_name: "Battle Instinct monster".into(),
                                maximum_health: 20,
                                forge_difficulty: None,
                                base_speed: 110,
                                outfit_look_type: 21,
                                object_look_type: None,
                                summonable: false,
                                convinceable: false,
                                mana_cost: None,
                                is_familiar: false,
                                condition_immunities: vec![],
                                defense: Some(1),
                                armor: Some(1),
                                mitigation: None,
                                resistances: vec![],
                                damage_immunities: vec![],
                                healing_from_damage: vec![],
                                preferred_distance: Some(1),
                                reward_boss: Some(false),
                                blocked_by_nearby_players: false,
                                flags: CreatureFlags {
                                    attackable: true,
                                    illusionable: false,
                                    health_hidden: false,
                                },
                            }],
                        )
                        .map_err(|error| format!("{error:?}"))?,
                    )?;
                    for (x, y) in [(9, 9), (9, 10), (9, 11), (10, 9), (10, 11)] {
                        let monster = runtime.admit_pinned_test_creature(
                            crate::foundation::MovementLocalPosition { x, y, floor: 7 },
                        )?;
                        runtime
                            .install_creature_policy(monster, "fixture:battle-instinct-monster")?;
                        assert!(runtime.companion_snapshot(monster)?.state.master.is_none());
                    }
                    let character = crate::domain::CharacterId::from_bytes(*session.as_bytes())
                        .map_err(|error| format!("{error:?}"))?;
                    runtime.bind_attacker_lease(
                        actor,
                        session,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(session.as_bytes())?,
                            1,
                        )?,
                    )?;
                    let mut states = ChannelSpellStates::default();
                    states
                        .initialize(
                            &runtime,
                            actor,
                            session,
                            crate::spell::cast::CharacterCastFacts {
                                vocation: crate::spell::Vocation::Knight,
                                level: 500,
                                ..FACTS
                            },
                            (0, 0),
                            SemanticTimeMicros::from_micros(0),
                        )
                        .ok_or("initialize")?;
                    let mut authority = Authority {
                        runtime,
                        states,
                        calls: Cell::new(0),
                        now: Cell::new(0),
                    };
                    let mut admitted = AdmittedSession {
                        game_session_id: session,
                        world_id: WorldId::decode(&[
                            1, 144, 0, 0, 0, 0x60, 112, 0, 128, 0, 0, 0, 0, 0, 0, 0x60,
                        ])?,
                        channel_id: ChannelId::decode(&[
                            1, 144, 0, 0, 0, 0x61, 112, 0, 128, 0, 0, 0, 0, 0, 0, 0x61,
                        ])?,
                        runtime_actor: Some(actor),
                        first_entry: FirstEntryOutcome::Positioned,
                        controller: None,
                        continuity: SessionContinuity::FRESH,
                        item_fence: None,
                    };
                    admitted.item_fence = Some(
                        crate::durability::item_transfer::CurrentCharacterItemFence {
                            character_id: character,
                            game_session_id: session,
                            connection_generation: crate::foundation::ConnectionGeneration::new(
                                admitted.continuity.connection_generation,
                            )?,
                            character_lease_generation: 1,
                            runtime_scope: authority.runtime.binding().runtime_scope(),
                            scope_ownership_generation: authority
                                .runtime
                                .binding()
                                .scope_generation(),
                        },
                    );
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        &[OfferedCapability {
                            id: CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
                            requires: &[],
                        }],
                        &[CAPABILITY_BATTLE_INSTINCT_STATUS_V1],
                    )
                    .ok_or("selection")?;
                    let command = oteryn_protocol_oteryn::ClientCommandView {
                        command_id: 1,
                        command_type: COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY,
                        payload: &[],
                    };
                    match route(&authority, &admitted, command).await {
                        Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload) => {
                            assert_eq!(
                                BattleInstinctStatus::decode(&payload),
                                Some(BattleInstinctStatus::default())
                            )
                        }
                        _ => return Err("current actor query was refused".into()),
                    }
                    let state = authority
                        .states
                        .get_mut(&authority.runtime, actor, session)
                        .ok_or("player state")?;
                    state
                        .apply_owner_promotion_transition(true, true, Some(100))
                        .map_err(|error| format!("{error:?}"))?;
                    let mut effects =
                        oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
                    effects
                        .active_unique_perks
                        .insert("battle_instinct".to_owned());
                    state.install_wheel_effects(effects);
                    match route(&authority, &admitted, command).await {
                        Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload) => {
                            assert_eq!(
                                BattleInstinctStatus::decode(&payload),
                                Some(BattleInstinctStatus {
                                    active: true,
                                    melee: 1,
                                    shielding: 6
                                })
                            )
                        }
                        _ => return Err("eligible actor query was refused".into()),
                    }
                    authority.now.set(100);
                    match route(&authority, &admitted, command).await {
                        Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload) => {
                            assert_eq!(
                                BattleInstinctStatus::decode(&payload),
                                Some(BattleInstinctStatus::default())
                            )
                        }
                        _ => return Err("expired eligibility query was refused".into()),
                    }
                    let mut stale = admitted;
                    let mut stale_fence = stale.item_fence.ok_or("item fence")?;
                    stale_fence.character_lease_generation = 2;
                    stale.item_fence = Some(stale_fence);
                    assert!(matches!(
                        route(&authority, &stale, command).await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                    ));
                    let mut invalid = command;
                    invalid.payload = &[0];
                    assert!(matches!(
                        route(&authority, &admitted, invalid).await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                    ));
                    assert_eq!(authority.calls.get(), 4);
                    admitted.continuity.selected_capabilities = SelectedCapabilities::NONE;
                    assert!(matches!(
                        route(&authority, &admitted, command).await,
                        Dispatch::Reply(crate::foundation::CommandStatus::Rejected, _)
                    ));
                    assert_eq!(authority.calls.get(), 4);
                    assert!(
                        crate::gameplay_transport::battle_instinct_status::project(
                            &authority.runtime,
                            &authority.states,
                            actor,
                            crate::foundation::GameSessionId::decode(&[
                                1, 144, 0, 0, 0, 0x73, 112, 0, 128, 0, 0, 0, 0, 0, 0, 0x73
                            ])?,
                            0
                        )
                        .is_none()
                    );
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "Battle Instinct route worker failed")?
}
