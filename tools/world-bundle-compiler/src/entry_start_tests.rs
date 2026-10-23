use super::*;
use std::collections::BTreeMap;

#[test]
fn tutorial_current_world_compiles_twice_and_accepts_the_islet_start() -> Result<(), Error> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let identity = derive_identity(&root, "oteryn")?;
    let bytes = build_twice(&root, &identity)?;
    let registry = registry(&root, &project(&root)?)?.0;
    check(
        &bytes,
        EntryStart {
            x: 32534,
            y: 32513,
            floor: -7,
        },
        &registry,
    )?;
    Ok(())
}

#[test]
fn tutorial_entry_start_requires_ground_and_a_steppable_orthogonal_neighbour() {
    let at = EntryStart {
        x: 32534,
        y: 32513,
        floor: -7,
    };
    assert!(check_cells(at, &BTreeMap::new()).is_err());
    assert!(
        check_cells(
            at,
            &BTreeMap::from([((at.x, at.y), false), ((at.x + 1, at.y), true)])
        )
        .is_err()
    );
    assert!(check_cells(at, &BTreeMap::from([((at.x, at.y), true)])).is_err());
    assert!(
        check_cells(
            at,
            &BTreeMap::from([((at.x, at.y), true), ((at.x + 1, at.y + 1), true)])
        )
        .is_err()
    );
    assert!(
        check_cells(
            at,
            &BTreeMap::from([((at.x, at.y), true), ((at.x + 1, at.y), false)])
        )
        .is_err()
    );
    assert!(
        check_cells(
            at,
            &BTreeMap::from([((at.x, at.y), true), ((at.x + 1, at.y), true)])
        )
        .is_ok()
    );
}
