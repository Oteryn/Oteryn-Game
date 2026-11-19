use super::fixture::{self as harness_fixture, Harness, debug, id};

use super::{Dependencies, TestResult};

use crate::gameplay_transport::connection::AdmittedSession;


#[path = "cyclopedia/fixture.rs"]
pub(super) mod support;

pub(super) fn pool(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes[12..20].try_into().expect("fixed result field"))
}
