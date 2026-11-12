//! Real party authority and admitted router, reused by the existing membership scenario.
use crate::bestiary_postgres_harness::{TestResult, debug};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{
        AdmittedSession, ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
    },
    tcp_tls::write_frame,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
    party_effects::*, world_spatial_entities::EntityRef,
};
use tokio::io::AsyncWriteExt;

#[test]
fn party_effects_real_dispatch_postgres_membership_and_shared_eligibility() -> TestResult {
    let started = std::time::Instant::now();
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::party_membership::run(true)?;
    super::party_buffs::run_party_buffs(None)?;
    for extra in [
        ("train_party", "knight"),
        ("enchant_party", "sorcerer"),
        ("heal_party", "druid"),
    ] {
        super::party_buffs::run_party_buffs(Some(extra))?;
    }
    eprintln!(
        "POSTGRES_EXECUTED case=party_effects elapsed={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    targets: &[EntityRef],
    shields: &[u8],
) -> TestResult {
    let original = admitted.continuity.selected_capabilities;
    // This legacy fixed-room harness supplies an already-admitted session. Model
    // the new bundle client's actual generated offer, rather than its fallback
    // fixture owner's non-bundle offer. The production party authority is unchanged.
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, 42, 280])
            .ok_or("party effects selection")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_PARTY_EFFECTS_V1)
    );
    assert!(!admitted.continuity.selected_capabilities.contains(134));
    let current = *admitted;
    let actor = current.runtime_actor.ok_or("actor")?;
    let own_index = targets
        .iter()
        .position(|target| target.identity == actor.placement_identity())
        .ok_or("self target")?;
    let shield = *shields.get(own_index).ok_or("self shield")?;
    let (mut server, mut client) = tokio::io::duplex(32768);
    let serving = async { serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await };
    let driving = async {
        loop {
            let frame = super::party_membership::read_live_frame(
                &mut client,
                current.continuity.connection_generation,
            )
            .await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        for (offset, payload) in [&[][..], &[0][..]].into_iter().enumerate() {
            let command_id = current.continuity.next_command_id + offset as u64;
            let frame = encode_client_command(
                current.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
                    command_type: COMMAND_TYPE_PARTY_EFFECTS_QUERY,
                    payload,
                },
            )
            .map_err(debug)?;
            write_frame(&mut client, &frame).await?;
            loop {
                let frame = super::party_membership::read_live_frame(
                    &mut client,
                    current.continuity.connection_generation,
                )
                .await?;
                let env = decode_wire_envelope(&frame).map_err(debug)?;
                if env.message_type() != MessageType::CommandResult {
                    continue;
                }
                let result = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command_id);
                if offset == 1 {
                    assert_eq!(result.status, CommandStatus::Rejected);
                    assert!(result.payload.is_empty());
                    break;
                }
                assert_eq!(result.status, CommandStatus::Accepted);
                let snapshot =
                    PartyEffects::decode(result.payload).ok_or("decode party effects")?;
                assert_eq!(snapshot.in_party, shield != 0);
                assert_eq!(snapshot.shared_enabled, matches!(shield, 5..=10));
                assert_eq!(snapshot.shared_active, matches!(shield, 5 | 6));
                assert_eq!(snapshot.eligible, matches!(shield, 5 | 6 | 9 | 10));
                assert_eq!(snapshot.bonuses, [0; 5]);
                assert_eq!(snapshot.party_spell_remaining_ms, [0; 5]);
                break;
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    result?;
    let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
        return Err(format!("party effects connection: {end:?}").into());
    };
    admitted.continuity = ended.continuity;
    let mut stale = *admitted;
    stale
        .item_fence
        .as_mut()
        .ok_or("fence")?
        .character_lease_generation = 0;
    assert!(owner.party_effects_page(stale).await.is_none());
    let mut old = *admitted;
    old.continuity.selected_capabilities = SelectedCapabilities::NONE;
    assert!(owner.party_effects_page(old).await.is_none());
    admitted.continuity.selected_capabilities = original;
    Ok(())
}

pub(super) async fn check_buffs(
    h: &crate::bestiary_postgres_harness::Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    spell: &str,
    affected: bool,
) -> TestResult {
    use crate::gameplay_transport::connection::{FirstEntryOutcome, SessionContinuity};
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("buff fence")?;
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    };
    let guard = AdmissionGuardStore::from_root(h.root.clone())
        .load(&[AdmissionAuthorityGuardKeyV1::Runtime(fence.runtime_scope)])
        .await?
        .into_iter()
        .next()
        .flatten()
        .ok_or("current runtime guard")?;
    let AdmissionAuthorityGuardStateV1::Runtime {
        ready: true,
        ownership_generation,
        world_policy_revision,
        ..
    } = guard.state
    else {
        return Err("ready runtime guard required".into());
    };
    assert_eq!(ownership_generation, fence.scope_ownership_generation.get());
    // The strong owner resolves the current runtime policy revision. A fixture
    // policy under an unrelated revision cannot qualify this observation.
    sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,$2) ON CONFLICT DO NOTHING").bind(owner.world_id.as_bytes().as_slice()).bind(&world_policy_revision).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,$2,2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'party-effects-fixture') ON CONFLICT DO NOTHING").bind(owner.world_id.as_bytes().as_slice()).bind(&world_policy_revision).execute(&h.pool).await?;
    let mut continuity = SessionContinuity::FRESH;
    continuity.connection_generation = fence.connection_generation.get();
    continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, 42, 280])
            .ok_or("buff capability")?;
    assert!(
        continuity
            .selected_capabilities
            .contains(CAPABILITY_PARTY_EFFECTS_V1)
    );
    assert!(!continuity.selected_capabilities.contains(134));
    let admitted = AdmittedSession {
        game_session_id: session,
        world_id: owner.world_id,
        channel_id: owner.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity,
        item_fence: Some(fence.item_fence()),
    };
    let bytes = owner
        .party_effects_query(admitted, 1)
        .await
        .ok_or("party buff projection")?;
    let snapshot = PartyEffects::decode(&bytes).ok_or("decode party buff projection")?;
    let index = match spell {
        "train_party" => 0,
        "protect_party" => 1,
        "enchant_party" => 2,
        "heal_party" => 3,
        "enlighten_party" => 4,
        _ => return Err("party spell fixture".into()),
    };
    let full = super::cl6_status::query_actor(owner, actor, session).await?;
    let member = full
        .members
        .iter()
        .find(|member| member.character == *fence.character_id.as_bytes())
        .ok_or("CL6 party member")?;
    assert!(
        member.conditions.is_some(),
        "same-Channel member must have a qualified condition observation"
    );
    let projected = member.party_spell_remaining_ms[index];
    if affected {
        assert!(
            projected > 0 && projected <= 120000,
            "CL6 {spell}: {projected}"
        );
    } else {
        assert_eq!(projected, 0, "CL6 out-of-range member");
    }
    assert!(
        full.members.len() >= 3,
        "native projection includes other members, not only self"
    );
    eprintln!("CL6_PARTY POSTGRES_ASSERTION_EXECUTED spell={spell} affected={affected}");
    let remaining = snapshot.party_spell_remaining_ms[index];
    if affected {
        assert!(remaining > 0 && remaining <= 120000, "{spell}: {remaining}");
    } else {
        assert_eq!(remaining, 0, "{spell}: distant member");
    }
    Ok(())
}
