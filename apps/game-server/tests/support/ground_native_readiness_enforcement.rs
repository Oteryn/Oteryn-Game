//! Private child reusing the existing genuine bestiary RuntimeReadiness issuer.
//! Shared by real fixture bootstrap and the owned not-ready native-writer case.
use super::{
    AdmissionAuthorityPublicationChangeV1, AdmissionAuthorityPublicationV1, RuntimeReadiness,
    TestResult, debug,
};

pub(crate) fn prepare(
    change: AdmissionAuthorityPublicationChangeV1,
    now: i64,
) -> TestResult<AdmissionAuthorityPublicationV1> {
    AdmissionAuthorityPublicationV1::prepare(&RuntimeReadiness(change), now)
        .map_err(|error| debug(error).into())
}
