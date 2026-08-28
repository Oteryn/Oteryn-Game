//! Deterministic, test-owned helpers for Oteryn technical contracts.

use oteryn_diagnostics::{
    CorrelationId, DiagnosticBuildError, DiagnosticCategory, DiagnosticCode, DiagnosticEvent,
    DiagnosticField, DiagnosticValue, FieldKey, SafeText, Severity, StaticTextError,
    TechnicalContext,
};
use oteryn_foundation::{
    ClientSessionEpoch, ClientTaskGeneration, ManualClock, Moment, MonotonicClock,
    ProcessGeneration, TimeError,
};
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TestTimeline {
    clock: ManualClock,
    process_generation: ProcessGeneration,
}

impl TestTimeline {
    #[must_use]
    pub fn new(start: Moment, process_generation: ProcessGeneration) -> Self {
        Self {
            clock: ManualClock::new(start),
            process_generation,
        }
    }

    #[must_use]
    pub fn clock(&self) -> ManualClock {
        self.clock.clone()
    }

    #[must_use]
    pub fn now(&self) -> Moment {
        self.clock.now()
    }

    #[must_use]
    pub const fn process_generation(&self) -> ProcessGeneration {
        self.process_generation
    }

    pub fn advance(&self, duration: Duration) -> Result<Moment, TimeError> {
        self.clock.advance(duration)
    }

    pub fn try_set(&self, requested: Moment) -> Result<(), TimeError> {
        self.clock.try_set(requested)
    }

    #[must_use]
    pub fn context(
        &self,
        session_epoch: Option<ClientSessionEpoch>,
        task_generation: Option<ClientTaskGeneration>,
        correlation_id: Option<CorrelationId>,
    ) -> TechnicalContext {
        let mut context = TechnicalContext::new(self.now(), self.process_generation);
        if let Some(generation) = session_epoch {
            context = context.with_session(generation);
        }
        if let Some(generation) = task_generation {
            context = context.with_task(generation);
        }
        if let Some(correlation) = correlation_id {
            context = context.with_correlation(correlation);
        }
        context
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSupportError {
    StaticText(StaticTextError),
    DiagnosticBuild(DiagnosticBuildError),
}

impl Display for TestSupportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaticText(error) => Display::fmt(error, formatter),
            Self::DiagnosticBuild(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for TestSupportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::StaticText(error) => Some(error),
            Self::DiagnosticBuild(error) => Some(error),
        }
    }
}

impl From<StaticTextError> for TestSupportError {
    fn from(error: StaticTextError) -> Self {
        Self::StaticText(error)
    }
}

impl From<DiagnosticBuildError> for TestSupportError {
    fn from(error: DiagnosticBuildError) -> Self {
        Self::DiagnosticBuild(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticEventFixture {
    event: DiagnosticEvent,
}

impl DiagnosticEventFixture {
    pub fn new(
        severity: Severity,
        category: DiagnosticCategory,
        code: DiagnosticCode,
        message: &'static str,
        context: TechnicalContext,
    ) -> Result<Self, TestSupportError> {
        let message = SafeText::trusted_static(message)?;
        Ok(Self {
            event: DiagnosticEvent::new(severity, category, code, message, context),
        })
    }

    pub fn try_add_field(
        &mut self,
        key: &'static str,
        value: DiagnosticValue,
    ) -> Result<(), TestSupportError> {
        let key = FieldKey::trusted_static(key)?;
        self.event.try_add_field(DiagnosticField::new(key, value))?;
        Ok(())
    }

    #[must_use]
    pub const fn event(&self) -> &DiagnosticEvent {
        &self.event
    }

    #[must_use]
    pub fn build(self) -> DiagnosticEvent {
        self.event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_diagnostics::SensitiveKind;

    #[test]
    fn timeline_and_context_are_deterministic() -> Result<(), Box<dyn std::error::Error>> {
        let timeline = TestTimeline::new(Moment::ZERO, ProcessGeneration::new(2));
        timeline.advance(Duration::from_secs(3))?;
        let context = timeline.context(
            Some(ClientSessionEpoch::new(5)),
            Some(ClientTaskGeneration::new(8)),
            Some(CorrelationId::new(13)),
        );
        assert_eq!(context.occurred_at(), timeline.now());
        assert_eq!(context.process_generation(), ProcessGeneration::new(2));
        assert_eq!(
            context.session_generation(),
            Some(ClientSessionEpoch::new(5))
        );
        assert_eq!(
            context.task_generation(),
            Some(ClientTaskGeneration::new(8))
        );
        Ok(())
    }

    #[test]
    fn diagnostic_fixture_preserves_redaction() -> Result<(), TestSupportError> {
        let marker = "synthetic-secret-shaped-marker";
        let mut fixture = DiagnosticEventFixture::new(
            Severity::Info,
            DiagnosticCategory::Internal,
            DiagnosticCode::new(7),
            "synthetic event",
            TechnicalContext::new(Moment::ZERO, ProcessGeneration::new(1)),
        )?;
        fixture.try_add_field(
            "sensitive_value",
            DiagnosticValue::redacted(SensitiveKind::Confidential, marker),
        )?;
        let event = fixture.build();
        assert!(!event.to_string().contains(marker));
        assert!(!format!("{event:?}").contains(marker));
        Ok(())
    }
}
