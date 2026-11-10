use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn analyser_boss_rewards_result(
        &self,
        response: &oteryn_client_runtime::boss_rewards::BossRewardsResponse,
    ) {
        let mut guard = lock(&self.0);
        let mailbox = &mut *guard;
        oteryn_client_ui::analyser::bosses::observe(
            &mut mailbox.analyser,
            &mailbox.boss_encounter,
            response,
        );
    }
    pub(in crate::play) fn analyser_result(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_session::GatedCommandResult,
    ) {
        match command.kind {
            734 => self.analyser_catalogue_result(result),
            735 => oteryn_client_ui::analyser::party::result(&mut lock(&self.0).analyser, result),
            _ => {}
        }
    }

    pub(in crate::play) fn analyser_catalogue_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        oteryn_client_ui::analyser::protocol::catalogue_result(&mut lock(&self.0).analyser, result);
    }

    pub(in crate::play) fn analyser_initialize(&self, selected: bool) {
        lock(&self.0).analyser = oteryn_client_ui::analyser::Panel {
            selected,
            ..Default::default()
        };
    }
}

pub(in crate::play) fn reduce(
    mailbox: &mut crate::play::mailbox::Mailbox,
    events: &[oteryn_session::SessionEvent],
) {
    use oteryn_session::SessionEvent;
    for event in events {
        match event {
            SessionEvent::AnalyserFacts(delta) => {
                oteryn_client_ui::analyser::protocol::apply(&mut mailbox.analyser, &delta.value)
            }
            SessionEvent::GatedSnapshotReplaced(delta) if delta.value.domain_id == 15 => {
                // oteryn-choice: recovery cannot replay facts. Start a fresh measurement interval.
                oteryn_client_ui::analyser::protocol::recover(&mut mailbox.analyser);
            }
            _ => {}
        }
    }
}

/// Task progress is copied only from confirmed task replies, never inferred from session kills.
pub(in crate::play) fn task_progress(mailbox: &mut crate::play::mailbox::Mailbox, confirmed: bool) {
    if confirmed
        && let Some(rows) = oteryn_client_ui::analyser::kills::task_rows(&mailbox.task_board)
    {
        mailbox.analyser.task_progress = Some(rows);
        mailbox.analyser.task_feedback = None;
    } else {
        mailbox.analyser.task_feedback = Some(tr!(
            "Hunting tasks not updated; previous confirmation retained"
        ));
    }
}

#[cfg(test)]
#[path = "tests/e2e_n07c.rs"]
mod e2e_n07c;
