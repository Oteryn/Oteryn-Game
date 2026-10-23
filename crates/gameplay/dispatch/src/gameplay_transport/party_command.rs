//! Basic party commands on the existing physical owner, session fence and World writer.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::durability::{DurabilityError, spell_item_transaction as item_tx, world_party};
use crate::foundation::{CommandId, CommandRef, PartyAction};
use oteryn_protocol_oteryn::party::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(super) const OFFERED: OfferedCapability = OfferedCapability {
    id: CAPABILITY_PARTY_V1,
    requires: &[6],
};
/// Bounded connection-local opaque handles. Never use a cached handle as membership authority.
#[derive(Default)]
pub struct SessionPartyView {
    handles: BTreeMap<u64, (u8, [u8; 16])>,
}
const MAX_HANDLES: usize = 4096;
impl SessionPartyView {
    fn handle(&mut self, admitted: AdmittedSession, kind: u8, id: [u8; 16]) -> Option<u64> {
        let mut hash = Sha256::new();
        hash.update(b"oteryn.party.handle.v1");
        hash.update(admitted.game_session_id.as_bytes());
        hash.update(admitted.continuity.connection_generation.to_be_bytes());
        hash.update([kind]);
        hash.update(id);
        let bytes: [u8; 32] = hash.finalize().into();
        let handle = u64::from_be_bytes(bytes[..8].try_into().ok()?);
        if handle == 0
            || self
                .handles
                .get(&handle)
                .is_some_and(|old| *old != (kind, id))
        {
            return None;
        }
        if !self.handles.contains_key(&handle) && self.handles.len() >= MAX_HANDLES {
            return None;
        }
        self.handles.insert(handle, (kind, id));
        Some(handle)
    }
    fn resolve(&self, kind: u8, handle: u64) -> Option<[u8; 16]> {
        self.handles
            .get(&handle)
            .filter(|(k, _)| *k == kind)
            .map(|(_, id)| *id)
    }
    fn project(
        &mut self,
        admitted: AdmittedSession,
        view: world_party::ingress::PartyIngressView,
    ) -> Option<PartyResult> {
        let mut result = PartyResult::refused(PartyDisposition::Ok);
        result.shared_xp_enabled = view.source.shared_xp_enabled;
        for (id, name) in view.members {
            result.members.push(PartyEntry {
                handle: self.handle(admitted, 1, id)?,
                name,
                leader: view.source.leader == Some(id),
                expires_at: 0,
            });
        }
        for (id, name, expires_at) in view.invitations {
            result.invitations.push(PartyEntry {
                handle: self.handle(admitted, 2, id)?,
                name,
                leader: false,
                expires_at,
            });
        }
        Some(result)
    }
}
fn bytes(disposition: PartyDisposition) -> Option<Vec<u8>> {
    encode_party_result(&PartyResult::refused(disposition)).ok()
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn party_wire_command(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        command_type: u32,
        payload: &[u8],
        view: &mut SessionPartyView,
    ) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_PARTY_V1)
        {
            return None;
        }
        let intent = if command_type == COMMAND_TYPE_PARTY_QUERY {
            if !payload.is_empty() {
                return None;
            }
            None
        } else if command_type == COMMAND_TYPE_PARTY_INTENT {
            Some(decode_party_intent(payload).ok()?)
        } else {
            return None;
        };
        let Some(actor) = admitted.runtime_actor else {
            return bytes(PartyDisposition::Rejected);
        };
        let Some(fence) = admitted.item_fence else {
            return bytes(PartyDisposition::Unavailable);
        };
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return bytes(PartyDisposition::Rejected);
        }
        let command = CommandId::new(command_id).ok()?;
        let Some(permit) = self.spell_lane_permit().await else {
            return bytes(PartyDisposition::Unavailable);
        };
        let mut runtime = self.runtime.lock().await;
        if runtime.assert_actor_spell_unreserved(actor).is_err()
            || !runtime
                .player_control_facts(actor, admitted.game_session_id)
                .is_ok_and(|p| p.control_loss.is_none())
            || !runtime
                .bound_attacker_lease(actor, admitted.game_session_id)
                .is_ok_and(|l| {
                    l.character_id().as_bytes() == fence.character_id.as_bytes()
                        && l.generation() == fence.character_lease_generation
                })
        {
            return bytes(PartyDisposition::Rejected);
        }
        // Keep the same guard order as attacks/spells. A hit cannot race the leave proof.
        let _states = self.states.spell_states.lock().await;
        let attack = self.states.attack.lock().await;
        let now = self.owner_now();
        let invite_target = if let Some(PartyIntent::Invite(target)) = intent {
            let visible = runtime.visible_entities();
            let Some(target_actor) = visible
                .players
                .iter()
                .find(|p| {
                    p.actor.placement_identity() == target.identity
                        && p.generation == target.generation
                })
                .map(|p| p.actor)
            else {
                return bytes(PartyDisposition::NotVisible);
            };
            let Ok(census) = runtime.positioned_actor_census() else {
                return bytes(PartyDisposition::Unavailable);
            };
            let Some(target_session) = census
                .into_iter()
                .find(|(a, _, _)| *a == target_actor)
                .and_then(|(_, _, s)| s)
            else {
                return bytes(PartyDisposition::NotVisible);
            };
            Some((target_actor, target_session))
        } else {
            None
        };
        let action = match intent {
            Some(PartyIntent::Accept(h)) => Some(PartyAction::Accept {
                party: match view.resolve(2, h) {
                    Some(id) => id,
                    None => return bytes(PartyDisposition::NotInvited),
                },
            }),
            Some(PartyIntent::Kick(h)) => Some(PartyAction::Kick {
                target: match view.resolve(1, h) {
                    Some(id) => id,
                    None => return bytes(PartyDisposition::NotMember),
                },
            }),
            Some(PartyIntent::PassLeadership(h)) => Some(PartyAction::TransferLeader {
                target: match view.resolve(1, h) {
                    Some(id) => id,
                    None => return bytes(PartyDisposition::NotMember),
                },
            }),
            Some(PartyIntent::Leave) => Some(PartyAction::Leave),
            Some(PartyIntent::Invite(_)) | None => None,
        };
        let mut binding_bytes = b"oteryn.party.wire.v1".to_vec();
        binding_bytes.extend_from_slice(&command_type.to_be_bytes());
        if let Some(intent) = intent {
            binding_bytes.extend_from_slice(&encode_party_intent(intent).ok()?);
        }
        let binding: [u8; 32] = Sha256::digest(binding_bytes).into();
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            return bytes(PartyDisposition::Unavailable);
        };
        let mut ctx = (self, &permit, &runtime, &attack);
        let outcome = pass.run_with_context(&mut ctx, move |holder, deadline, ctx| {
            Box::pin(async move {
                let (owner, permit, runtime, attack) = ctx;
                let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = item_tx::assert_spell_item_authority_in_transaction(&mut tx, permit, owner.root, owner.character, owner.holder, &fence, CommandRef::new(admitted.game_session_id, command), binding).await.map_err(|_| DurabilityError::Unavailable)?;
                let visible = if let Some((target, session)) = invite_target {
                    crate::durability::admission_journal::party_target_binding::prove_visible_party_target_in_transaction(&mut tx, owner.root, &authority, runtime, actor, target, session).await?
                } else { None };
                let action = visible.as_ref().map(|v| PartyAction::Invite { target: v.character() }).or(action);
                let receipt = if let Some(action) = action {
                    let receipt = if matches!(action, PartyAction::Leave) {
                        world_party::ingress::leave_in_transaction(&mut tx, &authority, runtime, actor, attack.party_attack_state(actor, admitted.game_session_id), now).await
                    } else {
                        world_party::apply_world_party_command_in_transaction(&mut tx, &authority, action, visible.as_ref()).await
                    }.map_err(|_| DurabilityError::Unavailable)?;
                    Some(receipt)
                } else { None };
                // Never turn a failed visibility proof into a successful query result.
                let refused = invite_target.is_some() && visible.is_none();
                let view = world_party::ingress::read_in_transaction(&mut tx, &authority).await.map_err(|_| DurabilityError::Unavailable)?;
                crate::durability::party_kill_xp::commit_party_kill_transaction(tx, deadline).await?;
                Ok((receipt, view, refused))
            })
        }).await;
        match outcome {
            Ok((receipt, source, refused)) => {
                if runtime
                    .project_world_party(actor, admitted.game_session_id, source.source.clone())
                    .is_err()
                {
                    return bytes(PartyDisposition::Unavailable);
                }
                if refused {
                    return bytes(PartyDisposition::NotVisible);
                }
                if let Some(receipt) = receipt {
                    return bytes(match receipt.disposition.as_str() {
                        "APPLIED" => PartyDisposition::Ok,
                        "NOT_LEADER" => PartyDisposition::NotLeader,
                        "ALREADY_MEMBER" => PartyDisposition::AlreadyInParty,
                        "NOT_INVITED" => PartyDisposition::NotInvited,
                        "NOT_VISIBLE" => PartyDisposition::NotVisible,
                        "LOGOUT_BLOCKED" => PartyDisposition::LogoutBlocked,
                        "NOT_MEMBER" => PartyDisposition::NotMember,
                        _ => PartyDisposition::Rejected,
                    });
                }
                let Some(projected) = view.project(admitted, source) else {
                    return bytes(PartyDisposition::Unavailable);
                };
                encode_party_result(&projected)
                    .ok()
                    .or_else(|| bytes(PartyDisposition::Unavailable))
            }
            Err(_) => bytes(PartyDisposition::Unavailable),
        }
    }
}

#[cfg(test)]
#[path = "party_command_tests.rs"]
mod tests;
