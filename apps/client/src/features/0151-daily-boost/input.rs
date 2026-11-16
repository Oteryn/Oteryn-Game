use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::daily_boost::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn daily_boost_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.daily_boost, intent)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.daily_boost.pending = false;
            mailbox.daily_boost.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn daily_boost_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::daily_boost::open_key(events) {
            return false;
        }
        {
            let mut mailbox = lock(&self.mailbox);
            mailbox.daily_boost.open = true;
        }
        refresh(&mut lock(&self.mailbox), &self.commands);
        true
    }
}

pub(in crate::play) fn window_open(mailbox: &crate::play::mailbox::Mailbox) -> bool {
    mailbox.daily_boost.open && !mailbox.quest_shutdown
}

pub(in crate::play) fn refresh(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    if !window_open(mailbox) {
        return;
    }
    if let Some(command) = request(&mut mailbox.daily_boost, Intent {
        command: oteryn_client_runtime::cui3::daily_boost::COMMAND_TYPE_DAILY_BOOST_QUERY,
        payload: Vec::new(),
    }) && commands.send(PlayCommand::Feature(command)).is_err() {
        mailbox.daily_boost.pending = false;
        mailbox.daily_boost.feedback = Some(tr!("The session has ended"));
    }
    if let Some(payload) = oteryn_client_ui::potion_durations::request(&mut mailbox.potion_durations)
        && commands.send(PlayCommand::Feature(oteryn_client_runtime::raw_feature::FeatureCommand {
            capability: 381, kind: 662, payload,
        })).is_err()
    {
        mailbox.potion_durations.pending = false;
        mailbox.potion_durations.error = Some(tr!("Session ended"));
    }
}
