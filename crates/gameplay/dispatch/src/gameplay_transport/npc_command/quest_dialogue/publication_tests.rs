//! Exercise final publication with the actual ChatRuntime engagement and NPC resolver.
use super::*;
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::sync::Arc;

struct Fixture {
    runtime: ChannelRuntimeV1,
    chat: ChatRuntime,
    conversations: NpcConversations,
    actor: ExactActorRef,
    session: GameSessionId,
    context: Context,
    intent: NpcTalkIntent,
}

impl Fixture {
    fn new() -> Result<Self, String> {
        let (mut runtime, actor, session) = runtime_with_capacity(0x72, 3);
        runtime
            .initialize_first_entry_position(actor)
            .map_err(|error| format!("{error:?}"))?;
        let at = runtime
            .read_actor_position(actor)
            .map_err(|error| format!("{error:?}"))?
            .position();
        runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: "oteryn:npc.ezebeth".into(),
                        placements: vec![NpcPlacement {
                            x: u16::try_from(at.x + 1).map_err(|error| format!("{error:?}"))?,
                            y: u16::try_from(at.y).map_err(|error| format!("{error:?}"))?,
                            floor: -i8::try_from(at.floor).map_err(|error| format!("{error:?}"))?,
                            direction: MovementFacing::South,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .map_err(|error| format!("{error:?}"))?;
        let npc = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|npc| runtime.npc_key(npc.actor) == Some("oteryn:npc.ezebeth"))
            .ok_or("source NPC fixture")?;
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|error| format!("{error:?}"))?;
        let chat = ChatRuntime::default();
        chat.install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
        chat.admit(
            session,
            "[|PLAYERNAME|] ${name} %s",
            SemanticTimeMicros::from_micros(1),
        );
        chat.set_npc_service(session);
        let conversations = NpcConversations::default();
        let intent = NpcTalkIntent {
            npc_actor: EntityRef {
                identity: npc.actor.placement_identity(),
                generation: npc.generation,
            },
            text: "hi".into(),
        };
        let greeting = conversations.talk_in(
            &chat,
            &runtime,
            actor,
            session,
            &intent,
            SemanticTimeMicros::from_micros(10),
            None,
        );
        assert_eq!(greeting.disposition, NpcIntentDisposition::Ok);
        let context = conversations
            .current_npc(&chat, &runtime, actor, session, &intent)
            .ok_or("actual source conversation")?;
        Ok(Self {
            runtime,
            chat,
            conversations,
            actor,
            session,
            context,
            intent,
        })
    }

    fn talk(&self, text: &str, at: u64) {
        let intent = NpcTalkIntent {
            npc_actor: self.intent.npc_actor,
            text: text.into(),
        };
        let outcome = self.conversations.talk_in(
            &self.chat,
            &self.runtime,
            self.actor,
            self.session,
            &intent,
            SemanticTimeMicros::from_micros(at),
            None,
        );
        assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{text}");
    }

    fn current(&self) -> Option<Context> {
        self.conversations.current_npc(
            &self.chat,
            &self.runtime,
            self.actor,
            self.session,
            &self.intent,
        )
    }

    fn rendered(&self) -> NpcTalkOutcome {
        reply(
            &self.chat,
            self.session,
            &self.context,
            &self.intent,
            vec!["Hello, |PLAYERNAME|.".into()],
        )
    }
}

fn assert_committed_without_conversation(outcome: NpcTalkOutcome) {
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
    assert!(!outcome.unknown, "a confirmed receipt is known");
    assert!(!outcome.inventory_changed);
    assert!(
        outcome.conversation.is_none(),
        "a stale receipt must not reopen a conversation"
    );
    assert!(outcome.window.is_none());
}

#[test]
fn committed_source_reply_keeps_the_current_conversation() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let outcome = publish_committed_reply(fixture.current(), &fixture.context, fixture.rendered());
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
    assert!(!outcome.unknown);
    let conversation = outcome.conversation.ok_or("current conversation")?;
    assert_eq!(
        conversation.open,
        Some((fixture.intent.npc_actor, NpcConversationState::Open))
    );
    assert_eq!(conversation.lines, ["Hello, |PLAYERNAME|."]);
    Ok(())
}

#[test]
fn committed_source_reply_does_not_reopen_a_closed_conversation() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let rendered_before_commit = fixture.rendered();
    fixture.talk("bye", 2_000_010);
    assert_eq!(fixture.current(), None);
    assert_committed_without_conversation(publish_committed_reply(
        fixture.current(),
        &fixture.context,
        rendered_before_commit,
    ));
    assert_eq!(fixture.chat.engaged(fixture.session), None);
    Ok(())
}

#[test]
fn committed_source_reply_does_not_target_a_new_conversation_generation() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let rendered_before_commit = fixture.rendered();
    fixture.talk("bye", 2_000_010);
    fixture.talk("hi", 4_000_010);
    let current = fixture.current().ok_or("new source conversation")?;
    assert_eq!(current.0, fixture.context.0);
    assert_eq!(current.2, fixture.context.2);
    assert_ne!(
        current.1, fixture.context.1,
        "real chat generation advanced"
    );
    assert_committed_without_conversation(publish_committed_reply(
        Some(current.clone()),
        &fixture.context,
        rendered_before_commit,
    ));
    assert_eq!(
        fixture.current(),
        Some(current),
        "the new conversation remains untouched"
    );
    Ok(())
}

#[test]
fn committed_source_reply_keeps_receipt_success_after_the_session_disappears() -> Result<(), String>
{
    let fixture = Fixture::new()?;
    fixture.chat.forget(fixture.session);
    let resumed_render = fixture.rendered();
    assert_eq!(resumed_render.disposition, NpcIntentDisposition::Rejected);
    assert_committed_without_conversation(publish_committed_reply(
        fixture.current(),
        &fixture.context,
        resumed_render,
    ));
    Ok(())
}
