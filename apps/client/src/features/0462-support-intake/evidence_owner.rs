//! Session-bound projection and retry state over the existing support delivery dispatcher.
#[cfg(test)]
use crate::support_delivery::{acknowledge, queue_support_at};
use crate::{
    combat_input::PlayCommand,
    support_delivery::{preview, read},
};
use oteryn_client_runtime::{raw_feature::FeatureCommand, support_intake as wire};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
/// Retained projection of exact approved evidence and the server's delivery reply.
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Evidence {
    pub path: PathBuf,
    pub kind: u32,
    pub text: String,
    pub pending: bool,
    source: Option<String>,
    pub result: Option<oteryn_session::GatedCommandResult>,
}
#[cfg(feature = "bevy-shell")]
impl Evidence {
    fn delivery_path(&self) -> PathBuf {
        if self.source.is_some() {
            self.path.with_file_name("approved-crash.txt")
        } else {
            self.path.clone()
        }
    }
    pub fn new(path: PathBuf, kind: u32, text: String) -> Self {
        Self {
            path,
            kind,
            text,
            pending: false,
            source: None,
            result: None,
        }
    }
}
#[cfg(feature = "bevy-shell")]
pub(crate) fn matches_file(row: &Evidence) -> bool {
    read(&row.path).ok().as_deref() == Some(row.source.as_deref().unwrap_or(&row.text))
}
#[cfg(feature = "bevy-shell")]
#[derive(Default)]
pub(crate) struct EvidenceOwner {
    rows: Vec<Evidence>,
    changed: Arc<AtomicBool>,
}
#[cfg(feature = "bevy-shell")]
impl EvidenceOwner {
    pub fn watch(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.changed)
    }
    pub fn rows(&self) -> &[Evidence] {
        &self.rows
    }
    pub fn refresh(&mut self, directory: &Path) {
        for evidence in evidence_at(directory) {
            self.observe(evidence);
        }
        let path = directory.join("pending-crash.txt");
        if let Ok(raw) = read(&path) {
            if self
                .rows
                .iter()
                .any(|row| row.path == path && row.source.as_deref() == Some(raw.as_str()))
            {
                return;
            }
            let text = preview(&raw);
            if oteryn_client_ui::support_intake::submission(wire::CRASH, &text, true).is_some() {
                let mut evidence = Evidence::new(path, wire::CRASH, text);
                evidence.source = Some(raw);
                self.observe(evidence);
            }
        }
    }
    pub fn observe(&mut self, evidence: Evidence) {
        if !self.rows.iter().any(|row| {
            (row.path == evidence.path || row.delivery_path() == evidence.path)
                && row.text == evidence.text
        }) {
            self.rows
                .retain(|row| row.path != evidence.path || row.pending);
            self.rows.push(evidence);
        }
    }
    pub fn submit(
        &mut self,
        expected: &Evidence,
        consent: bool,
        blocked: bool,
        send: impl FnOnce(FeatureCommand, PathBuf) -> bool,
    ) -> Option<()> {
        if blocked || !consent {
            return None;
        }
        let row = self.rows.iter_mut().find(|row| *row == expected)?;
        if row.pending || row.result.is_some() {
            return None;
        }
        let command = oteryn_client_ui::support_intake::submission(row.kind, &row.text, consent)?;
        let path = row.delivery_path();
        if row.source.is_some() {
            if !matches_file(row) || crate::crash_report::save(&row.text, &path, true).is_err() {
                return None;
            }
            if std::fs::remove_file(&row.path).is_err() {
                return None;
            }
            row.path = path.clone();
            row.source = None;
        }
        if !send(command, path) {
            return None;
        }
        row.pending = true;
        Some(())
    }
    pub(crate) fn reply(
        &mut self,
        path: &Path,
        command: &FeatureCommand,
        result: &oteryn_session::GatedCommandResult,
    ) {
        for row in &mut self.rows {
            if row.pending
                && row.delivery_path() == path
                && row.kind == command.kind
                && wire::decode_submission(&command.payload).ok() == Some(row.text.as_str())
            {
                row.pending = false;
                row.result = (matches!(
                    result.status,
                    oteryn_session::CommandStatus::Accepted
                        | oteryn_session::CommandStatus::DuplicateReplay
                ) && result.payload == [1])
                .then(|| result.clone());
                self.changed.store(true, Ordering::Release);
            }
        }
    }
}
pub(crate) fn queue_approved(
    sender: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    path: &Path,
    kind: u32,
    text: &str,
    send: impl FnOnce() -> bool,
) {
    let mut retained = RETAINED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !send() {
        return;
    }
    for entry in retained.iter_mut().filter(|entry| {
        entry
            .sender
            .upgrade()
            .is_some_and(|current| current.same_channel(sender))
    }) {
        entry
            .owner
            .observe(Evidence::new(path.to_owned(), kind, text.to_owned()));
        if let Some(row) = entry
            .owner
            .rows
            .iter_mut()
            .find(|row| row.path == path && row.text == text)
        {
            row.pending = true;
        }
    }
}
#[cfg(feature = "bevy-shell")]
struct Retained {
    key: usize,
    sender: tokio::sync::mpsc::WeakUnboundedSender<PlayCommand>,
    owner: EvidenceOwner,
}
#[cfg(feature = "bevy-shell")]
static RETAINED: std::sync::Mutex<Vec<Retained>> = std::sync::Mutex::new(Vec::new());
#[cfg(feature = "bevy-shell")]
pub(crate) fn with_owner<R>(
    key: usize,
    sender: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    f: impl FnOnce(&mut EvidenceOwner) -> R,
) -> R {
    let mut retained = RETAINED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    retained.retain(|entry| {
        entry
            .sender
            .upgrade()
            .is_some_and(|sender| !sender.is_closed())
    });
    let index = match retained.iter().position(|entry| entry.key == key) {
        Some(index) => index,
        None => {
            retained.push(Retained {
                key,
                sender: sender.downgrade(),
                owner: EvidenceOwner::default(),
            });
            retained.len() - 1
        }
    };
    f(&mut retained[index].owner)
}
#[cfg(feature = "bevy-shell")]
pub(crate) fn retain_reply(
    key: usize,
    path: &Path,
    command: &FeatureCommand,
    result: &oteryn_session::GatedCommandResult,
) {
    let mut retained = RETAINED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(entry) = retained.iter_mut().find(|entry| {
        entry.key == key
            && entry
                .sender
                .upgrade()
                .is_some_and(|sender| !sender.is_closed())
    }) {
        entry.owner.reply(path, command, result);
    }
}
#[cfg(feature = "bevy-shell")]
pub(crate) fn evidence_at(directory: &Path) -> Vec<Evidence> {
    [
        ("approved-crash.txt", wire::CRASH),
        ("approved-survey.txt", wire::SURVEY),
    ]
    .into_iter()
    .filter_map(|(name, kind)| {
        let path = directory.join(name);
        let text = read(&path).ok()?;
        oteryn_client_ui::support_intake::submission(kind, &text, true)?;
        Some(Evidence::new(path, kind, text))
    })
    .collect()
}

#[cfg(all(test, feature = "bevy-shell"))]
mod tests {
    use super::*;
    #[test]
    fn support_evidence_default_acknowledgement_unchanged() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = std::env::temp_dir().join(format!("u52-default-{}", std::process::id()));
        std::fs::create_dir_all(&directory)?;
        let path = directory.join("approved-crash.txt");
        std::fs::write(&path, "default approved evidence")?;
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        queue_support_at(&sender, &directory);
        let PlayCommand::SupportDelivery(command, queued_path) = receiver.try_recv()? else {
            return Err("expected existing support dispatcher".into());
        };
        assert_eq!(queued_path, path);
        assert!(receiver.try_recv().is_err());
        let mut result = oteryn_session::GatedCommandResult {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![0],
        };
        acknowledge(&path, &command, &result);
        assert!(path.exists());
        result.payload = vec![1];
        acknowledge(&path, &command, &result);
        assert!(!path.exists());
        std::fs::remove_dir(directory)?;
        Ok(())
    }
    #[test]
    fn support_evidence_reply_retains_only_matching_pending_target()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut owner = EvidenceOwner::default();
        let row = Evidence::new(
            "approved-crash.txt".into(),
            wire::CRASH,
            "approved evidence".into(),
        );
        owner.observe(row.clone());
        let command = oteryn_client_ui::support_intake::submission(row.kind, &row.text, true)
            .ok_or("invalid evidence")?;
        let result = oteryn_session::GatedCommandResult {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![1],
        };
        owner.reply(&row.path, &command, &result);
        assert!(owner.rows()[0].result.is_none());
        assert!(owner.submit(&row, true, false, |_, _| true).is_some());
        owner.reply(Path::new("changed"), &command, &result);
        assert!(owner.rows()[0].pending);
        owner.reply(&row.path, &command, &result);
        assert_eq!(owner.rows()[0].result, Some(result));
        assert!(!owner.rows()[0].pending);
        Ok(())
    }
}
