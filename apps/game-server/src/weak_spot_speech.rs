//! Actual source-specific ChatLine mailbox; no synthetic CHAT_INTENT or durable ledger.
use crate::chat::{ChatPosition, ChatText, SpeechMode, hears};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use oteryn_protocol_oteryn::chat::{
    ChatLine, ChatSpeaker, ChatSpeechMode, ChatWireError, encode_chat_line,
};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use std::collections::VecDeque;
use std::num::NonZeroU64;

// CHAT0-RL-11 existing undelivered lines per session ceiling; marker shares the bound.
const PENDING_LINES: usize = 64;
#[derive(Debug)]
struct Recipient {
    actor: ExactActorRef,
    session: GameSessionId,
    lines: VecDeque<(MovementLocalPosition, Vec<u8>)>,
    dropped: bool,
}
#[derive(Debug, Default)]
pub(crate) struct WeakSpotSpeechMailbox {
    welter_delivered: Vec<(ExactActorRef, u64)>,
    recipients: Vec<Recipient>,
    // Ephemeral per physical creature generation; prune with projected death owner state.
    delivered: Vec<ExactActorRef>,
}
#[derive(Debug)]
pub(crate) enum WeakSpotSpeechError {
    Owner(CarrierError),
    Wire(ChatWireError),
}
impl From<CarrierError> for WeakSpotSpeechError {
    fn from(e: CarrierError) -> Self {
        Self::Owner(e)
    }
}
impl From<ChatWireError> for WeakSpotSpeechError {
    fn from(e: ChatWireError) -> Self {
        Self::Wire(e)
    }
}
impl WeakSpotSpeechMailbox {
    /// Native sealed death+source gate; recipients are revalidated against current carrier.
    /// Caller invokes once per committed source death under the runtime lock. No subscriber
    /// list grants authority: only exact committed players of this owner are considered.
    pub(crate) fn publish(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        candidates: &[(ExactActorRef, GameSessionId)],
    ) -> Result<usize, WeakSpotSpeechError> {
        let (death, at) = {
            let owner = runtime.borrow_combat_death();
            if owner.creature_target_identity(actor)? != b"oteryn:creature.weak_spot" {
                return Ok(0);
            }
            owner.projected_death(actor)?
        };
        if self.delivered.contains(&actor) {
            return Ok(0);
        }
        let text = "The weak spot of the gates crashes!";
        // Actual native text rules; fixed donor text cannot fail this check.
        debug_assert!(ChatText::parse(text).is_ok());
        let line = ChatLine::Local {
            speaker: ChatSpeaker {
                identity: actor.placement_identity(),
                generation: NonZeroU64::new(death.actor_local_generation())
                    .ok_or(CarrierError::StaleActorGeneration)?,
            },
            speaker_name: "Weak Spot".to_owned(),
            mode: ChatSpeechMode::Say,
            text: text.to_owned(),
            position: ActorPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        };
        let payload = encode_chat_line(&line)?;
        {
            let owner = runtime.borrow_combat_death();
            self.delivered
                .retain(|old| owner.projected_death(*old).is_ok());
        }
        self.recipients.retain(|r| {
            runtime
                .player_control_facts(r.actor, r.session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        });
        let mut added = 0;
        let mut seen = Vec::new();
        for &(player, session) in candidates {
            if !runtime
                .player_control_facts(player, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            {
                continue;
            }
            if seen.contains(&(player, session)) {
                continue;
            }
            seen.push((player, session));
            let Ok(position) = runtime.read_actor_position(player) else {
                continue;
            };
            if !audible(at, position.position()) {
                continue;
            }
            let index = match self
                .recipients
                .iter()
                .position(|r| r.actor == player && r.session == session)
            {
                Some(index) => index,
                None => {
                    self.recipients.push(Recipient {
                        actor: player,
                        session,
                        lines: VecDeque::new(),
                        dropped: false,
                    });
                    self.recipients.len() - 1
                }
            };
            let receiver = &mut self.recipients[index];
            if receiver.lines.len() >= PENDING_LINES {
                receiver.dropped = true;
            }
            let maximum = if receiver.dropped {
                PENDING_LINES - 1
            } else {
                PENDING_LINES
            };
            while receiver.lines.len() >= maximum {
                receiver.lines.pop_front();
            }
            receiver.lines.push_back((at, payload.clone()));
            added += 1;
        }
        // Bound follows current carrier projected deaths; not persistent replay state.
        self.delivered.push(actor);
        Ok(added)
    }
    /// Actual protocol bytes for a current committed recipient. The connection-loop consumer
    /// sends them as existing CHAT delta1 behind capability7; no new protocol id is defined.
    /// Replacement snapshots must call clear_session; queued transient lines are not durable.
    pub(crate) fn drain(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<Vec<Vec<u8>>, WeakSpotSpeechError> {
        let index = self
            .recipients
            .iter()
            .position(|r| r.actor == actor && r.session == session);
        let Some(index) = index else {
            return Ok(Vec::new());
        };
        let recipient = self.recipients.remove(index);
        if !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Ok(Vec::new());
        }
        let at = runtime.read_actor_position(actor)?.position();
        let mut payloads = Vec::new();
        if recipient.dropped {
            payloads.push(encode_chat_line(&ChatLine::Dropped)?);
        }
        payloads.extend(
            recipient
                .lines
                .into_iter()
                .filter(|(from, bytes)| {
                    oteryn_protocol_oteryn::chat::decode_chat_line(bytes)
                        .is_ok_and(|line| callback_audible(&line, *from, at))
                })
                .map(|(_, bytes)| bytes),
        );
        Ok(payloads)
    }
    pub(crate) fn clear_session(&mut self, session: GameSessionId) {
        self.recipients.retain(|r| r.session != session);
    }
}
impl WeakSpotSpeechMailbox {
    /// Fixed source line already captured under current source occurrence/fence. Recipient
    /// controls, session and hearing are read again; sealed speaker payload survives removal.
    pub(crate) fn publish_callback(
        &mut self,
        runtime: &ChannelRuntimeV1,
        line: &crate::creature_damage_spell::CallbackSpeech,
        candidates: &[(ExactActorRef, GameSessionId)],
    ) -> Result<usize, WeakSpotSpeechError> {
        let from = line.position();
        let payload = encode_chat_line(line.line())?;
        let mut count = 0;
        let mut seen = Vec::new();
        self.recipients.retain(|r| {
            runtime
                .player_control_facts(r.actor, r.session)
                .is_ok_and(|f| f.control_loss.is_none())
        });
        for &(actor, session) in candidates {
            if seen.contains(&(actor, session)) {
                continue;
            }
            seen.push((actor, session));
            if !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|f| f.control_loss.is_none())
            {
                continue;
            }
            let Ok(at) = runtime.read_actor_position(actor) else {
                continue;
            };
            if !callback_audible(line.line(), from, at.position()) {
                continue;
            }
            let index = match self
                .recipients
                .iter()
                .position(|r| r.actor == actor && r.session == session)
            {
                Some(i) => i,
                None => {
                    self.recipients.push(Recipient {
                        actor,
                        session,
                        lines: VecDeque::new(),
                        dropped: false,
                    });
                    self.recipients.len() - 1
                }
            };
            let r = &mut self.recipients[index];
            if r.lines.len() >= PENDING_LINES {
                r.dropped = true
            }
            let maximum = if r.dropped {
                PENDING_LINES - 1
            } else {
                PENDING_LINES
            };
            while r.lines.len() >= maximum {
                r.lines.pop_front();
            }
            r.lines.push_back((from, payload.clone()));
            count += 1;
        }
        Ok(count)
    }
}
fn callback_audible(
    line: &ChatLine,
    from: MovementLocalPosition,
    to: MovementLocalPosition,
) -> bool {
    let ChatLine::Local { mode, .. } = line else {
        return false;
    };
    let mode = match mode {
        ChatSpeechMode::Yell => SpeechMode::Yell,
        _ => SpeechMode::Say,
    };
    let (Ok(a), Ok(b)) = (u8::try_from(from.floor), u8::try_from(to.floor)) else {
        return false;
    };
    hears(
        mode,
        ChatPosition {
            x: from.x,
            y: from.y,
            floor: a,
        },
        ChatPosition {
            x: to.x,
            y: to.y,
            floor: b,
        },
    )
    .is_some()
}

fn audible(speaker: MovementLocalPosition, listener: MovementLocalPosition) -> bool {
    let (Ok(speaker_floor), Ok(listener_floor)) =
        (u8::try_from(speaker.floor), u8::try_from(listener.floor))
    else {
        return false;
    };
    hears(
        SpeechMode::Say,
        ChatPosition {
            x: speaker.x,
            y: speaker.y,
            floor: speaker_floor,
        },
        ChatPosition {
            x: listener.x,
            y: listener.y,
            floor: listener_floor,
        },
    )
    .is_some()
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn protocol_line_preserves_exact_source_text_identity_and_generation() {
        let line = ChatLine::Local {
            speaker: ChatSpeaker {
                identity: [1; 16],
                generation: NonZeroU64::new(7).expect(
                    "weak_spot_speech.rs:tests:320: qualified fixture operation must succeed",
                ),
            },
            speaker_name: "Weak Spot".into(),
            mode: ChatSpeechMode::Say,
            text: "The weak spot of the gates crashes!".into(),
            position: ActorPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        };
        let bytes = encode_chat_line(&line)
            .expect("weak_spot_speech.rs:tests:331: qualified fixture operation must succeed");
        assert_eq!(
            oteryn_protocol_oteryn::chat::decode_chat_line(&bytes)
                .expect("weak_spot_speech.rs:tests:333: qualified fixture operation must succeed"),
            line
        );
    }
    #[test]
    fn exact_chat_range_and_floor_gate() {
        let p = MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        assert!(audible(
            p,
            MovementLocalPosition {
                x: 108,
                y: 106,
                floor: 7
            }
        ));
        assert!(!audible(
            p,
            MovementLocalPosition {
                x: 109,
                y: 106,
                floor: 7
            }
        ));
        assert!(!audible(
            p,
            MovementLocalPosition {
                x: 108,
                y: 106,
                floor: 8
            }
        ));
    }
}
/// Native visual outputs retain exact source assets/positions. No invented protocol frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WelterVisualCue {
    pub(crate) at: MovementLocalPosition,
    pub(crate) asset_binding: &'static str,
}
impl WeakSpotSpeechMailbox {
    /// Consume only the source-bound real native owner receipt, then reuse actual CHAT wire
    /// and egress queue. No caller-supplied text, positions or claimed healing result.
    pub(crate) fn publish_welter_consume(
        &mut self,
        runtime: &ChannelRuntimeV1,
        ledger: &crate::foundation::WelterConsumeLedger,
        registration: &crate::foundation::WelterSourceRegistration,
        caster: ExactActorRef,
        sequence: u64,
        candidates: &[(ExactActorRef, GameSessionId)],
    ) -> Result<Option<(usize, [WelterVisualCue; 2])>, WelterSpeechError> {
        let Some((generation, result)) =
            runtime.welter_committed_consume(ledger, registration, caster, sequence)?
        else {
            return Ok(None);
        };
        let Some((_, prey)) = result.consumed else {
            return Ok(None);
        };
        if let Some((_, old)) = self
            .welter_delivered
            .iter()
            .find(|(actor, _)| *actor == caster)
            && *old >= sequence
        {
            return Ok(None);
        }
        let at = result.caster_position;
        let text = "<the welter devours his spawn and heals himself>";
        debug_assert!(ChatText::parse(text).is_ok());
        let line = ChatLine::Local {
            speaker: ChatSpeaker {
                identity: caster.placement_identity(),
                generation: NonZeroU64::new(generation).ok_or(
                    crate::foundation::WelterConsumeError::Carrier(
                        CarrierError::StaleActorGeneration,
                    ),
                )?,
            },
            speaker_name: "The Welter".to_owned(),
            mode: ChatSpeechMode::Say,
            text: text.to_owned(),
            position: ActorPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        };
        let prey_at = result
            .prey_position
            .ok_or(crate::foundation::WelterConsumeError::MissingPosition)?;
        let payload = encode_chat_line(&line)?;
        self.welter_delivered
            .retain(|(actor, _)| runtime.contains_live_creature(*actor));
        self.recipients.retain(|r| {
            runtime
                .player_control_facts(r.actor, r.session)
                .is_ok_and(|f| f.control_loss.is_none())
        });
        let mut added = 0;
        let mut seen = Vec::new();
        for &(player, session) in candidates {
            if !runtime
                .player_control_facts(player, session)
                .is_ok_and(|f| f.control_loss.is_none())
                || seen.contains(&(player, session))
            {
                continue;
            }
            seen.push((player, session));
            let Ok(position) = runtime.read_actor_position(player) else {
                continue;
            };
            if !audible(at, position.position()) {
                continue;
            }
            let index = match self
                .recipients
                .iter()
                .position(|r| r.actor == player && r.session == session)
            {
                Some(i) => i,
                None => {
                    self.recipients.push(Recipient {
                        actor: player,
                        session,
                        lines: VecDeque::new(),
                        dropped: false,
                    });
                    self.recipients.len() - 1
                }
            };
            let receiver = &mut self.recipients[index];
            if receiver.lines.len() >= PENDING_LINES {
                receiver.dropped = true
            }
            let maximum = if receiver.dropped {
                PENDING_LINES - 1
            } else {
                PENDING_LINES
            };
            while receiver.lines.len() >= maximum {
                receiver.lines.pop_front();
            }
            receiver.lines.push_back((at, payload.clone()));
            added += 1;
        }
        if let Some((_, old)) = self
            .welter_delivered
            .iter_mut()
            .find(|(actor, _)| *actor == caster)
        {
            *old = sequence
        } else {
            self.welter_delivered.push((caster, sequence))
        }
        let prey_asset = match prey {
            crate::foundation::WelterPrey::Egg => "canary.appearance:effect/hitbypoison",
            crate::foundation::WelterPrey::Spawn => "canary.appearance:effect/drawblood",
        };
        Ok(Some((
            added,
            [
                WelterVisualCue {
                    at: prey_at,
                    asset_binding: prey_asset,
                },
                WelterVisualCue {
                    at,
                    asset_binding: "canary.appearance:effect/magic_blue",
                },
            ],
        )))
    }
}
#[derive(Debug)]
pub(crate) enum WelterSpeechError {
    Owner(crate::foundation::WelterConsumeError),
    Wire(ChatWireError),
}
impl From<crate::foundation::WelterConsumeError> for WelterSpeechError {
    fn from(e: crate::foundation::WelterConsumeError) -> Self {
        Self::Owner(e)
    }
}
impl From<ChatWireError> for WelterSpeechError {
    fn from(e: ChatWireError) -> Self {
        Self::Wire(e)
    }
}
