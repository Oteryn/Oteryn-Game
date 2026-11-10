use std::{path::Path, process::Command};
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
        .map(|text| text.trim().to_owned())
}
pub fn emit(root: &Path) -> Result<(), String> {
    let target = std::env::var("TARGET").map_err(|error| error.to_string())?;
    let commit = git(root, &["rev-parse", "HEAD"]).filter(|commit| {
        commit.len() == 40
            && commit
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    });
    let installed =
        std::env::var("OTERYN_RELEASE_ID").is_ok_and(|release| !release.contains(".local."));
    if installed && commit.is_none() {
        return Err("An installed release requires its exact 40-digit Game commit".into());
    }
    let commit = commit.unwrap_or_else(|| "0".repeat(40));
    println!("cargo:rustc-env=OTERYN_GAME_COMMIT={commit}");
    println!("cargo:rustc-env=OTERYN_BUILD_TARGET={target}");
    println!("cargo:rerun-if-env-changed=OTERYN_RELEASE_ID");
    for name in [
        Some("HEAD".to_owned()),
        Some("packed-refs".to_owned()),
        git(root, &["symbolic-ref", "-q", "HEAD"]),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(path) = git(root, &["rev-parse", "--git-path", &name]) {
            let path = root.join(path);
            if path.exists() {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
    Ok(())
}
