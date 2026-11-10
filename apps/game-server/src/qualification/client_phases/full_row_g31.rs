//! G31: options pages, HP/MP arcs, minimap memory, action bar, settings (apps/client local-reference-preferences, local-spell-action-bar, local-minimap).
//! These are client-local surfaces (no server route); see G05 for the dev-client limit.
//! (`sync.rs`) read and write a local file and call no server route, and protocol-oteryn has no
//! settings capability. The dev client (`oteryn_dev_client`) links only `client-runtime` and
//! `client-ui`, whose `reference_preferences` module exposes just the open shortcut, so the
//! import/export/backup/defaults controls cannot be driven from it.
//!
//! The row does not fake a pass: it proves the one surface the dev client does have (the
//! shortcut predicate answers `false` for no input) and reports MISSING-CLIENT naming it.
use super::*;
use oteryn_dev_client::client_ui::reference_preferences;

const SOURCE: &str = "source:crates/client-ui/src/features/local-reference-preferences/input.rs:4";

pub(super) async fn run_packet(_clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let (status, detail) = verdict(reference_preferences::open_key(&[]));
    evidence(&format!(
        "e2e_all scenario=G31.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn verdict(shortcut_fired_without_input: bool) -> (&'static str, String) {
    if shortcut_fired_without_input {
        return (
            "FAIL",
            "step=open-key;observed=fired-without-input".to_owned(),
        );
    }
    ("MISSING-CLIENT", SOURCE.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_names_the_dev_client_surface_without_spaces() {
        let (status, detail) = verdict(false);
        assert_eq!(status, "MISSING-CLIENT");
        assert!(detail.starts_with("source:") && !detail.contains(' '));
        assert_eq!(verdict(true).0, "FAIL");
    }
}
