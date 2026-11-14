//! Approved previews enter the same queue as other playable session commands.
use crate::combat_input::PlayCommand;
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_runtime::support_intake as wire;
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

pub(crate) fn directory() -> Option<PathBuf> {
    crate::native_login::settings_path()
        .ok()?
        .parent()
        .map(Path::to_owned)
}
pub(crate) fn preview(text: &str) -> String {
    let reference = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("Reference: {reference}\n{text}")
}
pub(crate) fn queue_support_at(
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    directory: &Path,
) {
    for (name, kind) in [
        ("approved-crash.txt", wire::CRASH),
        ("approved-survey.txt", wire::SURVEY),
    ] {
        let path = directory.join(name);
        let Ok(text) = read(&path) else { continue };
        let Some(command) = oteryn_client_ui::support_intake::submission(kind, &text, true) else {
            continue;
        };
        let _ = commands.send(PlayCommand::SupportDelivery(command, path));
    }
}
fn read(path: &Path) -> io::Result<String> {
    let mut text = String::new();
    std::fs::File::open(path)?
        .take((wire::MAX_PREVIEW_BYTES + 1) as u64)
        .read_to_string(&mut text)?;
    if text.len() > wire::MAX_PREVIEW_BYTES {
        return Err(io::Error::other("Report is too large"));
    }
    Ok(text)
}
pub(crate) fn acknowledge(
    path: &Path,
    command: &FeatureCommand,
    result: &oteryn_session::GatedCommandResult,
) {
    use oteryn_session::CommandStatus;
    if matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && result.payload == [1]
        && read(path).ok().as_deref() == wire::decode_submission(&command.payload).ok()
    {
        let _ = std::fs::remove_file(path);
    }
}

/// Retained projection of exact approved evidence and the server's delivery reply.
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Evidence {
    pub path: PathBuf,
    pub kind: u32,
    pub text: String,
    pub pending: bool,
    pub result: Option<oteryn_session::GatedCommandResult>,
}
#[cfg(feature = "bevy-shell")]
impl Evidence {
    pub fn new(path: PathBuf, kind: u32, text: String) -> Self {
        Self {
            path,
            kind,
            text,
            pending: false,
            result: None,
        }
    }
}
#[cfg(feature = "bevy-shell")]
#[derive(Default)]
pub(crate) struct EvidenceOwner {
    rows: Vec<Evidence>,
}
#[cfg(feature = "bevy-shell")]
impl EvidenceOwner {
    pub fn rows(&self) -> &[Evidence] {
        &self.rows
    }
    pub fn observe(&mut self, evidence: Evidence) {
        if !self
            .rows
            .iter()
            .any(|row| row.path == evidence.path && row.text == evidence.text)
        {
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
        send: impl FnOnce(FeatureCommand) -> bool,
    ) -> Option<()> {
        if blocked || !consent {
            return None;
        }
        let row = self.rows.iter_mut().find(|row| *row == expected)?;
        if row.pending || row.result.is_some() {
            return None;
        }
        let command = oteryn_client_ui::support_intake::submission(row.kind, &row.text, consent)?;
        if !send(command) {
            return None;
        }
        row.pending = true;
        Some(())
    }
    fn reply(
        &mut self,
        path: &Path,
        command: &FeatureCommand,
        result: &oteryn_session::GatedCommandResult,
    ) {
        for row in &mut self.rows {
            if row.pending
                && row.path == path
                && row.kind == command.kind
                && wire::decode_submission(&command.payload).ok() == Some(row.text.as_str())
            {
                row.pending = false;
                row.result = Some(result.clone());
            }
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
    if let Some(entry) = retained.iter_mut().find(|entry| entry.key == key) {
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
        assert!(owner.submit(&row, true, false, |_| true).is_some());
        owner.reply(Path::new("changed"), &command, &result);
        assert!(owner.rows()[0].pending);
        owner.reply(&row.path, &command, &result);
        assert_eq!(owner.rows()[0].result, Some(result));
        assert!(!owner.rows()[0].pending);
        Ok(())
    }
}
