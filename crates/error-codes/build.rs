//! Derives the workspace source id (`OTERYN_BUILD_SOURCE`), ARCH-ERROR-CODES-0 §1.10 item 2.

#[path = "src/build_source.rs"]
mod build_source;

fn main() {
    for directive in [
        "rerun-if-env-changed=OTERYN_BUILD_SHA",
        "rerun-if-changed=build.rs",
        "rerun-if-changed=src/build_source.rs",
    ] {
        println!("cargo:{directive}");
    }
    let sha = match std::env::var("OTERYN_BUILD_SHA") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            eprintln!("error: OTERYN_BUILD_SHA is not valid UTF-8");
            std::process::exit(1);
        }
    };
    match build_source::resolve(sha.as_deref()) {
        Ok(source) => println!("cargo:rustc-env=OTERYN_BUILD_SOURCE={source}"),
        Err(reason) => {
            eprintln!("error: {reason}");
            std::process::exit(1);
        }
    }
}
