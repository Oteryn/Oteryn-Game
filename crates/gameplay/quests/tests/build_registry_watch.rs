//! An absent optional binding directory must detect additions without rebuilding forever.
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn check(project: &Path, target: &Path) -> Result<(), Box<dyn Error>> {
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--quiet", "--target-dir"])
        .arg(target)
        .current_dir(project)
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(())
}

#[test]
fn npcfix_optional_binding_watch_is_stable_and_detects_its_first_document()
-> Result<(), Box<dyn Error>> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "npcfix-registry-watch-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    let project = fixture.0.join("crates/gameplay/quests");
    fs::create_dir_all(project.join("src"))?;
    let content = fixture.0.join("content/quests");
    fs::create_dir_all(content.join("definitions"))?;
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname='npcfix-registry-fixture'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )?;
    fs::write(
        project.join("src/lib.rs"),
        "mod registry { include!(concat!(env!(\"OUT_DIR\"),\"/authored_binding_documents.rs\")); }\n",
    )?;
    let helper = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("build_authored_bindings.rs")
        .canonicalize()?;
    fs::write(
        project.join("build.rs"),
        format!(
            "#[path={helper:?}] mod registry;\n\
             fn main() -> Result<(),Box<dyn std::error::Error>> {{\n\
             use std::io::Write;\n\
             let root=std::path::PathBuf::from(std::env::var(\"CARGO_MANIFEST_DIR\")?);\n\
             let out=std::path::PathBuf::from(std::env::var(\"OUT_DIR\")?);\n\
             std::fs::OpenOptions::new().create(true).append(true).open(root.join(\"runs\"))?.write_all(b\"run\\n\")?;\n\
             registry::generate(&root,&out)\n}}\n"
        ),
    )?;
    let target = fixture.0.join("target");
    check(&project, &target)?;
    assert_eq!(fs::read_to_string(project.join("runs"))?.lines().count(), 1);
    check(&project, &target)?;
    assert_eq!(
        fs::read_to_string(project.join("runs"))?.lines().count(),
        1,
        "unchanged input rebuilt because the watched optional directory is absent"
    );
    fs::create_dir(content.join("authored-bindings"))?;
    fs::write(content.join("authored-bindings/first.json"), "{}\n")?;
    check(&project, &target)?;
    assert_eq!(fs::read_to_string(project.join("runs"))?.lines().count(), 2);
    check(&project, &target)?;
    assert_eq!(fs::read_to_string(project.join("runs"))?.lines().count(), 2);
    Ok(())
}
