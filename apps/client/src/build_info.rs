use oteryn_platform_client::PlatformClientConfig;
use oteryn_platform_client::native_login::ReleaseIdentity;
use oteryn_platform_client::updates::{MAX_RELEASE_SEQUENCE, UpdateChannel};

// Downloads release IDs have a different grammar from the installer's diagnostic build ID.
const UPDATE_RELEASE_INPUTS: (Option<&str>, Option<&str>, Option<&str>) = (
    option_env!("OTERYN_CLIENT_UPDATE_CHANNEL"),
    option_env!("OTERYN_CLIENT_UPDATE_RELEASE_ID"),
    option_env!("OTERYN_CLIENT_UPDATE_RELEASE_SEQUENCE"),
);
const UPDATE_RELEASE: Option<ReleaseIdentity> = match UPDATE_RELEASE_INPUTS {
    (Some(channel), Some(id), Some(sequence)) => match channel.as_bytes() {
        b"stable" => ReleaseIdentity::new(UpdateChannel::Stable, id, release_sequence(sequence)),
        b"beta" => ReleaseIdentity::new(UpdateChannel::Beta, id, release_sequence(sequence)),
        _ => None,
    },
    _ => None,
};
const _: () = assert!(
    matches!(UPDATE_RELEASE_INPUTS, (None, None, None)) || UPDATE_RELEASE.is_some(),
    "Set a valid OTERYN_CLIENT_UPDATE_CHANNEL, OTERYN_CLIENT_UPDATE_RELEASE_ID and OTERYN_CLIENT_UPDATE_RELEASE_SEQUENCE"
);

// Setting this URL acknowledges that this Gateway and its issuer accept offer.release.
const BUILD_GATE_GATEWAY: Option<&str> = option_env!("OTERYN_CLIENT_BUILD_GATE_GATEWAY_URL");
const _: () = assert!(
    BUILD_GATE_GATEWAY.is_none() || UPDATE_RELEASE.is_some(),
    "A build gate Gateway requires a Downloads release identity"
);

const fn release_sequence(value: &str) -> u64 {
    let bytes = value.as_bytes();
    let mut sequence = 0;
    let mut i = 0;
    while i < bytes.len() {
        assert!(
            bytes[i].is_ascii_digit(),
            "Release sequence must be decimal"
        );
        sequence = sequence * 10 + (bytes[i] - b'0') as u64;
        assert!(
            sequence <= MAX_RELEASE_SEQUENCE,
            "Release sequence exceeds 2^53-1"
        );
        i += 1;
    }
    sequence
}

pub(crate) fn login_release(gateway: &str) -> Option<ReleaseIdentity> {
    release_for_gateway(gateway, BUILD_GATE_GATEWAY, UPDATE_RELEASE)
}

fn release_for_gateway(
    gateway: &str,
    confirmed: Option<&str>,
    release: Option<ReleaseIdentity>,
) -> Option<ReleaseIdentity> {
    let actual = PlatformClientConfig::new(gateway).ok()?;
    let confirmed = PlatformClientConfig::new(confirmed?).ok()?;
    release.filter(|_| actual.matches_gateway_login_endpoint(&confirmed))
}

#[cfg(windows)]
pub(crate) fn updater_release(channel: &str) -> Option<ReleaseIdentity> {
    UPDATE_RELEASE.filter(|release| release.update_channel().as_str() == channel)
}

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
mod tests;
