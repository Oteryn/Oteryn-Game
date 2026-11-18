//! Crystal 201189f8 data-global/npc/halvar.lua:70-115 and
//! scripts/quests/svargrond_arena/movements_arena_enter.lua:3-40.
use super::*;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::MovementPositionSnapshot;
use crate::gameplay_transport::{
    connection::UseCommand,
    npc_command::{NpcConversations, NpcTalkOutcome},
};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{ActorKind, Callback, Trigger};
use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const ACTION: u16 = 25100;
const HALVAR: &str = "oteryn:npc.halvar";

pub(super) struct Prompt {
    npc: EntityRef,
    context: (u64, u64, String),
    rank: u8,
}

enum Contact<'a> {
    Halvar(&'a NpcTalkIntent, &'a (u64, u64, String)),
    Entrance(MovementPositionSnapshot, u64),
}

pub(in crate::gameplay_transport) fn owns_callback(callback: &Callback) -> bool {
    callback.quest == "oteryn:quest.barbarian_arena_quest"
        && callback.trigger == Trigger::StepIn
        && callback.actor_kind == ActorKind::Player
        && callback.selector.action == Some(ACTION)
        && callback.selector.native.is_some()
        && callback.effects.is_empty()
        && !callback.has_state()
        && callback.teleport.is_none()
        && callback.passages.is_empty()
}

fn has_word(text: &str, words: &[&str]) -> bool {
    text.split(|c: char| !c.is_alphanumeric())
        .any(|word| words.iter().any(|w| word.eq_ignore_ascii_case(w)))
}

fn asks_to_fight(text: &str) -> bool {
    has_word(text, &["fight", "pit", "challenge", "arena"])
}

pub(super) fn entrance_source(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    position: MovementLocalPosition,
) -> Option<u64> {
    if !matches!(position.floor, 6 | 7)
        || !(32220..=32260).contains(&position.x)
        || !(31080..=31105).contains(&position.y)
    {
        return None;
    }
    let map = owner.bundle_map()?;
    let at = super::super::quest_callbacks::native(position)?;
    let tile = map.overlay.base().tile(at.x, at.y, at.floor)?;
    let mut sources = (0..tile.depths().iter().filter(|depth| **depth == 0).count())
        .filter_map(|ordinal| {
            oteryn_world_bundle::bundle::placement_key(
                at.floor,
                at.x,
                at.y,
                ordinal.try_into().ok()?,
            )
        })
        .filter(|key| {
            !map.facts.base_hidden(*key)
                && owner.callback_content().iter().any(|callback| {
                    owns_callback(callback)
                        && super::super::quest_callbacks::matches(map, callback, *key)
                })
        });
    let source = sources.next()?;
    sources.next().is_none().then_some(source)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn halvar_has_offer(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTalkIntent,
        runtime: &ChannelRuntimeV1,
    ) -> bool {
        self.callback_content().iter().any(owns_callback)
            && self
                .states
                .npc_conversations
                .current_npc(&self.states.chat, runtime, actor, session, intent)
                .is_some_and(|context| {
                    context.2 == HALVAR
                        && (asks_to_fight(&intent.text)
                            || self.states.arena_matches.halvar_prompts.lock().is_ok_and(
                                |prompts| {
                                    prompts.get(&session).is_some_and(|p| {
                                        p.context == context && p.npc == intent.npc_actor
                                    })
                                },
                            ))
                })
    }

    pub(in crate::gameplay_transport) async fn halvar_talk(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: NpcTalkIntent,
    ) -> NpcTalkOutcome {
        let reply = |line: &str| NpcConversations::bank_reply(&intent, line.into());
        let session = command.game_session_id;
        let Some(context) = ({
            let runtime = self.runtime.lock().await;
            self.states.npc_conversations.current_npc(
                &self.states.chat,
                &runtime,
                actor,
                session,
                &intent,
            )
        }) else {
            return reply("Sorry, not possible.");
        };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return reply("Sorry, not possible.");
        };
        if context.2 != HALVAR || command.item_fence != Some(fence.item_fence()) {
            return reply("Sorry, not possible.");
        }
        let Ok(command_id) = CommandId::new(command.command_id) else {
            return reply("Sorry, not possible.");
        };
        let cause = CommandRef::new(session, command_id);
        if asks_to_fight(&intent.text) {
            let Ok(payload) = wire::encode_match_request(A::Status) else {
                return reply("Sorry, not possible.");
            };
            let Ok(read) = self
                .root
                .read_arena_match(self.character, self.holder, fence, cause, &payload)
                .await
            else {
                return reply("Sorry, not possible.");
            };
            if read.state.paid_rank > 0 || matches!(read.state.phase, 1 | 2) {
                return reply("You already paid the fee, go and fight!");
            }
            let rank = read.state.completed + 1;
            let Some((name, fee)) = ["Greenhorn", "Scrapper", "Warlord"]
                .get(usize::from(rank - 1))
                .zip(donor::FEES.get(usize::from(rank - 1)))
            else {
                return reply("You've already completed the arena in all {difficulty levels}.");
            };
            let Ok(mut prompts) = self.states.arena_matches.halvar_prompts.lock() else {
                return reply("Sorry, not possible.");
            };
            prompts.insert(
                session,
                Prompt {
                    npc: intent.npc_actor,
                    context,
                    rank,
                },
            );
            return reply(&format!(
                "So you agree with the {{rules}} and want to participate in the {{challenge}}? The {{fee}} for one try in {{{name}}} is {fee} gold pieces. Do you really want to participate and pay the {{fee}}?"
            ));
        }
        let prompt = self
            .states
            .arena_matches
            .halvar_prompts
            .lock()
            .ok()
            .and_then(|mut prompts| prompts.remove(&session));
        let Some(prompt) = prompt.filter(|p| p.context == context && p.npc == intent.npc_actor)
        else {
            return reply("Sorry, not possible.");
        };
        if !has_word(&intent.text, &["yes"]) {
            return reply("Come back when you are ready then.");
        }
        let Some(proof) = durable::NativeArenaAdmission::payment(actor, cause, prompt.rank) else {
            return reply("Sorry, not possible.");
        };
        let contact = Contact::Halvar(&intent, &context);
        match self
            .settle_arena_admission(None, actor, proof, &contact)
            .await
        {
            Some(D::Applied) => {
                let mut outcome = reply(
                    "As you wish! You can pass the door now and enter the teleporter to the pits.",
                );
                outcome.inventory_changed = true;
                outcome
            }
            Some(D::InsufficientGold) => reply("You do not have enough money."),
            _ => reply("Sorry, not possible."),
        }
    }

    pub(in crate::gameplay_transport) async fn step_arena_entrance(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> Option<MovementPositionSnapshot> {
        if origin.position() == entered.position() {
            return None;
        }
        let map = self.bundle_map()?;
        let source = entrance_source(self, entered.position())?;
        let proof = durable::NativeArenaAdmission::entry(
            actor,
            session,
            entered,
            source,
            self.reset_epoch,
        )?;
        if self
            .settle_arena_admission(
                Some(permit),
                actor,
                proof,
                &Contact::Entrance(entered, source),
            )
            .await
            == Some(D::Applied)
        {
            let landing = self.runtime.lock().await.read_actor_position(actor).ok()?;
            if landing.position() != entered.position()
                && let Some(at) = super::super::quest_callbacks::native(landing.position())
            {
                // movements_arena_enter.lua:32-34 sends MAGIC_RED and announces the fight.
                map.facts.emit_movement_effect(at, 14);
                let _ = map
                    .facts
                    .emit_movement_speech(crate::map::facts::MovementSpeech {
                        speaker: actor,
                        position: at,
                        talk_type: 36,
                        text: "FIGHT!".into(),
                        captured_name: None,
                    });
            }
            return Some(landing);
        }
        let mut runtime = self.runtime.lock().await;
        if runtime.read_actor_position(actor) == Ok(entered)
            && walkable(self, &runtime, origin.position())
        {
            runtime
                .relocate_arena_player(actor, session, entered, origin.position())
                .ok()?;
        }
        runtime.read_actor_position(actor).ok()
    }

    async fn settle_arena_admission(
        &self,
        permit: Option<&SpellLanePermit>,
        actor: ExactActorRef,
        proof: durable::NativeArenaAdmission,
        contact: &Contact<'_>,
    ) -> Option<D> {
        let command = proof.command();
        let session = command.game_session_id();
        let paying = proof.is_payment();
        let _gate = arena_command_gate(&self.states.arena_matches.command, !paying).await?;
        let _payment_lane = if paying {
            Some(self.spell_lane_permit().await?)
        } else {
            None
        };
        let mut fence = self.current_quest_fence(session).await.ok()??;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        fence.expected_character_revision = slot.cursor(self.root, self.character).await.ok()?;
        let origin = durable::NativeArenaOrigin::Admission(proof);
        let read = self
            .root
            .read_arena_native_origin(self.character, self.holder, fence, &origin)
            .await
            .ok()?;
        if let Some(replay) = read.replay {
            return wire::decode_match_result(&replay)
                .ok()
                .map(|result| result.disposition);
        }
        let before = read.state;
        if !paying
            && !read.dead
            && read.pit_owned
            && before.phase == 1
            && before.pit == 1
            && before.session == *session.as_bytes()
            && before.scope_generation == fence.scope_ownership_generation.get()
        {
            pit_reset::reset_ground(self, permit?, actor, fence, &before).await?;
            self.restore_arena_projection(actor, session, &before, read.now_ms)
                .await?;
            return Some(D::Applied);
        }
        if read.dead || before.phase != 0 || before.rank != 0 || before.completed >= 3 {
            return Some(D::WrongState);
        }
        let rank = before.completed + 1;
        let mut after = before.clone();
        let mut change = None;
        let backpack = if paying {
            self.root
                .read_character_backpack(self.character, fence.character_id)
                .await
                .ok()?
        } else {
            None
        };
        {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || !runtime
                    .bound_attacker_lease(actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
                || self.states.spell_states.try_lock().ok()?.is_dead(actor)
            {
                return None;
            }
            match contact {
                Contact::Halvar(intent, context) => {
                    if before.paid_rank != 0
                        || self
                            .states
                            .npc_conversations
                            .current_npc(&self.states.chat, &runtime, actor, session, intent)
                            .as_ref()
                            != Some(*context)
                    {
                        return Some(D::WrongState);
                    }
                    change = Some(super::super::charm_progression::fee_change(
                        self,
                        &runtime,
                        backpack.as_ref(),
                        [
                            durable::identity(command, b"change-platinum"),
                            durable::identity(command, b"change-gold"),
                        ],
                    )?);
                    after.paid_rank = rank;
                }
                Contact::Entrance(snapshot, source) => {
                    let map = self.bundle_map()?;
                    if before.paid_rank != rank
                        || runtime.read_actor_position(actor) != Ok(*snapshot)
                        || map.facts.base_hidden(*source)
                        || !self.callback_content().iter().any(|callback| {
                            owns_callback(callback)
                                && super::super::quest_callbacks::matches(map, callback, *source)
                        })
                    {
                        return Some(D::WrongState);
                    }
                    if !self.arena_pit_ready(&runtime, rank, 1, session) {
                        return Some(D::Busy);
                    }
                    after = MatchState {
                        completed: before.completed,
                        entry_revision: fence.expected_character_revision.get(),
                        paid_rank: 0,
                        rank,
                        pit: 1,
                        phase: 1,
                        deadline_ms: read.now_ms + self.states.arena_matches.pit_millis(),
                        session: *session.as_bytes(),
                        scope_generation: fence.scope_ownership_generation.get(),
                        run: durable::identity(command, b"run"),
                    };
                }
            }
        }
        let bytes = slot
            .commit_arena_match(
                self.root,
                self.character,
                self.holder,
                fence,
                MatchTransition {
                    cause: command.into(),
                    native_origin: Some(origin),
                    payload: wire::encode_match_request(A::Enter).ok()?,
                    before,
                    after: after.clone(),
                    disposition: D::Applied,
                    change,
                    victory: None,
                    achievement: None,
                },
            )
            .await
            .ok()?;
        let result = wire::decode_match_result(&bytes).ok()?;
        if !paying && result.disposition == D::Applied {
            pit_reset::reset_ground(self, permit?, actor, fence, &after).await?;
            self.restore_arena_projection(actor, session, &after, read.now_ms)
                .await?;
        }
        Some(result.disposition)
    }
}
