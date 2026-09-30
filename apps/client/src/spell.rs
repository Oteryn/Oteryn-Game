//! Spell bar, feedback and vitals bars (SPELL wire contract section 9 step 3, client half).
//!
//! Pure client state over the session crate's types. Hotkeys arrive only as routed actions
//! (N3 `InputRouter`). While the server gate is closed every cast is `Rejected`, shown as
//! "spells unavailable"; no vitals arrive either, so no bar is drawn.

use crate::input::Targetable;
use oteryn_input_actions::{
    ActionId, ActionPhase, Binding, BindingMap, ContextDefinition, ContextId, ContextKind,
    InputAtom, InputChord, InputError, InputRouter, KeyCode, Modifiers, NormalizedInputEvent,
    RepeatPolicy,
};
use oteryn_renderer::{AtlasImage, BatchError, QuadInstance};
use oteryn_session::{
    ActorVitals, CastOutcome, Session, SessionError, SessionStream, SpellCastDisposition,
    SpellTarget,
};
use std::num::NonZeroU32;

/// Hotkeys `1` to `SPELL_SLOTS` cast spell-book index 1 to `SPELL_SLOTS` (USB HID codes 30..).
pub const SPELL_SLOTS: u16 = 4;
const FIRST_DIGIT_KEY: u16 = 30;
const ACTION_PREFIX: &str = "client.spell.";

/// Player-facing line for one cast result.
#[must_use]
pub const fn feedback_text(disposition: SpellCastDisposition) -> &'static str {
    match disposition {
        SpellCastDisposition::Cast => "Spell cast",
        SpellCastDisposition::CoolingDown => "Spell is cooling down",
        SpellCastDisposition::LevelTooLow => "Level too low",
        SpellCastDisposition::MagicLevelTooLow => "Magic level too low",
        SpellCastDisposition::NotEnoughMana => "Not enough mana",
        SpellCastDisposition::NotEnoughSoul => "Not enough soul",
        SpellCastDisposition::NotAvailable => "Spell not available",
        SpellCastDisposition::TargetRequired => "Select a target first",
        SpellCastDisposition::TargetIllegal => "Illegal target",
        SpellCastDisposition::Rejected => "Spells unavailable",
    }
}

/// Spell hotkeys routed through `InputRouter`; a text context suppresses them.
#[derive(Debug)]
pub struct SpellHotkeys {
    router: InputRouter,
    text: ContextId,
}

impl SpellHotkeys {
    pub fn new() -> Result<Self, InputError> {
        let gameplay = ContextId::new("spell-gameplay".to_owned())?;
        let text = ContextId::new("spell-text".to_owned())?;
        let mut bindings = Vec::new();
        for slot in 1..=SPELL_SLOTS {
            bindings.push(Binding::new(
                gameplay.clone(),
                InputChord::new(
                    Modifiers::NONE,
                    vec![InputAtom::Key(KeyCode::new(FIRST_DIGIT_KEY + slot - 1)?)],
                )?,
                ActionId::new(format!("{ACTION_PREFIX}{slot}"))?,
                RepeatPolicy::Ignore,
            ));
        }
        let map = BindingMap::new(
            vec![
                ContextDefinition::new(gameplay.clone(), ContextKind::Gameplay, 0),
                ContextDefinition::new(text.clone(), ContextKind::Text, 10),
            ],
            bindings,
            &[],
        )?;
        let mut router = InputRouter::new(map);
        router.set_context_active(&gameplay, true)?;
        Ok(Self { router, text })
    }

    pub fn set_text_active(&mut self, active: bool) -> Result<(), InputError> {
        self.router.set_context_active(&self.text, active).map(drop)
    }

    /// Spell-book indices whose hotkey started in `events`.
    pub fn route(&mut self, events: &[NormalizedInputEvent]) -> Vec<NonZeroU32> {
        events
            .iter()
            .flat_map(|event| self.router.process(event))
            .filter(|action| action.phase() == ActionPhase::Started)
            .filter_map(|action| {
                let slot = action.action().as_str().strip_prefix(ACTION_PREFIX)?;
                slot.parse().ok()
            })
            .collect()
    }
}

/// The last cast's feedback line, shown until the next cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpellFeedback {
    last: Option<SpellCastDisposition>,
}

impl SpellFeedback {
    pub const fn record(&mut self, disposition: SpellCastDisposition) {
        self.last = Some(disposition);
    }

    #[must_use]
    pub const fn text(&self) -> Option<&'static str> {
        match self.last {
            Some(disposition) => Some(feedback_text(disposition)),
            None => None,
        }
    }
}

/// Casts `spell` at the selected target (aimed) or without one, and records the disposition.
/// Vitals after a `Cast` are read from `Session::actor_vitals`.
pub async fn cast_selected<S: SessionStream>(
    session: &mut Session<S>,
    feedback: &mut SpellFeedback,
    spell: NonZeroU32,
    target: Option<Targetable>,
) -> Result<CastOutcome, SessionError> {
    let (intent, aim) = if target.is_some() {
        (SpellTarget::AttackTarget, true)
    } else {
        (SpellTarget::None, false)
    };
    let outcome = session.cast_spell(spell, intent, aim).await?;
    feedback.record(outcome.disposition);
    Ok(outcome)
}

pub const BAR_WIDTH_PX: f32 = 120.0;
pub const BAR_HEIGHT_PX: f32 = 10.0;
const BAR_GAP_PX: f32 = 4.0;

/// Fill width in pixels: `value / max` clamped to the bar, zero when `max` is zero.
#[must_use]
pub fn fill_px(value: u32, max: u32) -> f32 {
    if max == 0 {
        return 0.0;
    }
    // Vitals are small; f32 precision is ample for a 120 px bar.
    #[allow(clippy::cast_precision_loss)]
    let ratio = (value.min(max) as f32) / (max as f32);
    ratio * BAR_WIDTH_PX
}

/// Four quads at `origin` (pixels): health track and fill, then mana track and fill, drawn from
/// solid atlas cells (`track`, `health`, `mana`). Nothing without vitals.
pub fn vitals_bars(
    atlas: &AtlasImage,
    vitals: Option<&ActorVitals>,
    origin: [f32; 2],
    (track, health, mana): (u16, u16, u16),
) -> Result<Vec<QuadInstance>, BatchError> {
    let Some(vitals) = vitals else {
        return Ok(Vec::new());
    };
    let mut quads = Vec::with_capacity(4);
    for (row, (value, max, cell)) in [
        (vitals.health, vitals.max_health, health),
        (vitals.mana, vitals.max_mana, mana),
    ]
    .into_iter()
    .enumerate()
    {
        #[allow(clippy::cast_precision_loss)]
        let y = origin[1] + (row as f32) * (BAR_HEIGHT_PX + BAR_GAP_PX);
        quads.push(QuadInstance {
            position: [origin[0], y],
            size: [BAR_WIDTH_PX, BAR_HEIGHT_PX],
            uv: atlas.uv_rect(track)?,
        });
        quads.push(QuadInstance {
            position: [origin[0], y],
            size: [fill_px(value, max), BAR_HEIGHT_PX],
            uv: atlas.uv_rect(cell)?,
        });
    }
    Ok(quads)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_input_actions::ButtonState;
    use oteryn_placeholder_assets::{PlaceholderCell, placeholder_atlas};

    fn key(code: u16, state: ButtonState) -> Result<NormalizedInputEvent, InputError> {
        Ok(NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state,
            modifiers: Modifiers::NONE,
            repeat: false,
        })
    }

    #[test]
    fn digit_hotkeys_route_to_spell_book_indices() -> Result<(), InputError> {
        let mut hotkeys = SpellHotkeys::new()?;
        let pressed = hotkeys.route(&[key(31, ButtonState::Pressed)?]);
        assert_eq!(pressed.iter().map(|s| s.get()).collect::<Vec<_>>(), [2]);
        hotkeys.route(&[key(31, ButtonState::Released)?]);
        // Outside the bar (digit 5) and while text is active nothing routes.
        assert!(hotkeys.route(&[key(34, ButtonState::Pressed)?]).is_empty());
        hotkeys.route(&[key(34, ButtonState::Released)?]);
        hotkeys.set_text_active(true)?;
        assert!(hotkeys.route(&[key(30, ButtonState::Pressed)?]).is_empty());
        Ok(())
    }

    #[test]
    fn rejected_reads_spells_unavailable_and_others_are_distinct() {
        let mut feedback = SpellFeedback::default();
        assert_eq!(feedback.text(), None);
        feedback.record(SpellCastDisposition::Rejected);
        assert_eq!(feedback.text(), Some("Spells unavailable"));
        feedback.record(SpellCastDisposition::NotEnoughMana);
        assert_eq!(feedback.text(), Some("Not enough mana"));
    }

    #[test]
    fn fill_clamps_and_survives_zero_max() {
        assert!((fill_px(50, 100) - 60.0).abs() < f32::EPSILON);
        assert!((fill_px(500, 100) - BAR_WIDTH_PX).abs() < f32::EPSILON);
        assert!(fill_px(5, 0).abs() < f32::EPSILON);
    }

    #[test]
    fn bars_draw_track_and_fill_per_vital_and_nothing_without_vitals() -> Result<(), BatchError> {
        let source = placeholder_atlas();
        let atlas = AtlasImage::new(source.cell_px, source.columns, source.rows, source.rgba)?;
        let cells = (
            PlaceholderCell::Stone.index(),
            PlaceholderCell::Grass.index(),
            PlaceholderCell::Water.index(),
        );
        assert!(vitals_bars(&atlas, None, [0.0, 0.0], cells)?.is_empty());
        let vitals = ActorVitals {
            health: 75,
            max_health: 150,
            mana: 30,
            max_mana: 30,
            ..ActorVitals::default()
        };
        let quads = vitals_bars(&atlas, Some(&vitals), [8.0, 8.0], cells)?;
        assert_eq!(quads.len(), 4);
        assert!((quads[1].size[0] - 60.0).abs() < f32::EPSILON);
        assert!((quads[3].size[0] - BAR_WIDTH_PX).abs() < f32::EPSILON);
        assert!(quads[2].position[1] > quads[0].position[1]);
        Ok(())
    }
}
