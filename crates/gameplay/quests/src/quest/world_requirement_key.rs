//! Syntax for durable World requirements; the World owner still decides admission.
pub(super) fn valid(key: &str) -> bool {
    if key.is_empty() || key.len() > 100 {
        return false;
    }
    if let Some((family, city)) = key.split_once('/') {
        return family == "fury_gates"
            && !city.is_empty()
            && city.bytes().all(|byte| byte.is_ascii_lowercase());
    }
    key.bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests {
    #[test]
    fn legacy_names_and_exact_fury_cities_have_bounded_syntax() {
        for key in [
            "grimvale",
            "deeplings",
            "fury_gates",
            "fury_gates/thais",
            "fury_gates/libertybay",
        ] {
            assert!(super::valid(key), "{key}");
        }
    }

    #[test]
    fn malformed_or_cross_family_paths_are_rejected() {
        for key in [
            "",
            "fury_gates/",
            "/thais",
            "fury_gates/Thais",
            "fury_gates/thais/exit",
            "fury_gates/../thais",
            "grimvale/thais",
            "fury_gates/thais@1",
            " fury_gates/thais",
        ] {
            assert!(!super::valid(key), "{key}");
        }
        assert!(!super::valid(&"a".repeat(101)));
    }
}
