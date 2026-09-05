//! Permanent server-process qualification workload for the existing CI lanes.
//!
//! Cargo discovers this target in the Linux workspace tests; no product feature,
//! dependency, workflow, or classifier exception is needed. Run it directly with
//! `cargo test --locked -p oteryn-game-server --test server_ci_qualification`.
//! A server-only edit here can qualify routing through normal protected merge.
//! The routing proof is the complete protected Git range plus actual lane/job
//! results, not this filename or a passing test. Keep FULL for unproven inputs.

use std::process::Command;

#[test]
fn server_ci_qualification_smoke_process_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_oteryn-game-server"))
        .arg("--smoke")
        .output()?;

    assert!(output.status.success(), "smoke process failed: {output:?}");
    assert!(
        output.stderr.is_empty(),
        "unexpected diagnostic: {output:?}"
    );
    Ok(())
}

#[test]
fn server_ci_qualification_gameplay_process_stays_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    for arguments in [vec![], vec!["--smok"], vec!["--smoke=false"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_oteryn-game-server"))
            .args(&arguments)
            .output()?;

        assert_eq!(output.status.code(), Some(2), "{arguments:?}: {output:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}: {output:?}");
        let diagnostic = String::from_utf8(output.stderr)?;
        assert!(
            diagnostic.starts_with("Oteryn Game Server gameplay unavailable: "),
            "{arguments:?}: {diagnostic}"
        );
    }
    Ok(())
}
