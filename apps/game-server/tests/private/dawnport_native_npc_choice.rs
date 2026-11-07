//! Mount below gameplay_transport::npc_command for native owner tests; no copied production owner.
#![allow(clippy::expect_used)]
use super::*;
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;
use crate::gameplay_transport::vocation_choice::DAWNPORT_CHOICE_NPC;
use std::sync::Arc;

fn native_fixture(
    key: &str,
    distance: i32,
) -> (
    ChannelRuntimeV1,
    ExactActorRef,
    GameSessionId,
    EntityRef,
    ChatRuntime,
    NpcConversations,
) {
    let (mut runtime, actor, session) = runtime_with_capacity(0x72, 3);
    runtime
        .initialize_first_entry_position(actor)
        .expect("position");
    let at = runtime
        .read_actor_position(actor)
        .expect("position")
        .position();
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: key.into(),
                    placements: vec![NpcPlacement {
                        floor: -i8::try_from(at.floor).expect("floor"),
                        y: u16::try_from(at.y).expect("y"),
                        x: u16::try_from(at.x + distance).expect("x"),
                        direction: MovementFacing::South,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .expect("place native NPC");
    let placed = runtime.visible_entities().npcs[0];
    let npc = EntityRef {
        identity: placed.actor.placement_identity(),
        generation: placed.generation,
    };
    let chat = ChatRuntime::default();
    chat.admit(
        session,
        "Dawnport candidate",
        SemanticTimeMicros::from_micros(1),
    );
    chat.set_npc_service(session);
    // Oressa has a Dialogue, so no generated-reply entry exists.
    chat.install_npc_talk(Arc::new(super::super::npc_talk::NpcTalkBook::default()));
    (
        runtime,
        actor,
        session,
        npc,
        chat,
        NpcConversations::default(),
    )
}

fn say(
    f: &(
        ChannelRuntimeV1,
        ExactActorRef,
        GameSessionId,
        EntityRef,
        ChatRuntime,
        NpcConversations,
    ),
    text: &str,
    micros: u64,
) -> NpcTalkOutcome {
    f.5.talk_in(
        &f.4,
        &f.0,
        f.1,
        f.2,
        &NpcTalkIntent {
            npc_actor: f.3,
            text: text.into(),
        },
        SemanticTimeMicros::from_micros(micros),
        None,
    )
}

#[test]
fn real_npc_owner_requires_greeting_selection_and_confirmation() {
    let f = native_fixture(DAWNPORT_CHOICE_NPC, 2);
    assert_eq!(
        say(&f, "yes", 1).disposition,
        NpcIntentDisposition::NoConversation
    );
    assert!(f.5.dawnport_choice(f.2).is_none());
    assert_eq!(say(&f, "hi", 300_000).disposition, NpcIntentDisposition::Ok);
    assert_eq!(
        say(&f, "knight", 600_000).disposition,
        NpcIntentDisposition::Ok
    );
    assert!(f.5.dawnport_choice(f.2).is_none());
    assert_eq!(
        say(&f, "yes", 900_000).disposition,
        NpcIntentDisposition::Ok
    );
    let confirmed = f.5.dawnport_choice(f.2).expect("native confirmation");
    assert_eq!(confirmed.actor, f.1);
    assert_eq!(confirmed.npc, f.3);
    assert_eq!(confirmed.selected, "knight");
    assert!(
        confirmed.retained.is_none(),
        "dialogue itself is not a durable commit"
    );
    say(&f, "yes", 1_300_000);
    assert_eq!(
        f.5.dawnport_choice(f.2)
            .expect("retained choice")
            .occurrence,
        confirmed.occurrence
    );
}

#[test]
fn declining_a_topic_does_not_leave_a_confirmable_choice() {
    let f = native_fixture(DAWNPORT_CHOICE_NPC, 2);
    say(&f, "hi", 1);
    say(&f, "druid", 300_000);
    say(&f, "no", 600_000);
    assert_eq!(
        say(&f, "yes", 900_000).disposition,
        NpcIntentDisposition::Rejected
    );
    assert!(f.5.dawnport_choice(f.2).is_none());
}

#[test]
fn stale_npc_generation_cannot_confirm_the_previous_topic() {
    let f = native_fixture(DAWNPORT_CHOICE_NPC, 2);
    say(&f, "hi", 1);
    say(&f, "sorcerer", 300_000);
    let mut stale = f.3;
    stale.generation += 1;
    let outcome = f.5.talk_in(
        &f.4,
        &f.0,
        f.1,
        f.2,
        &NpcTalkIntent {
            npc_actor: stale,
            text: "yes".into(),
        },
        SemanticTimeMicros::from_micros(600_000),
        None,
    );
    assert_eq!(outcome.disposition, NpcIntentDisposition::Stale);
    assert!(f.5.dawnport_choice(f.2).is_none());
}

#[test]
fn remote_npc_and_unowned_session_do_not_create_a_choice() {
    let f = native_fixture(DAWNPORT_CHOICE_NPC, 5);
    assert_eq!(
        say(&f, "hi", 1).disposition,
        NpcIntentDisposition::NotInRange
    );
    assert!(f.5.dawnport_choice(f.2).is_none());
    let mut wrong_session = f.2.as_bytes().to_owned();
    wrong_session[15] ^= 1;
    let wrong_session = GameSessionId::decode(&wrong_session).expect("other session");
    let outcome = f.5.talk_in(
        &f.4,
        &f.0,
        f.1,
        wrong_session,
        &NpcTalkIntent {
            npc_actor: f.3,
            text: "hi".into(),
        },
        SemanticTimeMicros::from_micros(1),
        None,
    );
    assert_eq!(outcome.disposition, NpcIntentDisposition::Rejected);
    assert!(f.5.dawnport_choice(wrong_session).is_none());
}

#[test]
fn asking_about_monk_cannot_confirm_an_older_four_vocation_quote() {
    let f = native_fixture(DAWNPORT_CHOICE_NPC, 2);
    say(&f, "hi", 1);
    say(&f, "knight", 300_000);
    say(&f, "monk", 600_000);
    assert_eq!(
        say(&f, "yes", 900_000).disposition,
        NpcIntentDisposition::Rejected
    );
    assert!(f.5.dawnport_choice(f.2).is_none());
}
