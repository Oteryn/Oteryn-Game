use super::*;

#[test]
fn uuid_string_matches_postgres_text() {
    let value: Vec<u8> = (0u8..16).collect();
    assert_eq!(uuid_string(&value), "00010203-0405-0607-0809-0a0b0c0d0e0f");
}
