//! Mount optional per-quest source qualification modules only in the test harness.
use std::fmt::Write as _;
use std::path::Path;

pub fn generate(root: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let directory = root.join("src/quest/batch3_a");
    println!("cargo:rerun-if-changed={}", directory.display());
    let programs = root.join("../../../content/dialogues/quest-rules");
    println!("cargo:rerun-if-changed={}", programs.display());
    let mut mounts = String::new();
    for quest in [
        "bigfoot_burden",
        "hot_cuisine",
        "in_service_of_yalahar",
        "the_cursed_crystal",
        "the_way_of_the_monk",
    ] {
        let module = directory.join(format!("{quest}.rs"));
        if !module.is_file() {
            continue;
        }
        let program = programs.join(format!("quest-batch3-a-{quest}.json"));
        if !program.is_file() {
            return Err(format!("source harness {quest} requires {}", program.display()).into());
        }
        println!("cargo:rerun-if-changed={}", module.display());
        println!("cargo:rerun-if-changed={}", program.display());
        if quest == "hot_cuisine" {
            let recipes = root.join("../../../content/quests/hot-cuisine-batch3.json");
            if !recipes.is_file() {
                return Err("Hot Cuisine source harness requires its own recipe data".into());
            }
            println!("cargo:rerun-if-changed={}", recipes.display());
            // Hot Cuisine also has a production mount; share it to keep one test module.
            writeln!(mounts, "pub use crate::quest::hot_cuisine;")?;
            continue;
        }
        let module = module.to_str().ok_or("source harness path is not UTF-8")?;
        writeln!(mounts, "#[path = {module:?}]\npub mod {quest};")?;
    }
    std::fs::write(out.join("quest_batch3_a_modules.rs"), mounts)?;
    Ok(())
}
