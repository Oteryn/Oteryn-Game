use std::path::PathBuf;

fn main() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() != 3 || arguments[1] != "workspace" {
        return Err("usage: oteryn-architecture-check workspace <root>".to_owned());
    }
    let root = PathBuf::from(&arguments[2]);
    oteryn_architecture_check::validate_workspace(&root)?;
    println!("workspace-boundaries: PASS");
    Ok(())
}
