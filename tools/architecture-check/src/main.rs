use std::path::PathBuf;

fn main() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() != 3 {
        return Err(
            "usage: oteryn-architecture-check <workspace|client-structure> <root>".to_owned(),
        );
    }
    let root = PathBuf::from(&arguments[2]);
    match arguments[1].as_str() {
        "workspace" => oteryn_architecture_check::validate_workspace(&root)?,
        "client-structure" => oteryn_architecture_check::client_structure::validate(&root)?,
        _ => return Err("expected workspace or client-structure".into()),
    }
    println!("workspace-boundaries: PASS");
    Ok(())
}
