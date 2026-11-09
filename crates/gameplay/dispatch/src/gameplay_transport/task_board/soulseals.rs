//! T114: one-use authorization, authoritative catalogue, and atomic solo entry.
use super::{ComposedFreshAdmission, TaskBoardReply};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::{
    SecureIdentifiers,
    connection::{AdmittedSession, ConnectionIdentifiers, UseCommand},
};
use oteryn_protocol_oteryn::task_board::{self as board, soulpit as wire};
use std::{collections::BTreeMap, sync::Mutex};
#[derive(Clone)]
struct Quote {
    token: [u8; 16],
    expires: u64,
    fence: CurrentCharacterItemFence,
    content: [u8; 32],
}
#[derive(Default)]
pub(crate) struct Quotes(Mutex<BTreeMap<[u8; 16], Quote>>);
fn rules() -> wire::EntryRules {
    wire::EntryRules {
        required_level: crate::domain::soulpit::REQUIRED_LEVEL,
        time_limit_seconds: (crate::domain::soulpit::TIME_LIMIT_MICROS / 1_000_000) as u32,
        authorization_ms: 10_000,
    }
}
fn refused(reason: &str) -> TaskBoardReply {
    encode(wire::Reply {
        status: wire::Status::Refused,
        balance: None,
        authorization: None,
        next: None,
        total: 0,
        entries: Vec::new(),
        rules: rules(),
        reason: reason.into(),
    })
}
fn encode(reply: wire::Reply) -> TaskBoardReply {
    reply
        .encode()
        .map_or(TaskBoardReply::Rejected, TaskBoardReply::Payload)
}
impl Quotes {
    fn revoke(&self, key: [u8; 16]) -> Result<(), ()> {
        self.0.lock().map_err(|_| ())?.remove(&key);
        Ok(())
    }
    fn issue(&self, key: [u8; 16], quote: Quote, now: u64) -> Result<(), ()> {
        let mut quotes = self.0.lock().map_err(|_| ())?;
        quotes.retain(|_, q| q.expires > now);
        if !quotes.contains_key(&key) && quotes.len() >= 4096 {
            return Err(());
        }
        quotes.insert(key, quote);
        Ok(())
    }
    fn consume(
        &self,
        key: [u8; 16],
        token: [u8; 16],
        fence: CurrentCharacterItemFence,
        content: [u8; 32],
        now: u64,
    ) -> bool {
        self.0
            .lock()
            .ok()
            .and_then(|mut q| q.remove(&key))
            .is_some_and(|q| {
                q.token == token && q.expires > now && q.fence == fence && q.content == content
            })
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn soulseal_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> TaskBoardReply {
        let Ok(request) = wire::Request::decode(payload) else {
            return TaskBoardReply::Rejected;
        };
        let Ok(id) = CommandId::new(id) else {
            return TaskBoardReply::Rejected;
        };
        let Some(fence) = admitted.item_fence else {
            return TaskBoardReply::Rejected;
        };
        if !admitted
            .continuity
            .selected_capabilities
            .contains(board::CAPABILITY_TASK_BOARD_V1)
            || fence.game_session_id != admitted.game_session_id
        {
            return TaskBoardReply::Rejected;
        }
        let command = CommandRef::new(admitted.game_session_id, id);
        if matches!(&request, wire::Request::Enter { .. }) {
            match self
                .root
                .read_soulseal_receipt(self.character, fence.character_id, command)
                .await
            {
                Ok(Some(receipt)) => {
                    if receipt.intent != payload {
                        return TaskBoardReply::Rejected;
                    }
                    match self.retry_native_soulpit(admitted, id.get(), payload).await {
                        Ok(Some(outcome)) => {
                            if self.soulpit_result(outcome).is_err() {
                                return TaskBoardReply::Unavailable;
                            }
                        }
                        Ok(None) => {}
                        Err(()) => return TaskBoardReply::Unavailable,
                    }
                    return encode(wire::Reply {
                        status: wire::Status::Started,
                        balance: Some(receipt.balance),
                        authorization: None,
                        next: None,
                        total: 0,
                        entries: Vec::new(),
                        rules: rules(),
                        reason: String::new(),
                    });
                }
                Ok(None) => {}
                Err(_) => return TaskBoardReply::Unavailable,
            }
        }
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return refused("Fight content unavailable");
        };
        let Some(current) = crate::gameplay_transport::character_gameplay_fence::current(
            self,
            admitted.game_session_id,
            false,
        )
        .await
        else {
            return refused("Session no longer current");
        };
        if current.item_fence() != fence {
            return refused("Session authority changed");
        }
        let Some(actor) = admitted.runtime_actor else {
            return TaskBoardReply::Rejected;
        };
        let mut fights = crate::durability::task_board::soulseal_fights(native);
        let ready = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime.binding().runtime_scope() != fence.runtime_scope
            {
                return TaskBoardReply::Rejected;
            }
            fights.retain(|fight| {
                native
                    .creature_profiles()
                    .records
                    .iter()
                    .find(|r| r.profile.target.key == fight.species)
                    .is_some_and(|r| {
                        crate::ai_think::CreatureAiTable::qualify_soulpit_creature(
                            &runtime,
                            native,
                            &r.profile.target,
                        )
                        .is_ok()
                    })
            });
            let states = self.states.spell_states.lock().await;
            !states.is_dead(actor)
                && states.spell_idle(&runtime, actor, admitted.game_session_id)
                && runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|f| f.control_loss.is_none())
                && runtime.positioned_actor_census().ok().is_some_and(|c| {
                    c.iter().any(|(a, p, s)| {
                        *a == actor && *s == Some(admitted.game_session_id) && {
                            let p = p.position();
                            (p.x, p.y, p.floor) == (32375, 31158, 8)
                        }
                    })
                })
        };
        if fights.is_empty() {
            return refused("No qualified fights are available");
        }
        match request {
            wire::Request::Catalogue { after } => {
                // Even an unready refresh invalidates the previous entrance permit.
                if self
                    .states
                    .soulseal_quotes
                    .revoke(*admitted.game_session_id.as_bytes())
                    .is_err()
                {
                    return TaskBoardReply::Unavailable;
                }
                let Ok((balance, level)) = self
                    .root
                    .read_current_soulseals(self.character, self.holder, current)
                    .await
                else {
                    return TaskBoardReply::Unavailable;
                };
                let ready = ready && level >= crate::domain::soulpit::REQUIRED_LEVEL;
                let now = self.owner_now().get();
                let token = if ready {
                    let Some(token) = SecureIdentifiers.game_session_id().map(|s| *s.as_bytes())
                    else {
                        return TaskBoardReply::Unavailable;
                    };
                    if self
                        .states
                        .soulseal_quotes
                        .issue(
                            *admitted.game_session_id.as_bytes(),
                            Quote {
                                token,
                                expires: now
                                    .saturating_add(crate::domain::soulpit::SOULSEAL_AUTH_MICROS),
                                fence,
                                content: native.source_digest(),
                            },
                            now,
                        )
                        .is_err()
                    {
                        return TaskBoardReply::Unavailable;
                    }
                    Some(token)
                } else {
                    None
                };
                let total = match fights.len().try_into() {
                    Ok(n) if n <= 4096 => n,
                    _ => return refused("Fight catalogue exceeds its bound"),
                };
                let mut page: Vec<_> = fights
                    .into_iter()
                    .filter(|f| f.race > after)
                    .take(wire::PAGE_SIZE + 1)
                    .collect();
                let more = page.len() > wire::PAGE_SIZE;
                page.truncate(wire::PAGE_SIZE);
                let next = more.then(|| page.last().map(|f| f.race)).flatten();
                encode(wire::Reply {
                    status: wire::Status::Catalogue,
                    balance: Some(balance),
                    authorization: token,
                    next,
                    total,
                    rules: rules(),
                    reason: if ready {
                        String::new()
                    } else {
                        "Stand on Soulpit entrance slot one, alive and idle, to authorize entry"
                            .into()
                    },
                    entries: page
                        .into_iter()
                        .map(|f| wire::Fight {
                            race: f.race,
                            name: f.name,
                            stars: f.stars,
                            cost: f.cost,
                        })
                        .collect(),
                })
            }
            wire::Request::Enter {
                authorization,
                race,
            } => {
                if !self.states.soulseal_quotes.consume(
                    *admitted.game_session_id.as_bytes(),
                    authorization,
                    fence,
                    native.source_digest(),
                    self.owner_now().get(),
                ) {
                    return refused(
                        "Entrance authorization expired or was already used; refresh the catalogue",
                    );
                }
                let Some(fight) = fights.into_iter().find(|f| f.race == race) else {
                    return refused("Fight is not available");
                };
                if !ready {
                    return refused("Entry requires the Soulpit entrance slot, alive and idle");
                }
                let request = crate::gameplay_transport::soulpit::SoulpitEntryIntent {
                    actor,
                    command: UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: id.get(),
                        item_fence: Some(fence),
                    },
                    encoded_intent: payload.into(),
                    selected: Some(
                        crate::gameplay_transport::soulpit::SoulpitSelection::Soulseals {
                            species: fight.species,
                        },
                    ),
                };
                match self.enter_shared_soulpit(request).await {
                    Ok(Some(result))
                        if result.disposition
                            == oteryn_protocol_oteryn::world_object::UseDisposition::Committed =>
                    {
                        let Ok(Some(receipt)) = self
                            .root
                            .read_soulseal_receipt(self.character, fence.character_id, command)
                            .await
                        else {
                            return TaskBoardReply::Unavailable;
                        };
                        encode(wire::Reply {
                            status: wire::Status::Started,
                            balance: Some(receipt.balance),
                            authorization: None,
                            next: None,
                            total: 0,
                            entries: Vec::new(),
                            rules: rules(),
                            reason: String::new(),
                        })
                    }
                    Ok(_) => refused(
                        "Entry refused: check level, seals and room capacity, then refresh; no payment committed",
                    ),
                    Err(()) => TaskBoardReply::Unavailable,
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "soulseal_quotes_tests.rs"]
mod tests;
