//! Structured, bounded and secret-safe diagnostic contracts.
//!
//! This crate defines values that future sinks may consume. It does not install
//! a logger or subscriber, start background work, write files, send telemetry,
//! create crash reports or participate in application correctness.

use oteryn_foundation::{ClientSessionEpoch, ClientTaskGeneration, Moment, ProcessGeneration};
use std::fmt::{self, Debug, Display, Formatter};
use std::time::Duration;

pub const MAX_SAFE_TEXT_BYTES: usize = 160;
pub const MAX_FIELD_KEY_BYTES: usize = 32;
pub const MAX_EVENT_FIELDS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl Display for Severity {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticCategory {
    Lifecycle,
    Security,
    Validation,
    Resource,
    Performance,
    Internal,
}

impl Display for DiagnosticCategory {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Lifecycle => "lifecycle",
            Self::Security => "security",
            Self::Validation => "validation",
            Self::Resource => "resource",
            Self::Performance => "performance",
            Self::Internal => "internal",
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticCode(u32);

impl DiagnosticCode {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "D{:08}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticTextError {
    Empty,
    TooLong { max_bytes: usize },
    InvalidCharacters,
}

impl Display for StaticTextError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("diagnostic static text cannot be empty"),
            Self::TooLong { max_bytes } => {
                write!(
                    formatter,
                    "diagnostic static text exceeds {max_bytes} bytes"
                )
            }
            Self::InvalidCharacters => {
                formatter.write_str("diagnostic static text contains invalid characters")
            }
        }
    }
}

impl std::error::Error for StaticTextError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SafeText(&'static str);

impl SafeText {
    pub fn trusted_static(value: &'static str) -> Result<Self, StaticTextError> {
        if value.is_empty() {
            return Err(StaticTextError::Empty);
        }
        if value.len() > MAX_SAFE_TEXT_BYTES {
            return Err(StaticTextError::TooLong {
                max_bytes: MAX_SAFE_TEXT_BYTES,
            });
        }
        if value.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(StaticTextError::InvalidCharacters);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Display for SafeText {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldKey(&'static str);

impl FieldKey {
    pub fn trusted_static(value: &'static str) -> Result<Self, StaticTextError> {
        if value.is_empty() {
            return Err(StaticTextError::Empty);
        }
        if value.len() > MAX_FIELD_KEY_BYTES {
            return Err(StaticTextError::TooLong {
                max_bytes: MAX_FIELD_KEY_BYTES,
            });
        }
        let mut bytes = value.bytes();
        let valid_first = bytes.next().is_some_and(|byte| byte.is_ascii_lowercase());
        let valid_rest =
            bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        if !valid_first || !valid_rest {
            return Err(StaticTextError::InvalidCharacters);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Display for FieldKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SensitiveKind {
    AccessToken,
    RefreshToken,
    AuthorizationCode,
    PkceVerifier,
    GameTicket,
    SessionSecret,
    Cookie,
    PrivateChat,
    PersonalPath,
    Confidential,
}

impl Display for SensitiveKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AccessToken => "access-token",
            Self::RefreshToken => "refresh-token",
            Self::AuthorizationCode => "authorization-code",
            Self::PkceVerifier => "pkce-verifier",
            Self::GameTicket => "game-ticket",
            Self::SessionSecret => "session-secret",
            Self::Cookie => "cookie",
            Self::PrivateChat => "private-chat",
            Self::PersonalPath => "personal-path",
            Self::Confidential => "confidential",
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SensitiveValue {
    kind: SensitiveKind,
}

impl SensitiveValue {
    #[must_use]
    pub const fn redacted(kind: SensitiveKind, _value: &str) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> SensitiveKind {
        self.kind
    }
}

impl Debug for SensitiveValue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "SensitiveValue(<redacted:{}>)", self.kind)
    }
}

impl Display for SensitiveValue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "<redacted:{}>", self.kind)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticValue {
    SafeText(SafeText),
    Unsigned(u64),
    Signed(i64),
    Boolean(bool),
    Duration(Duration),
    Moment(Moment),
    ProcessGeneration(ProcessGeneration),
    ClientSessionEpoch(ClientSessionEpoch),
    ClientTaskGeneration(ClientTaskGeneration),
    Redacted(SensitiveValue),
}

impl DiagnosticValue {
    #[must_use]
    pub const fn redacted(kind: SensitiveKind, value: &str) -> Self {
        Self::Redacted(SensitiveValue::redacted(kind, value))
    }
}

impl Display for DiagnosticValue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SafeText(value) => Display::fmt(value, formatter),
            Self::Unsigned(value) => Display::fmt(value, formatter),
            Self::Signed(value) => Display::fmt(value, formatter),
            Self::Boolean(value) => Display::fmt(value, formatter),
            Self::Duration(value) => write_duration(formatter, *value),
            Self::Moment(value) => Display::fmt(value, formatter),
            Self::ProcessGeneration(value) => Display::fmt(value, formatter),
            Self::ClientSessionEpoch(value) => Display::fmt(value, formatter),
            Self::ClientTaskGeneration(value) => Display::fmt(value, formatter),
            Self::Redacted(value) => Display::fmt(value, formatter),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CorrelationId(u64);

impl CorrelationId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl Display for CorrelationId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "C{:016x}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechnicalContext {
    occurred_at: Moment,
    process_generation: ProcessGeneration,
    session_generation: Option<ClientSessionEpoch>,
    task_generation: Option<ClientTaskGeneration>,
    correlation_id: Option<CorrelationId>,
}

impl TechnicalContext {
    #[must_use]
    pub const fn new(occurred_at: Moment, process_generation: ProcessGeneration) -> Self {
        Self {
            occurred_at,
            process_generation,
            session_generation: None,
            task_generation: None,
            correlation_id: None,
        }
    }

    #[must_use]
    pub const fn with_session(mut self, generation: ClientSessionEpoch) -> Self {
        self.session_generation = Some(generation);
        self
    }

    #[must_use]
    pub const fn with_task(mut self, generation: ClientTaskGeneration) -> Self {
        self.task_generation = Some(generation);
        self
    }

    #[must_use]
    pub const fn with_correlation(mut self, correlation_id: CorrelationId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    #[must_use]
    pub const fn occurred_at(self) -> Moment {
        self.occurred_at
    }

    #[must_use]
    pub const fn process_generation(self) -> ProcessGeneration {
        self.process_generation
    }

    #[must_use]
    pub const fn session_generation(self) -> Option<ClientSessionEpoch> {
        self.session_generation
    }

    #[must_use]
    pub const fn task_generation(self) -> Option<ClientTaskGeneration> {
        self.task_generation
    }

    #[must_use]
    pub const fn correlation_id(self) -> Option<CorrelationId> {
        self.correlation_id
    }
}

impl Display for TechnicalContext {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "t={} process={}",
            self.occurred_at, self.process_generation
        )?;
        if let Some(generation) = self.session_generation {
            write!(formatter, " session={generation}")?;
        }
        if let Some(generation) = self.task_generation {
            write!(formatter, " task={generation}")?;
        }
        if let Some(correlation_id) = self.correlation_id {
            write!(formatter, " correlation={correlation_id}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticField {
    key: FieldKey,
    value: DiagnosticValue,
}

impl DiagnosticField {
    #[must_use]
    pub const fn new(key: FieldKey, value: DiagnosticValue) -> Self {
        Self { key, value }
    }

    #[must_use]
    pub const fn key(self) -> FieldKey {
        self.key
    }

    #[must_use]
    pub const fn value(self) -> DiagnosticValue {
        self.value
    }
}

impl Display for DiagnosticField {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}={}", self.key, self.value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticBuildError {
    TooManyFields { max_fields: usize },
    DuplicateField,
}

impl Display for DiagnosticBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyFields { max_fields } => {
                write!(formatter, "diagnostic event exceeds {max_fields} fields")
            }
            Self::DuplicateField => {
                formatter.write_str("diagnostic event contains a duplicate field key")
            }
        }
    }
}

impl std::error::Error for DiagnosticBuildError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticEvent {
    severity: Severity,
    category: DiagnosticCategory,
    code: DiagnosticCode,
    message: SafeText,
    context: TechnicalContext,
    fields: Vec<DiagnosticField>,
}

impl DiagnosticEvent {
    #[must_use]
    pub fn new(
        severity: Severity,
        category: DiagnosticCategory,
        code: DiagnosticCode,
        message: SafeText,
        context: TechnicalContext,
    ) -> Self {
        Self {
            severity,
            category,
            code,
            message,
            context,
            fields: Vec::new(),
        }
    }

    pub fn try_add_field(&mut self, field: DiagnosticField) -> Result<(), DiagnosticBuildError> {
        if self.fields.len() >= MAX_EVENT_FIELDS {
            return Err(DiagnosticBuildError::TooManyFields {
                max_fields: MAX_EVENT_FIELDS,
            });
        }
        if self.fields.iter().any(|existing| existing.key == field.key) {
            return Err(DiagnosticBuildError::DuplicateField);
        }
        self.fields.push(field);
        Ok(())
    }

    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    #[must_use]
    pub const fn category(&self) -> DiagnosticCategory {
        self.category
    }

    #[must_use]
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    #[must_use]
    pub const fn message(&self) -> SafeText {
        self.message
    }

    #[must_use]
    pub const fn context(&self) -> TechnicalContext {
        self.context
    }

    #[must_use]
    pub fn fields(&self) -> &[DiagnosticField] {
        &self.fields
    }

    #[must_use]
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }
}

impl Display for DiagnosticEvent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}/{} [{}] {}",
            self.severity, self.category, self.code, self.context, self.message
        )?;
        for field in &self.fields {
            write!(formatter, " {field}")?;
        }
        Ok(())
    }
}

fn write_duration(formatter: &mut Formatter<'_>, duration: Duration) -> fmt::Result {
    write!(
        formatter,
        "{}.{:09}s",
        duration.as_secs(),
        duration.subsec_nanos()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_value_discards_runtime_text() {
        let marker = "synthetic-secret-marker";
        let value = DiagnosticValue::redacted(SensitiveKind::SessionSecret, marker);
        assert!(!value.to_string().contains(marker));
        assert!(!format!("{value:?}").contains(marker));
    }

    #[test]
    fn event_fields_are_unique_and_bounded() -> Result<(), Box<dyn std::error::Error>> {
        let mut event = DiagnosticEvent::new(
            Severity::Warning,
            DiagnosticCategory::Validation,
            DiagnosticCode::new(21),
            SafeText::trusted_static("operation rejected")?,
            TechnicalContext::new(Moment::ZERO, ProcessGeneration::new(3))
                .with_session(ClientSessionEpoch::new(5))
                .with_task(ClientTaskGeneration::new(8)),
        );
        let key = FieldKey::trusted_static("attempt")?;
        event.try_add_field(DiagnosticField::new(key, DiagnosticValue::Unsigned(1)))?;
        assert_eq!(
            event.try_add_field(DiagnosticField::new(key, DiagnosticValue::Unsigned(2))),
            Err(DiagnosticBuildError::DuplicateField)
        );
        Ok(())
    }
}
