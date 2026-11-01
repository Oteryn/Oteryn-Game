//! Fenced dispatch for the existing Team Finder owner and ordinary party invitation.
use super::actor::{actor_is_current, party_target};
use super::handles::{HandleKind, SessionTeamFinderView};
use super::projection::{DatabaseResult, project};
use crate::durability::{DurabilityError, spell_item_transaction as item_tx, team_finder};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::team_finder_members::*;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
enum Intent {
    Browse(Option<u64>),
    Applicants(Option<u64>),
    Member { handle: u64, request: bool },
    Leader { handle: u64, status: MemberStatus },
}

impl Intent {
    fn decode(command_type: u32, payload: &[u8]) -> Option<Self> {
        match command_type {
            COMMAND_TYPE_TEAM_FINDER_QUERY => match decode_query(payload).ok()? {
                TeamFinderQuery::Browse { after } => Some(Self::Browse(after)),
                TeamFinderQuery::Applicants { after } => Some(Self::Applicants(after)),
            },
            COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION => {
                let (handle, request) = match decode_member_action(payload).ok()? {
                    MemberAction::Request { leader } => (leader, true),
                    MemberAction::Cancel { leader } => (leader, false),
                };
                Some(Self::Member { handle, request })
            }
            COMMAND_TYPE_TEAM_FINDER_LEADER_ACTION => {
                let (handle, status) = match decode_leader_action(payload).ok()? {
                    LeaderAction::Invite { member } => (member, MemberStatus::Invited),
                    LeaderAction::Accept { member } => (member, MemberStatus::Accepted),
                    LeaderAction::Deny { member } => (member, MemberStatus::Denied),
                };
                Some(Self::Leader { handle, status })
            }
            _ => None,
        }
    }

    fn handle(self) -> (HandleKind, Option<u64>) {
        match self {
            Self::Browse(after) => (HandleKind::Listing, after),
            Self::Applicants(after) => (HandleKind::Applicant, after),
            Self::Member { handle, .. } => (HandleKind::Listing, Some(handle)),
            Self::Leader { handle, .. } => (HandleKind::Applicant, Some(handle)),
        }
    }

    fn query(self) -> bool {
        matches!(self, Self::Browse(_) | Self::Applicants(_))
    }

    fn accepts(self) -> bool {
        matches!(
            self,
            Self::Leader {
                status: MemberStatus::Accepted,
                ..
            }
        )
    }

    fn feedback(self) -> Option<&'static str> {
        // Crystal protocolgame.cpp:3097-3105,3148; emitted only after a new commit.
        match self {
            Self::Member { request: true, .. } => Some("There is a new request to join your team."),
            Self::Leader {
                status: MemberStatus::Invited,
                ..
            } => Some("You are invited to a new team."),
            Self::Leader {
                status: MemberStatus::Accepted,
                ..
            } => Some("Your team finder request was accepted."),
            Self::Leader {
                status: MemberStatus::Denied,
                ..
            } => Some("Your team finder request was denied."),
            _ => None,
        }
    }
}

pub(in crate::gameplay_transport) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
    view: &mut SessionTeamFinderView,
) -> Option<(bool, Vec<u8>)> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_TEAM_FINDER_MEMBERS_V1)
    {
        return Some((false, Vec::new()));
    }
    let Some(intent) = Intent::decode(command_type, payload) else {
        return Some((false, Vec::new()));
    };
    let (kind, handle) = intent.handle();
    let target = handle.and_then(|handle| {
        view.resolve(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
            kind,
            handle,
        )
    });
    if intent.query() && handle.is_some() && target.is_none() {
        return Some((false, Vec::new()));
    }
    let fence = admitted.item_fence?;
    let actor = admitted.runtime_actor?;
    let permit = owner.spell_lane_permit().await?;
    let runtime = owner.runtime.lock().await;
    if !actor_is_current(&runtime, admitted) {
        return Some((false, Vec::new()));
    }
    let reserved = intent.accepts() && runtime.assert_actor_spell_unreserved(actor).is_err();
    let invite_target = if intent.accepts() {
        target.and_then(|target| {
            party_target(&runtime, target.character)
                .filter(|(_, session)| *session == target.session)
        })
    } else {
        None
    };
    // The ordinary invitation proof borrows the current runtime, just as PartyIntent::Invite.
    // Other operations release the runtime before their database work.
    let runtime = intent.accepts().then_some(runtime);
    let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
    let mut hash = Sha256::new();
    hash.update(b"oteryn.team-finder.wire.v1");
    hash.update(command_type.to_be_bytes());
    hash.update(payload);
    let binding = hash.finalize().into();
    let pass = owner.root.try_issue_semantic_pass().ok()?;
    let mut context = (owner, runtime.as_deref());
    let outcome = pass.run_with_context(&mut context, move |holder, deadline, context| {
        Box::pin(async move {
            let (owner, runtime) = context;
            let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
            let authority = item_tx::assert_spell_item_authority_in_transaction(
                &mut tx, &permit, owner.root, owner.character, owner.holder, &fence, command, binding,
            ).await.map_err(|_| DurabilityError::Unavailable)?;
            let result = if !intent.query()
                && let Some(receipt) = team_finder::replay_in_transaction(&mut tx, &authority)
                    .await.map_err(|_| DurabilityError::Unavailable)?
            {
                DatabaseResult::Mutation(receipt)
            } else if reserved {
                // A retained result wins over a later transient spell reservation.
                DatabaseResult::Mutation(rejected())
            } else {
                let limit = i64::try_from(MAX_PAGE_ENTRIES + 1)
                    .map_err(|_| DurabilityError::Unavailable)?;
                match intent {
                    Intent::Browse(_) => DatabaseResult::Browse(
                        team_finder::browse_in_transaction(&mut tx, &authority, target.map(|t| t.character), limit)
                            .await.map_err(|_| DurabilityError::Unavailable)?,
                    ),
                    Intent::Applicants(_) => {
                        let (listing, occupied, rows) = team_finder::applicants_in_transaction(
                            &mut tx, &authority, target.map(|t| t.character), limit,
                        ).await.map_err(|_| DurabilityError::Unavailable)?;
                        DatabaseResult::Applicants(listing, occupied, rows)
                    }
                    Intent::Member { request, .. } => DatabaseResult::Mutation(if let Some(target) = target {
                        team_finder::apply_member_in_transaction(&mut tx, &authority, target, request)
                            .await.map_err(|_| DurabilityError::Unavailable)?
                    } else { rejected() }),
                    Intent::Leader { status, .. } => DatabaseResult::Mutation(if let Some(target) = target {
                        let visible = if let (Some(runtime), Some((target_actor, session))) = (runtime, invite_target) {
                            crate::durability::admission_journal::party_target_binding::prove_visible_party_target_in_transaction(
                                &mut tx, owner.root, &authority, runtime, actor, target_actor, session,
                            ).await?
                        } else { None };
                        team_finder::apply_leader_member_in_transaction(&mut tx, &authority, target, status, visible.as_ref())
                            .await.map_err(|_| DurabilityError::Unavailable)?
                    } else { rejected() }),
                }
            };
            crate::durability::party_kill_xp::commit_party_kill_transaction(tx, deadline).await?;
            Ok(result)
        })
    }).await.ok()?;
    if let DatabaseResult::Mutation(receipt) = &outcome
        && receipt.accepted
        && !receipt.replayed
        && let (Some(session), Some(text)) = (receipt.notify_session, intent.feedback())
    {
        owner.states.chat.push_party_spell_feedback(session, text);
    }
    project(view, admitted, outcome)
}

fn rejected() -> team_finder::MutationOutcome {
    team_finder::MutationOutcome {
        accepted: false,
        replayed: false,
        notify_session: None,
    }
}
