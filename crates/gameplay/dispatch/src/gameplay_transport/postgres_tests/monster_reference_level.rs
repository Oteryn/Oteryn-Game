//! MRL-3: current SQL party membership feeds the real scheduled monster reader.
use crate::bestiary_postgres_harness::{TestResult, debug};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

#[test]
fn reference_level_party_lookup_postgres_real_dispatch() -> TestResult {
    super::party_membership::run_reference_level()
}

pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &[AdmittedSession],
) -> TestResult {
    use crate::durability::spell_item_transaction::{
        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
    };
    let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    for (index, session) in admitted.iter().enumerate() {
        let mut facts = crate::gameplay_transport::actor_spell::tests::FACTS;
        facts.level = [300, 800, 900][index];
        states
            .initialize(
                &runtime,
                session.runtime_actor.ok_or("actor")?,
                session.game_session_id,
                facts,
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("initialize player state")?;
    }
    let pass = owner.root.try_issue_semantic_pass().map_err(debug)?;
    let mut context = (owner, &runtime, &states, &permit, admitted);
    pass.run_with_context(&mut context, |holder, deadline, context| {
        Box::pin(async move {
            let (owner, runtime, states, permit, admitted) = context;
            let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
            let binding = runtime.binding();
            let scope = crate::foundation::RuntimeScopeRefV1::channel(
                binding.world_id(), binding.channel_id(),
            );
            let authority = assert_spell_item_scope_in_transaction(
                &mut tx, permit, owner.root, owner.character, owner.holder,
                scope, binding.scope_generation().get(),
            ).await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            let parties = crate::gameplay_transport::monster_profile_cycle::physical_spell::read_reference_parties(
                &mut tx, &authority, owner.root, runtime, states, true,
            ).await;
            assert_eq!(parties.len(), admitted.len(), "every admitted member has a current party");
            for session in admitted.iter() {
                let actor = session.runtime_actor.expect("admitted actor");
                let members = parties.get(&actor.placement_identity()).expect("current party");
                assert_eq!(members.len(), admitted.len());
                for member in admitted.iter() {
                    let actor = member.runtime_actor.expect("member actor");
                    let position = runtime.read_actor_position(actor).expect("current position").position();
                    let level = states.get(runtime, actor, member.game_session_id).expect("current state").character_facts().level;
                    assert!(members.contains(&(position, level)));
                }
            }
            let unavailable_states = crate::gameplay_transport::actor_spell::ChannelSpellStates::default();
            let unavailable = crate::gameplay_transport::monster_profile_cycle::physical_spell::read_reference_parties(
                &mut tx, &authority, owner.root, runtime, &unavailable_states, true,
            ).await;
            assert!(unavailable.is_empty(), "missing current levels fall back to per-player damage");
            let disabled = crate::gameplay_transport::monster_profile_cycle::physical_spell::read_reference_parties(
                &mut tx, &authority, owner.root, runtime, states, false,
            ).await;
            assert!(disabled.is_empty(), "default per-player content needs no party lookup");
            tx.rollback().await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            Ok(())
        })
    }).await.map_err(debug)?;
    Ok(())
}
