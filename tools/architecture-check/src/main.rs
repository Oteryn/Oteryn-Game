use std::path::PathBuf;

fn main() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() < 3 || arguments.len() > 4 {
        return Err(
            "usage: oteryn-architecture-check <workspace|client-structure|logging-events|logging-coverage> <root> [--update-baseline]".to_owned(),
        );
    }
    let root = PathBuf::from(&arguments[2]);
    match arguments[1].as_str() {
        "workspace" => oteryn_architecture_check::validate_workspace(&root)?,
        "client-structure" => oteryn_architecture_check::client_structure::validate(&root)?,
        "logging-events" => oteryn_architecture_check::logging::validate(
            &root,
            arguments
                .get(3)
                .is_some_and(|value| value == "--update-baseline"),
        )?,
        "logging-coverage" => {
            print!("{}", oteryn_architecture_check::logging::coverage(&root)?);
        }
        _ => return Err("unknown architecture check".into()),
    }
    println!("workspace-boundaries: PASS");
    Ok(())
}
