// APP-3 wrappers over the single existing fresh-admission fixture implementation.
impl Source {
    pub(crate) fn appearance_observer(
        now: i64,
        account: String,
        character: CharacterId,
    ) -> TestResult<Self> {
        let mut current = Self::new(now)?.current;
        current.account_id = account;
        current.character_id = character;
        Self::with_current(now, current)
    }
    pub(crate) fn appearance_observer_request(
        &self,
        character: &str,
        now: i64,
    ) -> TestResult<FreshAdmissionCommitRequestV1> {
        self.request_for(
            checked(GameSessionId::decode(&super::super::fixture::id(110)))?,
            checked(AuthenticatedTransportRefV1::decode(&[8; 16]))?,
            Some((&self.current.account_id, character)),
            now,
        )
    }
}

impl Source {
    // Use the owning publisher's exact current signing revision, including its source time.
    // The observer refreshes this publication; it never reconstructs authority from a receipt.
    fn published_signing_trust(
        &self,
    ) -> Result<FreshSigningTrustObservationV1, Fnd04EvidenceError> {
        let row = self
            .rows
            .iter()
            .find(|row| matches!(&row.key, AdmissionAuthorityGuardKeyV1::SigningTrust { key_id, profile } if key_id == "fresh-1" && profile == PRE_ADMISSION_PROFILE))
            .ok_or(Fnd04EvidenceError::ExplicitlyDenied)?;
        let mut proof = provenance(
            FreshEvidencePurposeV1::SigningTrust,
            row.source.source_observed_at,
        );
        proof.source_authority = row.source.authority.clone();
        proof.source_revision = row.source.source_revision;
        proof.accepted_source_revision = row.source.source_revision;
        proof.decision_identity = row.source.decision_identity.clone();
        proof.accepted_decision_identity = row.source.decision_identity.clone();
        proof.publication_revision = row.publication_revision;
        proof.clock_uncertainty_seconds = row.source.clock_uncertainty_seconds;
        Ok(FreshSigningTrustObservationV1 {
            key_id: "fresh-1".into(),
            public_key: self.key.verifying_key().to_bytes(),
            trusted: true,
            provenance: proof,
        })
    }
}
