//! Mazarius dialogue uses the shared, generation-bound quest conversation owner.
//! The unqualified essence exchange is an explicit refusal with no state changes.
//! Source: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/npc/mazarius.lua:62-108,189-194.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::ferumbras_ascension::{Dialogue, NPC};
use std::collections::BTreeMap;

const ACCESS: &str = "oteryn:quest-progress/quest/u10_90/ferumbras_ascension/access";

pub(super) enum Prepared {
    Dialogue(Step),
    ExchangeUnavailable,
}

enum Selection {
    Dialogue(Choice),
    ExchangeUnavailable,
}

fn source() -> Option<&'static Dialogue> {
    static DATA: std::sync::OnceLock<Result<Dialogue, String>> = std::sync::OnceLock::new();
    DATA.get_or_init(Dialogue::load).as_ref().ok()
}

// Crystal data/npclib/npc_system/modules.lua:5,379-388: FocusModule
// uses both word boundaries, including literal '+', unlike MsgContains.
fn source_greeting(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    ["hi", "hello"].into_iter().any(|word| {
        let offsets: Vec<_> = message
            .match_indices(word)
            .map(|(offset, _)| offset)
            .collect();
        !offsets.is_empty()
            && offsets.into_iter().all(|offset| {
                let guarded = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'+';
                offset
                    .checked_sub(1)
                    .and_then(|previous| message.as_bytes().get(previous))
                    .is_none_or(|byte| !guarded(*byte))
                    && message
                        .as_bytes()
                        .get(offset + word.len())
                        .is_none_or(|byte| !guarded(*byte))
            })
    })
}

fn select(
    dialogue: &Dialogue,
    npc: &str,
    message: &str,
    topic: &Topic,
    fresh_conversation: bool,
    tracks: &BTreeMap<String, i64>,
) -> Option<Selection> {
    if npc != NPC {
        return None;
    }
    if fresh_conversation && source_greeting(message) {
        // A fresh common topic may follow a public greeting. Only an actual
        // source greeting gets greet text; an initial keyword must still run.
        return Some(Selection::Dialogue(Choice {
            transition: None,
            next_topic: Some(Topic::default()),
            reply: vec![dialogue.greet.clone()],
        }));
    }
    if !fresh_conversation && topic.value.is_some() {
        return None;
    }
    let source_topic = if fresh_conversation {
        0
    } else {
        u8::try_from(topic.number).ok()?
    };
    let access = tracks.get(ACCESS).copied().unwrap_or(-1);
    let branch = dialogue.choose(message, source_topic, access)?;
    if branch.exchange.is_some() {
        // Crystal mazarius.lua:97-108 requests one atomic inventory/state exchange.
        // Its owner is Left out by ANSWER section 5; do not show either guessed
        // inventory refusal or success, and do not reset source topic 3 to 0.
        // https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728
        return Some(Selection::ExchangeUnavailable);
    }
    Some(Selection::Dialogue(Choice {
        transition: None,
        next_topic: branch.next_topic.map(|next| Topic::from(u16::from(next))),
        reply: branch.reply.clone(),
    }))
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Prepared> {
    let dialogue = source()?;
    let mut exchange_unavailable = false;
    let step = quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, fresh, tracks| match select(
            dialogue,
            &context.2,
            &intent.text,
            topic,
            fresh,
            tracks,
        )? {
            Selection::Dialogue(choice) => Some(choice),
            Selection::ExchangeUnavailable => {
                exchange_unavailable = true;
                None
            }
        },
    );
    match step {
        Some(step) => Some(Prepared::Dialogue(step)),
        None if exchange_unavailable => Some(Prepared::ExchangeUnavailable),
        None => None,
    }
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    prepared: Prepared,
) -> NpcTalkOutcome {
    match prepared {
        Prepared::Dialogue(step) => {
            quest_dialogue::apply(admission, actor, command, intent, step).await
        }
        Prepared::ExchangeUnavailable => NpcTalkOutcome::only(NpcIntentDisposition::Rejected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_choice(selected: Option<Selection>) -> Result<Choice, String> {
        match selected {
            Some(Selection::Dialogue(choice)) => Ok(choice),
            _ => Err("expected source dialogue".into()),
        }
    }

    fn accept_topic(topic: &mut Topic, choice: Choice) {
        if let Some(next) = choice.next_topic {
            *topic = next;
        }
    }

    #[test]
    fn ferumbras_caller_source_workflow_greets_preserves_topic_and_refuses_exchange()
    -> Result<(), String> {
        let dialogue = Dialogue::load()?;
        let tracks = BTreeMap::new();
        let mut topic = Topic::from(3);
        let greet = text_choice(select(&dialogue, NPC, "hi", &topic, true, &tracks))?;
        assert_eq!(greet.reply, vec![dialogue.greet.clone()]);
        assert!(greet.transition.is_none());
        accept_topic(&mut topic, greet);
        assert_eq!(topic, Topic::default());

        let short = text_choice(select(&dialogue, NPC, "short", &topic, false, &tracks))?;
        assert_eq!(short.reply.len(), 2);
        accept_topic(&mut topic, short);
        assert_eq!(topic, Topic::from(2));

        let yes = text_choice(select(&dialogue, NPC, "yes", &topic, false, &tracks))?;
        assert_eq!(
            yes.reply,
            vec![
                "Good!, but I need 30 {demonic essences} to exchange with the demonic messenger for a ticket for you to enter the Abodes of Torments."
            ]
        );
        assert!(yes.transition.is_none());
        assert!(yes.next_topic.is_none());
        accept_topic(&mut topic, yes);
        assert_eq!(topic, Topic::from(2));

        let essence = text_choice(select(&dialogue, NPC, "essence", &topic, false, &tracks))?;
        assert_eq!(
            essence.reply,
            vec!["Do you have 30 demonic essences to offer to the demonic messenger?"]
        );
        accept_topic(&mut topic, essence);
        assert_eq!(topic, Topic::from(3));

        assert!(matches!(
            select(&dialogue, NPC, "yes", &topic, false, &tracks),
            Some(Selection::ExchangeUnavailable)
        ));
        assert_eq!(topic, Topic::from(3));
        assert!(tracks.is_empty());
        Ok(())
    }

    #[test]
    fn ferumbras_caller_reads_the_admitted_access_track_without_rewriting_it() -> Result<(), String>
    {
        let dialogue = Dialogue::load()?;
        let tracks = BTreeMap::from([(ACCESS.to_owned(), 1)]);
        let choice = text_choice(select(
            &dialogue,
            NPC,
            "yes",
            &Topic::from(2),
            false,
            &tracks,
        ))?;
        assert_eq!(choice.reply, vec!["What?!"]);
        assert_eq!(choice.next_topic, Some(Topic::default()));
        assert!(choice.transition.is_none());
        assert_eq!(tracks.get(ACCESS), Some(&1));
        assert!(matches!(
            select(&dialogue, NPC, "yes", &Topic::from(3), false, &tracks),
            Some(Selection::ExchangeUnavailable)
        ));
        Ok(())
    }

    #[test]
    fn ferumbras_caller_preserves_source_keyword_order_and_full_paragraphs() -> Result<(), String> {
        let dialogue = Dialogue::load()?;
        let tracks = BTreeMap::new();
        let choice = text_choice(select(
            &dialogue,
            NPC,
            "yes essence",
            &Topic::from(2),
            false,
            &tracks,
        ))?;
        assert!(choice.next_topic.is_none());
        assert!(choice.reply[0].starts_with("Good!, but I need 30"));
        let long = text_choice(select(
            &dialogue,
            NPC,
            "long",
            &Topic::default(),
            false,
            &tracks,
        ))?;
        assert_eq!(long.reply.len(), 14);
        assert_eq!(
            long.reply,
            dialogue.choose("long", 0, -1).ok_or("source long")?.reply
        );
        assert!(long.transition.is_none());
        Ok(())
    }

    #[test]
    fn ferumbras_caller_never_selects_another_npc_or_a_named_table_topic() -> Result<(), String> {
        let dialogue = Dialogue::load()?;
        let tracks = BTreeMap::new();
        assert!(
            select(
                &dialogue,
                "oteryn:npc.ezebeth",
                "hi",
                &Topic::default(),
                true,
                &tracks,
            )
            .is_none()
        );
        let held = Topic {
            number: 3,
            value: Some("foreign-table".into()),
        };
        assert!(select(&dialogue, NPC, "yes", &held, false, &tracks).is_none());
        Ok(())
    }
    #[test]
    fn ferumbras_first_private_keyword_after_public_greeting_is_not_swallowed() -> Result<(), String>
    {
        let dialogue = Dialogue::load()?;
        let tracks = BTreeMap::new();
        // A public "hi" has opened the actual conversation, but no command7
        // selector has initialized its topic yet. Source begins at topic0.
        let stale = Topic {
            number: 3,
            value: Some("earlier-conversation".into()),
        };
        let brings = text_choice(select(&dialogue, NPC, "brings", &stale, true, &tracks))?;
        assert_eq!(
            brings.reply,
            dialogue.choose("brings", 0, -1).ok_or("brings")?.reply
        );
        assert_eq!(brings.next_topic, Some(Topic::default()));
        assert!(brings.transition.is_none());
        let short = text_choice(select(&dialogue, NPC, "short", &stale, true, &tracks))?;
        assert_eq!(short.reply.len(), 2);
        assert_eq!(short.next_topic, Some(Topic::from(2)));
        let yes = text_choice(select(&dialogue, NPC, "yes", &stale, true, &tracks))?;
        assert_eq!(
            yes.reply,
            dialogue.choose("yes", 0, -1).ok_or("unprepared yes")?.reply
        );
        assert!(yes.transition.is_none());
        assert!(source_greeting("hello"));
        assert!(source_greeting("HI, Mazarius"));
        for text in ["high", "this", "hi+", "+hi", "brings", "short"] {
            assert!(!source_greeting(text), "{text}");
        }
        Ok(())
    }
}
