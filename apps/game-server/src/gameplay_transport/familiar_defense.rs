//! Real source familiar self-defense: fenced owner read, physical HP+clock
//! successor, and source FX in the existing outbox, all in one locked turn.
use super::super::ComposedFreshAdmission;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ExactActorRef, GameSessionId, GameSessionState, RuntimeScopeRefV1};
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn tick_familiar_defenses(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        let Some(active) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return;
        };
        if active.familiar_defenses().is_none() {
            return;
        }
        let mut runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        if states.has_pending_spell_commit(actor, session)
            || runtime.actor_spell_reserved(actor)
            || active.source_digest() != runtime.content_pin().server_artifact_digest()
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return;
        }
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            return;
        };
        let mut context = (self, &mut *runtime, &mut *states);
        let _=pass.run_with_context(&mut context,move|holder,deadline,ctx|Box::pin(async move{
            let(owner,runtime,states)=ctx;
            let mut tx=crate::durability::spell_item_transaction::begin_spell_owner_transaction(holder,deadline).await?;
            let scope=RuntimeScopeRefV1::channel(runtime.binding().world_id(),runtime.binding().channel_id());
            let generation=runtime.binding().scope_generation().get();
            let _authority=crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(&mut tx,owner.root,owner.character,owner.holder,scope,generation).await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            let current=FreshAdmissionStore::from_root(owner.root.clone()).current_session_in_transaction(&mut tx,session).await?;
            if current.session_state()!=GameSessionState::Active||current.current_runtime_scope()!=scope||current.current_scope_generation().get()!=generation{return Err(crate::durability::DurabilityError::Unavailable)}
            let active=owner.active_generation.and_then(|g|g.native_gameplay()).ok_or(crate::durability::DurabilityError::Unavailable)?;
            let policies=active.familiar_defenses().ok_or(crate::durability::DurabilityError::Unavailable)?;
            let now=owner.owner_now().get();let unix:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&mut *tx).await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            let companions=runtime.owned_companions(actor,session).map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            for snapshot in companions {
                let Some(policy)=policies.defense(&snapshot.state.policy.definition_key,&snapshot.state.policy.definition_revision) else{continue};
                if !runtime.familiar_defense_due(snapshot.actor,policy,now,unix).unwrap_or(false){continue}
                let Some(outbox)=states.presentations.as_mut() else{continue};
                let Ok(cue)=outbox.prepare_familiar_defense_before_draw(runtime,active,policy,&snapshot) else{continue};
                let Some(prepared)=runtime.prepare_familiar_defense(snapshot.actor,policy,now,unix).map_err(|_|crate::durability::DurabilityError::Unavailable)? else{continue};
                let receipt=runtime.commit_familiar_defense(prepared).map_err(|_|crate::durability::DurabilityError::Unavailable)?;
                outbox.install_familiar_defense_preflighted(cue,&receipt);
            }
            tx.rollback().await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            Ok(())
        })).await;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use crate::content::spell_familiar_defenses::CompiledFamiliarDefenses;
    use crate::foundation::*;
    fn id(n: u8) -> [u8; 16] {
        [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
    }
    fn fixture() -> (
        ChannelRuntimeV1,
        ExactActorRef,
        GameSessionId,
        ExactActorRef,
        FamiliarSelfHealDefense,
    ) {
        let world = WorldId::decode(&id(1)).unwrap();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .unwrap();
        let session = GameSessionId::decode(&id(4)).unwrap();
        let admission = runtime.reserve_fresh_session(session).unwrap();
        let master = runtime.commit_fresh_session(admission).unwrap();
        runtime.initialize_first_entry_position(master).unwrap();
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/native-gameplay/familiar-defenses-source-vectors.json"
        ))
        .unwrap();
        let record = vectors["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                r["creature_key"] == "canary:creature/knight_familiar"
                    && r["source"]["path"]
                        .as_str()
                        .unwrap()
                        .starts_with("data-otservbr-global/")
            })
            .unwrap()
            .clone();
        // This explicit physical test fixture qualifies actual defense source
        // against matching typed Creature HP/speed; it is not SQL evidence.
        let creature = CompiledCreaturePolicy {
            definition_key: record["creature_key"].as_str().unwrap().into(),
            definition_revision: record["creature_revision"].as_str().unwrap().into(),
            display_name: "Knight Familiar".into(),
            maximum_health: record["maximum_health"].as_i64().unwrap(),
            base_speed: i32::try_from(record["base_speed"].as_i64().unwrap()).unwrap(),
            outfit_look_type: 991,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: true,
            condition_immunities: Vec::new(),
            armor: None,
            mitigation: None,
            resistances: Vec::new(),
            damage_immunities: Vec::new(),
            healing_from_damage: vec![],
            preferred_distance: None,
            reward_boss: Some(false),
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        };
        let digest = runtime.content_pin().server_artifact_digest();
        let qualified = CompiledFamiliarDefenses::from_active_artifact(
            &serde_json::to_vec(
                &serde_json::json!({"schema":vectors["schema"],"records":[record]}),
            )
            .unwrap(),
            digest,
            std::slice::from_ref(&creature),
        )
        .unwrap();
        let policy = qualified
            .defense(&creature.definition_key, &creature.definition_revision)
            .unwrap()
            .clone();
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(digest, vec![creature]).unwrap(),
            )
            .unwrap();
        let mut spawn = runtime
            .prepare_companion_spawn(
                master,
                session,
                "Knight Familiar",
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                Some(1000),
                1,
                0,
                false,
            )
            .unwrap();
        spawn.bind_semantic_creation(0).unwrap();
        let familiar = runtime.install_companion_spawn(spawn).unwrap();
        (runtime, master, session, familiar, policy)
    }
    #[test]
    fn actual_source_defense_heals_existing_hp_clamps_and_never_repeats_due() {
        let (mut runtime, _, _, familiar, policy) = fixture();
        let target_key = runtime
            .companion_snapshot(familiar)
            .unwrap()
            .state
            .policy
            .definition_key
            .clone();
        let health = runtime.companion_snapshot(familiar).unwrap().health;
        runtime
            .borrow_exact_actor_commit()
            .commit_damage(
                familiar,
                OwnerDamageCommand {
                    target: target_key.as_bytes(),
                    occurrence: b"fixture:wound",
                    binding: b"fixture:wound\0fixture:qualified-defense-hp",
                    damage: 150,
                },
            )
            .unwrap();
        assert!(
            !runtime
                .familiar_defense_due(familiar, &policy, 1_999_999, 1)
                .unwrap()
        );
        assert!(
            runtime
                .prepare_familiar_defense(familiar, &policy, 1_999_999, 1)
                .unwrap()
                .is_none()
        );
        let mut saw_heal = false;
        for ordinal in 1..=32_u64 {
            let now = ordinal * 2_000_000;
            let before = runtime.companion_snapshot(familiar).unwrap();
            let prepared = runtime
                .prepare_familiar_defense(familiar, &policy, now, 1)
                .unwrap()
                .unwrap();
            assert_eq!(runtime.companion_snapshot(familiar).unwrap(), before);
            let receipt = runtime.commit_familiar_defense(prepared).unwrap();
            let after = runtime.companion_snapshot(familiar).unwrap();
            assert_eq!(receipt.health_before(), before.health);
            assert_eq!(receipt.health_after(), after.health);
            assert_eq!(
                after.state.familiar_defense.as_ref().unwrap().ordinal,
                ordinal
            );
            if receipt.effect().is_some() {
                saw_heal = true;
                assert_eq!(
                    after.health,
                    (before.health + 300).min(before.maximum_health)
                );
            } else {
                assert_eq!(after.health, before.health);
            }
            assert!(
                runtime
                    .prepare_familiar_defense(familiar, &policy, now, 1)
                    .unwrap()
                    .is_none()
            );
        }
        assert!(saw_heal);
        assert_eq!(runtime.companion_snapshot(familiar).unwrap().health, health);
    }
    #[test]
    fn actual_defense_rechecks_changed_hp_and_never_resurrects() {
        let (mut runtime, _, _, familiar, policy) = fixture();
        let target_key = runtime
            .companion_snapshot(familiar)
            .unwrap()
            .state
            .policy
            .definition_key
            .clone();
        let prepared = runtime
            .prepare_familiar_defense(familiar, &policy, 2_000_000, 1)
            .unwrap()
            .unwrap();
        runtime
            .borrow_exact_actor_commit()
            .commit_damage(
                familiar,
                OwnerDamageCommand {
                    target: target_key.as_bytes(),
                    occurrence: b"fixture:wound",
                    binding: b"fixture:wound\0fixture:qualified-defense-hp",
                    damage: 1,
                },
            )
            .unwrap();
        let changed = runtime.companion_snapshot(familiar).unwrap();
        assert!(runtime.commit_familiar_defense(prepared).is_err());
        assert_eq!(runtime.companion_snapshot(familiar).unwrap(), changed);
        runtime
            .borrow_exact_actor_commit()
            .commit_damage(
                familiar,
                OwnerDamageCommand {
                    target: target_key.as_bytes(),
                    occurrence: b"fixture:lethal",
                    binding: b"fixture:lethal\0fixture:qualified-defense-lethal",
                    damage: changed.health,
                },
            )
            .unwrap();
        assert!(
            runtime
                .prepare_familiar_defense(familiar, &policy, 2_000_000, 1)
                .is_err()
        );
        assert_eq!(
            runtime
                .companion_snapshot_including_dead(familiar)
                .unwrap()
                .health,
            0
        );
    }
}
