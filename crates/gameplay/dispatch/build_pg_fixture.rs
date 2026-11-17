use std::{error::Error, path::Path};

#[path = "build_pg_fixture_cfg.rs"]
mod cfgs;

// The server also mounts this fixture. Inject dispatch-only test cfgs into its generated
// dispatch mount so the original shared fixture and every test body stay unchanged.
pub fn generate(root: &Path, out: &Path) -> Result<(), Box<dyn Error>> {
    let source = root.join(
        "../../../apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs",
    );
    let parent = source.parent().ok_or("fixture parent")?;
    let mut grouped = String::new();
    for line in std::fs::read_to_string(&source)?.lines() {
        if line.trim() == "#[test]" {
            grouped.push_str("#[cfg(feature = \"pg-tests-g1\")]\n");
        }
        if let Some(path) = line
            .trim()
            .strip_prefix("#[path = \"")
            .and_then(|line| line.strip_suffix("\"]"))
        {
            grouped.push_str(&format!("#[path = {:?}]\n", parent.join(path)));
        } else {
            grouped.push_str(line);
            grouped.push('\n');
        }
    }
    for &(before, after) in cfgs::PATCHES {
        grouped = grouped.replace(before, after);
    }
    let fixture = out.join("postgres_fixture.rs");
    std::fs::write(&fixture, grouped)?;
    std::fs::write(
        out.join("postgres_fixture_mount.rs"),
        format!("#[path = {fixture:?}]\nmod fixture;\n"),
    )?;
    Ok(())
}
