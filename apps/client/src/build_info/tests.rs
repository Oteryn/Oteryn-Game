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

#[test]
fn native_login_only_offers_release_to_the_confirmed_gateway() {
    let release = ReleaseIdentity::new(UpdateChannel::Stable, "rel-42", 42);
    assert!(release.is_some());
    let staging = "https://gateway.staging.example/";
    let production = "https://gateway.example/";
    assert_eq!(release_for_gateway(staging, None, release), None);
    assert_eq!(
        release_for_gateway(production, Some(staging), release),
        None
    );
    assert_eq!(
        release_for_gateway(staging, Some(staging), release),
        release
    );
}

#[test]
fn build_gate_release_matches_the_effective_login_endpoint() {
    let release = ReleaseIdentity::new(UpdateChannel::Stable, "rel-42", 42);
    let confirmed = Some("https://gateway.example/");
    for gateway in [
        "https://gateway.example",
        "https://GATEWAY.example:443/",
        "https://gateway.example/legacy",
    ] {
        assert_eq!(release_for_gateway(gateway, confirmed, release), release);
    }
    for gateway in [
        "https://gateway.example:444/",
        "https://gateway.example/other/",
    ] {
        assert_eq!(release_for_gateway(gateway, confirmed, release), None);
    }
}
