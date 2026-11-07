//! Minimal test-only access to the existing private native admission fixture.
use super::{AdmissionError, FreshAdmissionFacts};

impl FreshAdmissionFacts {
    pub fn shared_test_facts(nonce: u64, lease: u64, scope: u64) -> Result<Self, AdmissionError> {
        Self::for_test(nonce, lease, scope)
    }
}
