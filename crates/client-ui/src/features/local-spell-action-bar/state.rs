pub use crate::actor_cooldown_deltas::state::ActionBarPanel;
use oteryn_client_runtime::item_effects::{
    ActorPosition, PresentationEventKind, PresentationSource,
};
use oteryn_client_runtime::spell::{HouseEditorList, ParameterSpellCastResult, PrivateSpellEffect};

/// Reduce a private completion to the action-bar status and the caster's local visual cue.
pub fn parameter_cast_feedback(
    result: &ParameterSpellCastResult,
    position: ActorPosition,
) -> (String, Option<PresentationEventKind>) {
    let (text, effect) = result.feedback.as_ref().map_or_else(
        || {
            (
                oteryn_client_runtime::spell::feedback_text(result.disposition),
                PrivateSpellEffect::None,
            )
        },
        |feedback| (feedback.text.as_str(), feedback.effect),
    );
    let text = if let Some(editor) = &result.editor {
        let label = match editor.list {
            HouseEditorList::Guest => "House guest list".into(),
            HouseEditorList::Subowner => "House subowner list".into(),
            HouseEditorList::Door(id) => format!("House door {id} list"),
        };
        format!("{label}\n{}", editor.text)
    } else {
        text.to_owned()
    };
    // Source appearance IDs from content/presentations/bindings/client-ids.json.
    let effect_id = match effect {
        PrivateSpellEffect::None => return (text, None),
        PrivateSpellEffect::Poff => 3,
        PrivateSpellEffect::MagicBlue => 13,
        PrivateSpellEffect::Teleport => 11,
    };
    (
        text,
        Some(PresentationEventKind::MagicEffect {
            position,
            effect_id,
            source: PresentationSource::Own,
        }),
    )
}
