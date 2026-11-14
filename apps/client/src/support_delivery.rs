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
        #[cfg(feature = "bevy-shell")]
        crate::play::feature_0462_support_intake::input::evidence_owner::queue_approved(
            commands,
            &path,
            kind,
            &text,
            || {
                commands
                    .send(PlayCommand::SupportDelivery(command, path.clone()))
                    .is_ok()
            },
        );
        #[cfg(not(feature = "bevy-shell"))]
        let _ = commands.send(PlayCommand::SupportDelivery(command, path));
    }
}
pub(crate) fn read(path: &Path) -> io::Result<String> {
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
