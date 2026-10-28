/// The release this executable belongs to (CLIENT-INSTALLER-0 §2):
/// `<client_version>+<channel>.<route>.g<game_commit[0..12]>`, from `OTERYN_RELEASE_ID` at build
/// time. A build without it is a local developer build, `<client_version>+dev.local.g000000000000`.
pub const RELEASE_ID: &str = match option_env!("OTERYN_RELEASE_ID") {
    Some(release_id) => release_id,
    None => concat!(env!("CARGO_PKG_VERSION"), "+dev.local.g000000000000"),
};
const _: () = assert!(
    is_valid_release_id(RELEASE_ID),
    "OTERYN_RELEASE_ID must be <client_version>+<channel>.<route>.g<12 lowercase hex>, at most 50 bytes"
);

/// `client_build` reported to Platform and the Gateway: `oteryn-client/<release_id>`.
pub const CLIENT_BUILD: &str = {
    const PREFIX: &[u8] = b"oteryn-client/";
    const BYTES: [u8; PREFIX.len() + RELEASE_ID.len()] = {
        let mut bytes = [0; PREFIX.len() + RELEASE_ID.len()];
        let mut index = 0;
        while index < bytes.len() {
            bytes[index] = if index < PREFIX.len() {
                PREFIX[index]
            } else {
                RELEASE_ID.as_bytes()[index - PREFIX.len()]
            };
            index += 1;
        }
        bytes
    };
    match std::str::from_utf8(&BYTES) {
        Ok(client_build) => client_build,
        Err(_) => "oteryn-client/invalid",
    }
};

/// Whether `value` is a `<release_id>` of this client version: `<client_version>+<channel>.<route>.g<12 hex>`
/// with channel `dev`, `preproduction` or `stable`, route `ci` or `rel` (`local` only for the
/// all-zero commit of a developer build), lowercase hex, no `~`, at most 50 bytes.
#[must_use]
pub const fn is_valid_release_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    let version = env!("CARGO_PKG_VERSION").as_bytes();
    starts_with_at(bytes, 0, version)
        && starts_with_at(bytes, version.len(), b"+")
        && is_contract_release_id(value)
}

/// Whether `value` is a `<release_id>` of any client version: as [`is_valid_release_id`], but
/// `<client_version>` is any `<major>.<minor>.<patch>` of decimal numbers without leading zeros.
/// The launcher accepts these, because `current.txt` may name a release of another version.
#[must_use]
pub const fn is_contract_release_id(value: &str) -> bool {
    const CHANNELS: [&[u8]; 3] = [b"dev", b"preproduction", b"stable"];
    const ROUTES: [&[u8]; 3] = [b"ci", b"rel", b"local"];
    let bytes = value.as_bytes();
    if bytes.len() > 50 {
        return false;
    }
    let Some(mut at) = version_end(bytes) else {
        return false;
    };
    if !starts_with_at(bytes, at, b"+") {
        return false;
    }
    at += 1;
    let Some(channel_end) = matching_word(bytes, at, &CHANNELS) else {
        return false;
    };
    at = channel_end;
    if !starts_with_at(bytes, at, b".") {
        return false;
    }
    let Some(route_end) = matching_word(bytes, at + 1, &ROUTES) else {
        return false;
    };
    let local = route_end - (at + 1) == 5;
    at = route_end;
    if !starts_with_at(bytes, at, b".g") || bytes.len() != at + 14 {
        return false;
    }
    at += 2;
    while at < bytes.len() {
        let digit = bytes[at];
        if !(digit.is_ascii_digit() || (digit >= b'a' && digit <= b'f')) || (local && digit != b'0')
        {
            return false;
        }
        at += 1;
    }
    true
}

/// The end of a leading `<major>.<minor>.<patch>`, each a decimal number without leading zeros.
const fn version_end(bytes: &[u8]) -> Option<usize> {
    let mut at = 0;
    let mut part = 0;
    while part < 3 {
        if part > 0 {
            if !starts_with_at(bytes, at, b".") {
                return None;
            }
            at += 1;
        }
        let start = at;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
        }
        if at == start || (at - start > 1 && bytes[start] == b'0') {
            return None;
        }
        part += 1;
    }
    Some(at)
}

const fn starts_with_at(bytes: &[u8], at: usize, prefix: &[u8]) -> bool {
    if at + prefix.len() > bytes.len() {
        return false;
    }
    let mut index = 0;
    while index < prefix.len() {
        if bytes[at + index] != prefix[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// The end of the word of `words` that `bytes` holds at `at`, followed by a `.`.
const fn matching_word(bytes: &[u8], at: usize, words: &[&[u8]]) -> Option<usize> {
    let mut index = 0;
    while index < words.len() {
        let end = at + words[index].len();
        if starts_with_at(bytes, at, words[index]) && starts_with_at(bytes, end, b".") {
            return Some(end);
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_id_and_client_build_are_compiled_in() {
        assert!(is_valid_release_id(RELEASE_ID));
        assert_eq!(CLIENT_BUILD, format!("oteryn-client/{RELEASE_ID}"));
        assert!(CLIENT_BUILD.len() <= 64);
    }

    #[test]
    fn release_id_format_is_enforced() {
        let version = env!("CARGO_PKG_VERSION");
        for valid in [
            "+dev.ci.g1a2b3c4d5e6f",
            "+preproduction.rel.g0123456789ab",
            "+stable.ci.gabcdef012345",
            "+dev.local.g000000000000",
        ] {
            assert!(is_valid_release_id(&format!("{version}{valid}")), "{valid}");
        }
        for invalid in [
            "+dev.ci.g1a2b3c4d5e6",
            "+dev.ci.g1a2b3c4d5e6f0",
            "+dev.ci.g1A2B3C4D5E6F",
            "+dev.ci.g1a2b3c4d5e6~",
            "+beta.ci.g1a2b3c4d5e6f",
            "+dev.nightly.g1a2b3c4d5e6f",
            "+dev.local.g1a2b3c4d5e6f",
            "+dev.ci.1a2b3c4d5e6f",
            "dev.ci.g1a2b3c4d5e6f",
            "+dev.cig1a2b3c4d5e6f",
        ] {
            assert!(
                !is_valid_release_id(&format!("{version}{invalid}")),
                "{invalid}"
            );
        }
        assert!(!is_valid_release_id("9.9.9+dev.ci.g1a2b3c4d5e6f"));
        assert!(!is_valid_release_id(""));
        for other in [
            "9.9.9+dev.ci.g1a2b3c4d5e6f",
            "0.10.0+stable.rel.g0123456789ab",
        ] {
            assert!(is_contract_release_id(other), "{other}");
        }
        for invalid in [
            "",
            "01.0.0+dev.ci.g1a2b3c4d5e6f",
            "1.0+dev.ci.g1a2b3c4d5e6f",
            "1.0.0.0+dev.ci.g1a2b3c4d5e6f",
            "1.0.x+dev.ci.g1a2b3c4d5e6f",
            "1.0.0-rc1+dev.ci.g1a2b3c4d5e6f",
            "9.9.9+dev.ci.g1A2B3C4D5E6F",
            "9.9.9+dev.local.g1a2b3c4d5e6f",
            "999999999999999999999999999999.0.0+dev.ci.g1a2b3c4d5e6f",
        ] {
            assert!(!is_contract_release_id(invalid), "{invalid}");
        }
    }
}
