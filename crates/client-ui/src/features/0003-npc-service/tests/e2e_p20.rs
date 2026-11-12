//! P20 replay: the real King Tibianus promotion dialogue payloads applied through the production
//! `DialoguePanel::request` and `DialoguePanel::apply` (conversation decoded by
//! `npc_service::decode_npc_conversation`, exactly as the client transport does).
use super::*;

type Error = Box<dyn std::error::Error>;

struct Replay {
    greeting: Vec<u8>,
    quote: Vec<u8>,
    confirm: Vec<u8>,
    expected: String,
}

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, Error> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn identity(text: &str) -> Result<[u8; 16], Error> {
    if text.len() != 32 {
        return Err("identity hex has the wrong length".into());
    }
    let mut out = [0u8; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)?;
    }
    Ok(out)
}

fn conversation(npc: EntityRef, lines: &[&str]) -> Result<Vec<u8>, Error> {
    npc_service::encode_npc_conversation(&npc_service::NpcConversation {
        open: Some((npc, npc_service::NpcConversationState::Open)),
        lines: lines.iter().map(|line| (*line).into()).collect(),
    })
    .map_err(|e| format!("{e:?}").into())
}

fn load() -> Result<Replay, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Replay {
            greeting: std::fs::read(root.join("p20-greeting.bin"))?,
            quote: std::fs::read(root.join("p20-quote.bin"))?,
            confirm: std::fs::read(root.join("p20-confirm.bin"))?,
            expected: std::fs::read_to_string(root.join("p20-expected.txt"))?,
        });
    }
    let npc = EntityRef {
        identity: [9; 16],
        generation: 4,
    };
    Ok(Replay {
        greeting: conversation(npc, &["Greetings, brave adventurer."])?,
        quote: conversation(npc, &["Do you want to be promoted for 20000 gold coins?"])?,
        confirm: conversation(npc, &["You need to be premium to be promoted."])?,
        expected: "npc_identity=09090909090909090909090909090909\nnpc_generation=4\ngreeting=Greetings, brave adventurer.\nquote=Do you want to be promoted for 20000 gold coins?\nconfirm=You need to be premium to be promoted.\n".into(),
    })
}

fn decode(bytes: &[u8]) -> Result<npc_service::NpcConversation, Error> {
    npc_service::decode_npc_conversation(bytes).map_err(|e| format!("{e:?}").into())
}

#[test]
fn p20_dialogue_replay() -> Result<(), Error> {
    let replay = load()?;
    let npc = EntityRef {
        identity: identity(value(&replay.expected, "npc_identity")?)?,
        generation: value(&replay.expected, "npc_generation")?.parse()?,
    };
    let mut panel = DialoguePanel {
        selected: true,
        ..DialoguePanel::default()
    };
    for (text, bytes, key) in [
        ("hi", &replay.greeting, "greeting"),
        ("promotion", &replay.quote, "quote"),
        ("yes", &replay.confirm, "confirm"),
    ] {
        let intent = panel.request(npc, text).ok_or("talk request refused")?;
        assert_eq!((intent.npc_actor, intent.text.as_str()), (npc, text));
        assert!(panel.pending);
        panel.apply(decode(bytes)?);
        let lines: Vec<&str> = value(&replay.expected, key)?.split('|').collect();
        assert_eq!(panel.lines, lines);
        assert_eq!(panel.text(), lines.join("\n"));
        assert!(!panel.pending);
        assert_eq!(panel.target, Some(npc));
    }

    // A refused talk keeps the last reply and shows the refusal; a corrupt payload never decodes.
    let shown = panel.lines.clone();
    assert!(panel.request(npc, "hi").is_some());
    panel.result(NpcIntentDisposition::NotInRange);
    assert_eq!(panel.feedback.as_deref(), Some("Talk refused: NotInRange"));
    assert_eq!(panel.lines, shown);
    assert!(panel.target.is_none());
    assert!(npc_service::decode_npc_conversation(&replay.quote[..replay.quote.len() - 1]).is_err());
    Ok(())
}
