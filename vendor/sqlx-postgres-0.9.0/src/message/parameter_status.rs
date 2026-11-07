use sqlx_core::bytes::Bytes;

use crate::error::Error;
use crate::io::BufExt;
use crate::message::{BackendMessage, BackendMessageFormat};

#[derive(Debug)]
pub struct ParameterStatus {
    pub name: String,
    pub value: String,
}

pub(crate) fn wp3_borrowed_server_version(buf: &[u8]) -> Result<Option<&str>, Error> {
    let name_end = buf
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| err_protocol!("unterminated WP3 ParameterStatus name"))?;
    let name = std::str::from_utf8(&buf[..name_end])
        .map_err(|_| err_protocol!("WP3 ParameterStatus name is not UTF-8"))?;
    let value_tail = buf
        .get(name_end + 1..)
        .ok_or_else(|| err_protocol!("truncated WP3 ParameterStatus value"))?;
    let value_end = value_tail
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| err_protocol!("unterminated WP3 ParameterStatus value"))?;
    if value_end + 1 != value_tail.len() {
        return Err(err_protocol!("WP3 ParameterStatus has trailing bytes"));
    }
    let value = std::str::from_utf8(&value_tail[..value_end])
        .map_err(|_| err_protocol!("WP3 ParameterStatus value is not UTF-8"))?;
    Ok((name == "server_version").then_some(value))
}

impl BackendMessage for ParameterStatus {
    const FORMAT: BackendMessageFormat = BackendMessageFormat::ParameterStatus;

    fn decode_body(mut buf: Bytes) -> Result<Self, Error> {
        let name = buf.get_str_nul()?;
        let value = buf.get_str_nul()?;

        Ok(Self { name, value })
    }
}

#[test]
fn test_decode_parameter_status() {
    const DATA: &[u8] = b"client_encoding\x00UTF8\x00";

    let m = ParameterStatus::decode_body(DATA.into()).unwrap();

    assert_eq!(&m.name, "client_encoding");
    assert_eq!(&m.value, "UTF8")
}

#[test]
fn test_decode_empty_parameter_status() {
    const DATA: &[u8] = b"\x00\x00";

    let m = ParameterStatus::decode_body(DATA.into()).unwrap();

    assert!(m.name.is_empty());
    assert!(m.value.is_empty());
}

#[test]
fn wp3_parameter_status_retains_only_normalized_server_version() {
    assert_eq!(
        wp3_borrowed_server_version(b"server_version\017.6\0").unwrap(),
        Some("17.6")
    );
    assert_eq!(
        wp3_borrowed_server_version(b"application_name\0peer-controlled\0").unwrap(),
        None
    );
    assert!(wp3_borrowed_server_version(b"server_version\017.6\0trailing").is_err());
    assert!(wp3_borrowed_server_version(b"server_version\0unterminated").is_err());
}

#[cfg(all(test, not(debug_assertions)))]
#[bench]
fn bench_decode_parameter_status(b: &mut test::Bencher) {
    const DATA: &[u8] = b"client_encoding\x00UTF8\x00";

    b.iter(|| {
        ParameterStatus::decode_body(test::black_box(Bytes::from_static(DATA))).unwrap();
    });
}

#[test]
fn test_decode_parameter_status_response() {
    const PARAMETER_STATUS_RESPONSE: &[u8] = b"crdb_version\0CockroachDB CCL v21.1.0 (x86_64-unknown-linux-gnu, built 2021/05/17 13:49:40, go1.15.11)\0";

    let message = ParameterStatus::decode_body(Bytes::from(PARAMETER_STATUS_RESPONSE)).unwrap();

    assert_eq!(message.name, "crdb_version");
    assert_eq!(
        message.value,
        "CockroachDB CCL v21.1.0 (x86_64-unknown-linux-gnu, built 2021/05/17 13:49:40, go1.15.11)"
    );
}
