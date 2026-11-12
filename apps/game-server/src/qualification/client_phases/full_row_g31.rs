//! G31: options pages, HP/MP arcs, minimap memory, action bar, settings (apps/client local-reference-preferences, local-spell-action-bar, local-minimap).
//! These are client-local surfaces (no server route); see G05 for the dev-client limit.
//! The row does not fake a pass or claim a missing client surface: it proves the one surface the
//! dev client does reach (the shortcut predicate answers `false` for no input) and otherwise ends
//! FAIL with `needed:` naming the local driver the row's controls would require.
use super::*;
use oteryn_dev_client::client_ui::reference_preferences;

const NEEDED: &str = "needed:local-options-arcs-minimap-memory-action-bar-settings-driver";

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
    ("FAIL", NEEDED.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_names_the_needed_driver_without_spaces() {
        let (status, detail) = verdict(false);
        assert_eq!(status, "FAIL");
        assert!(detail.starts_with("needed:") && !detail.contains(' '));
        assert_eq!(verdict(true).0, "FAIL");
    }
}
