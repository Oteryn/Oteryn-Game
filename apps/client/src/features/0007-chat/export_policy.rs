use std::io;

pub(super) fn validate_destination(
    regular_file: bool,
    trusted_owner: bool,
    restricted_acl: bool,
) -> io::Result<()> {
    if regular_file && trusted_owner && restricted_acl {
        Ok(())
    } else {
        Err(io::ErrorKind::PermissionDenied.into())
    }
}

#[cfg(test)]
#[test]
fn windows_export_refuses_reparse_points_and_broad_access() -> io::Result<()> {
    validate_destination(true, true, true)?;
    for (file, owner, acl) in [
        (false, true, true),
        (true, false, true),
        (true, true, false),
    ] {
        assert_eq!(
            validate_destination(file, owner, acl)
                .err()
                .map(|error| error.kind()),
            Some(io::ErrorKind::PermissionDenied)
        );
    }
    Ok(())
}
