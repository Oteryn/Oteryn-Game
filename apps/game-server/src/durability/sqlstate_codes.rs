//! The registered code of each SQLSTATE the game database raises (ERR-NODE-1, `E3001`–`E3009`).

oteryn_error_codes::error_kinds! {
    /// A registered database SQLSTATE.
    pub enum SqlstateKind {
        RegistrationRejected = (3001, "REGISTRATION_REJECTED", Conflict, Terminal),
        NodeIncarnationNotCurrent = (3002, "NODE_INCARNATION_NOT_CURRENT", StaleGeneration, Terminal),
        DescriptorIssuanceConflict = (3003, "DESCRIPTOR_ISSUANCE_CONFLICT", Conflict, Terminal),
        InboxItemHasContents = (3004, "INBOX_ITEM_HAS_CONTENTS", InvalidInput, Terminal),
        InboxWorldMismatch = (3005, "INBOX_WORLD_MISMATCH", Conflict, Terminal),
        InboxItemInAnotherLocation = (3006, "INBOX_ITEM_IN_ANOTHER_LOCATION", Conflict, Terminal),
        InboxUnknownCauseKind = (3007, "INBOX_UNKNOWN_CAUSE_KIND", InvalidInput, Terminal),
        InboxAlreadyDelivered = (3008, "INBOX_ALREADY_DELIVERED", Conflict, Terminal),
        ContentActivationSequenceConflict = (3009, "CONTENT_ACTIVATION_SEQUENCE_CONFLICT", Conflict, Terminal),
    }
}

oteryn_error_codes::error_kinds! {
    /// A tool failure that carries no registered SQLSTATE.
    pub enum ToolKind {
        MigrationFailed = (3010, "MIGRATION_FAILED", InternalUnavailable, Terminal),
        ProficiencyImportFailed = (3011, "PROFICIENCY_IMPORT_FAILED", InternalUnavailable, Terminal),
    }
}

impl SqlstateKind {
    /// The kind of a registered SQLSTATE; `None` for any other state.
    #[must_use]
    pub fn from_sqlstate(state: &str) -> Option<Self> {
        Some(match state {
            "OTN01" => Self::RegistrationRejected,
            "OTN02" => Self::NodeIncarnationNotCurrent,
            "OTN03" => Self::DescriptorIssuanceConflict,
            "OTI01" => Self::InboxItemHasContents,
            "OTI02" => Self::InboxWorldMismatch,
            "OTI03" => Self::InboxItemInAnotherLocation,
            "OTI04" => Self::InboxUnknownCauseKind,
            "OTI05" => Self::InboxAlreadyDelivered,
            "OTC01" => Self::ContentActivationSequenceConflict,
            _ => return None,
        })
    }

    /// The registered SQLSTATE of this kind.
    #[must_use]
    pub const fn sqlstate(self) -> &'static str {
        match self {
            Self::RegistrationRejected => "OTN01",
            Self::NodeIncarnationNotCurrent => "OTN02",
            Self::DescriptorIssuanceConflict => "OTN03",
            Self::InboxItemHasContents => "OTI01",
            Self::InboxWorldMismatch => "OTI02",
            Self::InboxItemInAnotherLocation => "OTI03",
            Self::InboxUnknownCauseKind => "OTI04",
            Self::InboxAlreadyDelivered => "OTI05",
            Self::ContentActivationSequenceConflict => "OTC01",
        }
    }

    /// The registered kind of a failure: the first SQLSTATE on its source chain.
    #[must_use]
    pub fn of_error(error: &(dyn std::error::Error + 'static)) -> Option<Self> {
        let mut current = Some(error);
        while let Some(error) = current {
            if let Some(sqlx::Error::Database(database)) = error.downcast_ref::<sqlx::Error>()
                && let Some(kind) = database.code().as_deref().and_then(Self::from_sqlstate)
            {
                return Some(kind);
            }
            if let Some(kind) = error
                .downcast_ref::<super::DurabilityError>()
                .and_then(super::DurabilityError::sqlstate)
            {
                return Some(kind);
            }
            current = error.source();
        }
        None
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;

    /// Every code is the registry's, with the same name, category and progression.
    pub(crate) fn assert_in_registry(codes: &[oteryn_error_codes::ErrorCode]) {
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json"
        ))
        .expect("game registry");
        for code in codes {
            let entry = registry["codes"]
                .as_array()
                .expect("codes")
                .iter()
                .find(|entry| entry["code"].as_u64() == Some(u64::from(code.number)))
                .unwrap_or_else(|| panic!("E{} is not registered", code.number));
            assert_eq!(entry["name"], code.name, "E{}", code.number);
            assert_eq!(
                entry["category"],
                code.category().as_str(),
                "E{}",
                code.number
            );
            assert_eq!(
                entry["progression"],
                code.progression().as_str(),
                "E{}",
                code.number
            );
        }
    }

    #[test]
    fn sqlstate_codes_match_the_registry() {
        assert_in_registry(
            &SqlstateKind::ALL
                .iter()
                .map(|kind| kind.code())
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn tool_codes_match_the_registry() {
        assert_in_registry(
            &ToolKind::ALL
                .iter()
                .map(|kind| kind.code())
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn every_kind_round_trips_its_sqlstate() {
        for kind in SqlstateKind::ALL {
            assert_eq!(SqlstateKind::from_sqlstate(kind.sqlstate()), Some(*kind));
        }
        assert_eq!(SqlstateKind::from_sqlstate("23503"), None);
        assert_eq!(SqlstateKind::ALL.len(), 9);
    }

    #[derive(Debug)]
    struct Raised(&'static str);

    impl std::fmt::Display for Raised {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("raised")
        }
    }

    impl std::error::Error for Raised {}

    impl sqlx::error::DatabaseError for Raised {
        fn message(&self) -> &str {
            "raised"
        }

        fn code(&self) -> Option<std::borrow::Cow<'_, str>> {
            Some(self.0.into())
        }

        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }

        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::Other
        }
    }

    fn raised(state: &'static str) -> sqlx::Error {
        sqlx::Error::Database(Box::new(Raised(state)))
    }

    #[test]
    fn a_registered_sqlstate_survives_the_durability_error() {
        let error = super::super::DurabilityError::from(raised("OTN01"));
        assert_eq!(
            SqlstateKind::of_error(&error),
            Some(SqlstateKind::RegistrationRejected)
        );
        let boxed: Box<dyn std::error::Error> = Box::new(error);
        assert_eq!(
            SqlstateKind::of_error(boxed.as_ref()),
            Some(SqlstateKind::RegistrationRejected)
        );
    }

    #[test]
    fn a_registered_sqlstate_survives_a_migration_error() {
        let error = super::super::DurabilityError::from(sqlx::migrate::MigrateError::Execute(
            raised("OTC01"),
        ));
        assert_eq!(
            SqlstateKind::of_error(&error),
            Some(SqlstateKind::ContentActivationSequenceConflict)
        );
    }

    #[test]
    fn an_unregistered_sqlstate_has_no_kind() {
        let error = super::super::DurabilityError::from(raised("23503"));
        assert_eq!(SqlstateKind::of_error(&error), None);
    }

    #[test]
    fn a_plain_error_has_no_kind() {
        assert_eq!(SqlstateKind::of_error(&std::io::Error::other("x")), None);
        assert_eq!(SqlstateKind::of_error(&sqlx::Error::RowNotFound), None);
    }
}
