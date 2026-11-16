//! Platform-neutral property and service form: title, labelled fields, buttons and a summary.
//! The session owns the values; the desktop shell only presents and submits them.
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use tokio::sync::mpsc::UnboundedSender;

/// Open, pending, the field values and the summary text.
pub(in crate::play) type Snapshot = (bool, bool, Vec<String>, String);

#[derive(Clone, Copy)]
pub(in crate::play) struct Form {
    pub title: &'static str,
    pub labels: &'static [&'static str],
    pub buttons: &'static [&'static str],
    pub snapshot: fn(&Mailbox) -> Snapshot,
    pub set_open: fn(&mut Mailbox, bool),
    pub submit: fn(&mut Mailbox, &UnboundedSender<PlayCommand>, usize, Vec<String>),
}

/// A feature's per-frame upkeep while its form is open, such as paging a catalogue.
type Tick = fn(&mut Mailbox, &UnboundedSender<PlayCommand>);

/// One registered form window: how it is built and its optional per-frame upkeep.
/// Each feature's own key or opener opens it; the window only presents what is open.
#[derive(Clone, Copy)]
pub struct FeatureForm {
    build: fn(&PlayLink) -> Form,
    tick: Option<Tick>,
}

/// What a form window shows on this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormView {
    pub title: &'static str,
    pub labels: &'static [&'static str],
    pub buttons: &'static [&'static str],
    pub open: bool,
    pub pending: bool,
    pub fields: Vec<String>,
    pub text: String,
}

/// One trimmed field value; a missing field reads as empty.
pub(in crate::play) fn field(values: &[String], index: usize) -> &str {
    values.get(index).map_or("", |value| value.trim())
}

const fn entry(build: fn(&PlayLink) -> Form, tick: Option<Tick>) -> FeatureForm {
    FeatureForm { build, tick }
}

/// The forms the shared frontend presents with its generic form window, in feature order.
/// Every feature with its own Bevy presenter stays out of this list.
const FORMS: &[FeatureForm] = &[
    entry(crate::play::feature_0008_achievement_notices::view::form, None),
    entry(crate::play::feature_0025_appearance::view::memorial::form, None),
    entry(crate::play::feature_0039_timed_item::view::form, None),
    entry(crate::play::feature_0040_guild::view::form, None),
    entry(crate::play::feature_0043_vip::input::account::form, None),
    entry(crate::play::feature_0053_world_event::view::form, None),
    entry(crate::play::feature_0054_training::view::form, None),
    entry(crate::play::feature_0055_guild_bank::view::form, None),
    entry(crate::play::feature_0059_house_move_out::view::form, None),
    entry(crate::play::feature_0063_market::input::coins::form, None),
    entry(crate::play::feature_0080_chat_private_channel::view::form, None),
    entry(crate::play::feature_0086_house_transfer::view::form, None),
    entry(crate::play::feature_0097_character_info::view::form, None),
    entry(crate::play::feature_0097_character_info::view::titles_form, None),
    entry(crate::play::feature_0099_guildhall::view::form, None),
    entry(crate::play::feature_0103_guild_war::view::form, None),
    entry(crate::play::feature_0115_mail::view::form_window, None),
    entry(crate::play::feature_0117_boss_encounter::view::cooldown_form, None),
    entry(crate::play::feature_0147_exiva_settings::view::form, None),
    entry(crate::play::feature_0149_raid_schedule::view::form, None),
    entry(crate::play::feature_0150_house_podium::view::form, None),
    entry(crate::play::feature_0151_daily_boost::view::form, None),
    entry(crate::play::feature_0154_store::view::premium_game::premium_form, None),
    entry(crate::play::feature_0165_hireling::view::form, None),
    entry(crate::play::feature_0168_animus_mastery::view::form, None),
    entry(crate::play::feature_0170_arena::view::form, None),
    entry(crate::play::feature_0179_hotkey_equip::view::form, None),
    entry(crate::play::feature_0182_daily_reward::view::returner_form, None),
    entry(crate::play::feature_0197_cyclopedia_tracker::view::form, None),
    entry(crate::play::feature_0199_cyclopedia_character::view::sanctions::form, None),
    entry(crate::play::feature_0207_house_furniture_wrap::view::form, None),
    entry(crate::play::feature_0224_boss_rewards::view::form, None),
    entry(crate::play::feature_0225_boss_difficulty::view::form, None),
    entry(crate::play::feature_0240_chat_party_room::view::form, None),
    entry(crate::play::feature_0250_hireling_management::view::form, None),
    entry(crate::play::feature_0259_chat_public_channels::view::form, None),
    entry(crate::play::feature_0280_party_effects::protocol::form, None),
    entry(crate::play::feature_0381_potion_durations::protocol::form, None),
    entry(crate::play::feature_0445_house_furniture_rotation::view::form, None),
    entry(
        crate::play::feature_local_account_achievements::view::form,
        Some(crate::play::feature_local_account_achievements::view::tick),
    ),
];

impl FeatureForm {
    /// Every form the generic window presents.
    #[must_use]
    pub fn all() -> &'static [FeatureForm] {
        FORMS
    }
    /// The window title the form was registered with.
    #[must_use]
    pub fn title(&self, link: &PlayLink) -> &'static str {
        (self.build)(link).title
    }
}

impl PlayLink {
    /// The form's current presentation, read under the mailbox lock and released at once.
    #[must_use]
    pub fn feature_form_view(&self, form: FeatureForm) -> FormView {
        let built = (form.build)(self);
        let (open, pending, fields, text) = (built.snapshot)(&lock(&self.mailbox));
        FormView {
            title: built.title,
            labels: built.labels,
            buttons: built.buttons,
            open,
            pending,
            fields,
            text,
        }
    }

    /// Runs the feature's per-frame upkeep while its form is open.
    pub fn feature_form_tick(&self, form: FeatureForm) {
        let Some(tick) = form.tick else {
            return;
        };
        let built = (form.build)(self);
        let mut mailbox = lock(&self.mailbox);
        if (built.snapshot)(&mailbox).0 {
            tick(&mut mailbox, &self.commands);
        }
    }

    /// Submits one button with the field values; refused while the form waits for the server.
    pub fn feature_form_submit(&self, form: FeatureForm, action: usize, values: Vec<String>) {
        let built = (form.build)(self);
        if action >= built.buttons.len() {
            return;
        }
        let mut mailbox = lock(&self.mailbox);
        let (open, pending, _, _) = (built.snapshot)(&mailbox);
        if !open || pending {
            return;
        }
        (built.submit)(&mut mailbox, &self.commands, action, values);
    }

    /// Closes the form window; the feature's session state stays.
    pub fn feature_form_close(&self, form: FeatureForm) {
        let built = (form.build)(self);
        (built.set_open)(&mut lock(&self.mailbox), false);
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    /// Admits the capabilities the generic form window tests drive; no server reply is made up.
    pub(crate) fn bevy_feature_forms_selected(&self) {
        let mut mailbox = lock(&self.0);
        mailbox.chat_party_room.selected = true;
        mailbox.party_effects.selected = true;
    }
}
