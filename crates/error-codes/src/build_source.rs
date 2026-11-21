//! The `<source>` part of a build id (ARCH-ERROR-CODES-0 §1.10 item 2). Shared by `build.rs`.

/// `<source>`: the first 12 digits of a 40-digit `OTERYN_BUILD_SHA`, else `dev.local`.
/// Development builds make no source identity claim and stay stable across commits.
pub fn resolve(sha: Option<&str>) -> Result<String, String> {
    if let Some(sha) = sha {
        if sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Ok(sha[..12].to_ascii_lowercase());
        }
        return Err("OTERYN_BUILD_SHA must be exactly 40 hexadecimal digits".to_owned());
    }
    Ok("dev.local".to_owned())
}

#[cfg(test)]
mod tests {
    use super::resolve;

    const SHA: &str = "0123456789ABCDEF0123456789abcdef01234567";

    #[test]
    fn a_set_sha_gives_a_bare_source() {
        assert_eq!(resolve(Some(SHA)), Ok("0123456789ab".to_owned()));
    }

    #[test]
    fn an_unset_sha_gives_a_stable_development_source() {
        assert_eq!(resolve(None), Ok("dev.local".to_owned()));
    }

    #[test]
    fn a_value_that_is_not_forty_hex_digits_fails_the_build() {
        for bad in [
            "",
            "abc",
            &SHA[..39],
            &format!("{SHA}0"),
            &SHA.replace('0', "g"),
        ] {
            assert!(resolve(Some(bad)).is_err(), "{bad:?}");
        }
    }
}
