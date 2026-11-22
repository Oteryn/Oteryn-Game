//! Observe changes in the character's confirmed Bosstiary kill total across pages.
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_runtime::{boss_encounter, raw_feature::GatedCommandResult};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub(in crate::play) struct BossCapture {
    last_scan: Option<Instant>,
    previous_total: Option<u128>,
    page_total: u128,
    cursor: String,
    waiting: bool,
    discard: bool,
    advanced: bool,
}

impl BossCapture {
    fn clear_scan(&mut self) {
        self.cursor.clear();
        self.page_total = 0;
    }

    pub(in crate::play) fn receive(&mut self, result: &GatedCommandResult) -> bool {
        if !std::mem::take(&mut self.waiting) {
            return false;
        }
        if std::mem::take(&mut self.discard) {
            return true;
        }
        let page = crate::play::feature_0115_mail::protocol::accepted(result)
            .then(|| boss_encounter::decode(&result.payload))
            .flatten();
        let Some(page) = page.filter(|page| {
            page.progress
                .first()
                .is_none_or(|(key, _)| *key > self.cursor)
        }) else {
            self.clear_scan();
            return true;
        };
        self.page_total += page
            .progress
            .iter()
            .map(|(_, n)| u128::from(*n))
            .sum::<u128>();
        if let Some(after) = page.progress_next {
            self.cursor = after;
        } else {
            self.advanced |= self
                .previous_total
                .replace(self.page_total)
                .is_some_and(|n| self.page_total > n);
            self.clear_scan();
        }
        true
    }
}

impl PlayLink {
    pub(in crate::play) fn boss_capture(&self, enabled: bool, now: Instant) -> (bool, bool) {
        let mut mailbox = lock(&self.mailbox);
        let mailbox = &mut *mailbox;
        let panel = &mut mailbox.boss_encounter;
        let scan = &mut mailbox.boss_capture;
        if !enabled || !panel.selected {
            if scan.last_scan.is_some() {
                scan.clear_scan();
                scan.previous_total = None;
                scan.advanced = false;
                scan.last_scan = None;
                scan.discard = scan.waiting;
            }
            return (false, false);
        }
        let due = !scan.cursor.is_empty()
            || scan
                .last_scan
                .is_none_or(|last| now.saturating_duration_since(last) >= Duration::from_secs(5));
        if due && !panel.pending && !scan.waiting {
            if scan.cursor.is_empty() {
                scan.last_scan = Some(now);
            }
            if let Some(query) = boss_encounter::query(&scan.cursor)
                && self.commands.send(PlayCommand::Feature(query)).is_ok()
            {
                scan.waiting = true;
                panel.pending = true;
            } else {
                scan.clear_scan();
            }
        }
        (
            scan.previous_total.is_some(),
            std::mem::take(&mut scan.advanced),
        )
    }
}
