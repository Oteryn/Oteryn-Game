/// A sequenced write's success: the revision it committed or replayed, if any.
trait SequencedOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision>;
}

impl SequencedOutcome for ExperienceCommitOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        let (Self::Committed(receipt) | Self::AlreadyCommitted(receipt)) = self;
        Some(receipt.committed_character_revision)
    }
}

impl SequencedOutcome for CharacterDeathOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        let (Self::Committed(receipt) | Self::AlreadyCommitted(receipt)) = self;
        Some(receipt.committed_character_revision)
    }
}

impl SequencedOutcome for BestiaryKillOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Committed(receipt) | Self::AlreadyCommitted(receipt) => {
                Some(receipt.committed_character_revision)
            }
            Self::Saturated { .. } => None,
        }
    }
}

impl SequencedOutcome for CharmCommandOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        let (Self::Committed(receipt) | Self::AlreadyCommitted(receipt)) = self;
        Some(receipt.committed_character_revision)
    }
}

impl SequencedOutcome for BuildCommitOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        let (Self::Committed(receipt) | Self::AlreadyCommitted(receipt)) = self;
        Some(receipt.committed_character_revision)
    }
}

impl SequencedOutcome for ProficiencyCommitOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Committed(receipt) | Self::AlreadyCommitted(receipt) => {
                Some(receipt.committed_character_revision)
            }
            Self::Refused(_) => None,
        }
    }
}

impl SequencedOutcome for super::task_board::TaskBoardCommit {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        self.transition
            .as_ref()
            .map(|receipt| receipt.committed_character_revision)
    }
}

impl SequencedOutcome for QuestTransitionOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Committed(receipt) | Self::AlreadyCommitted(receipt) => {
                Some(receipt.committed_character_revision)
            }
            Self::Refused(_) | Self::ObligationClosed => None,
        }
    }
}

impl SequencedOutcome for ProficiencyModificationOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Committed(receipt) | Self::AlreadyCommitted(receipt) => {
                Some(receipt.committed_character_revision)
            }
            Self::RevisionChanged | Self::Refused(_) => None,
        }
    }
}

impl SequencedOutcome for WheelCommitOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Committed(receipt) | Self::AlreadyCommitted(receipt) => {
                Some(receipt.committed_character_revision)
            }
            Self::Refused(_) => None,
        }
    }
}

impl SequencedOutcome for WheelResetOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Reset(receipt) => Some(receipt.committed_character_revision),
            Self::Current | Self::Unresolved => None,
        }
    }
}

impl SequencedOutcome for MonkStateSaveOutcome {
    fn committed_revision(&self) -> Option<CharacterRevision> {
        match self {
            Self::Committed(receipt) | Self::AlreadyCommitted(receipt) => {
                Some(receipt.committed_character_revision)
            }
            Self::Unchanged => None,
        }
    }
}

/// A sequenced write's error, as the slot classifies it.
trait SequencedError {
    fn is_mismatch(&self) -> bool;
    /// The write may or may not have committed.
    fn is_unknown(&self) -> bool;
    fn rejected() -> Self;
    fn unavailable(error: DurabilityError) -> Self;

    fn from_authority(error: CharacterAuthorityError) -> Self
    where
        Self: Sized,
    {
        match error {
            CharacterAuthorityError::Unavailable(error) => Self::unavailable(error),
            CharacterAuthorityError::Rejected
            | CharacterAuthorityError::Conflict
            | CharacterAuthorityError::NameUnavailable => Self::rejected(),
        }
    }
}

impl SequencedError for CharacterProgressionError {
    fn is_mismatch(&self) -> bool {
        matches!(self, Self::CharacterRevisionMismatch)
    }
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unavailable(_))
    }
    fn rejected() -> Self {
        Self::AuthorityRejected
    }
    fn unavailable(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl SequencedError for BestiaryProgressError {
    fn is_mismatch(&self) -> bool {
        matches!(self, Self::CharacterRevisionMismatch)
    }
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unavailable(_))
    }
    fn rejected() -> Self {
        Self::AuthorityRejected
    }
    fn unavailable(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl SequencedError for CharmStateError {
    fn is_mismatch(&self) -> bool {
        matches!(self, Self::CharacterRevisionMismatch)
    }
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unavailable(_))
    }
    fn rejected() -> Self {
        Self::AuthorityRejected
    }
    fn unavailable(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
