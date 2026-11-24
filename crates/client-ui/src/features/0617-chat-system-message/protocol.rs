//! Typed feedback identifies system messages used by the party panel.
use oteryn_client_runtime::chat::ChatLine;
pub fn feedback(line: &ChatLine, typed: bool) -> Option<&str> {
    match line {
        ChatLine::System { text } => Some(text),
        ChatLine::Private { speaker_name, text } if !typed && speaker_name == "Game" => Some(text),
        _ => None,
    }
}

#[cfg(test)]
#[test]
fn observations() {
    let line = ChatLine::Private {
        speaker_name: "Game".into(),
        text: "message".into(),
    };
    assert_eq!(feedback(&line, true), None);
    assert_eq!(feedback(&line, false), Some("message"));
}
