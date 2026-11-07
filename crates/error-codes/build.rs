//! Derives the workspace source id (`OTERYN_BUILD_SOURCE`), ARCH-ERROR-CODES-0 §1.10 item 2.

#[path = "src/build_source.rs"]
mod build_source;

use std::process::Command;

fn git(arguments: &[&str]) -> Option<String> {
    let output = Command::new("git").args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn watch(path: &str) {
    if std::path::Path::new(path).exists() {
        println!("cargo:rerun-if-changed={path}");
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=OTERYN_BUILD_SHA");
    watch("build.rs");
    if let Some(head) = git(&["rev-parse", "--git-path", "HEAD"]) {
        watch(&head);
    }
    if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git(&["rev-parse", "--git-path", &reference])
    {
        watch(&path);
    }
    // A branch that exists only in `packed-refs` has no loose ref file to watch.
    if let Some(packed) = git(&["rev-parse", "--git-path", "packed-refs"]) {
        watch(&packed);
    }
    let sha = match std::env::var("OTERYN_BUILD_SHA") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            eprintln!("error: OTERYN_BUILD_SHA is not valid UTF-8");
            std::process::exit(1);
        }
    };
    let head = git(&["rev-parse", "--short=12", "HEAD"]);
    match build_source::resolve(sha.as_deref(), head.as_deref()) {
        Ok(source) => println!("cargo:rustc-env=OTERYN_BUILD_SOURCE={source}"),
        Err(reason) => {
            eprintln!("error: {reason}");
            std::process::exit(1);
        }
    }
}
