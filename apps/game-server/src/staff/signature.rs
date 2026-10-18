//! D1040–D1042. Verify the received bytes before decoding, against independently
//! pinned trust. A verified intent is never a COMMITTED execution receipt.
use super::audit::{ActionKind, Reason, ResultCode};
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
};
use oteryn_release_manifest::jcs;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub const REQUEST_NAMESPACE: &str = "oteryn-ops-request-v1";
pub const RECEIPT_NAMESPACE: &str = "oteryn-ops-submission-receipt-v1";
pub const MAX_REQUEST_BYTES: usize = 32_768;
pub const MAX_SIGNATURE_BYTES: usize = 8_192;
pub const MAX_ROSTER_BYTES: usize = 65_536;
pub const MAX_OUTCOME_BYTES: usize = 4_096;
pub const MAX_EVIDENCE_BATCH: i64 = 64;
pub const EXECUTION_WINDOW_MS: u64 = 600_000;
pub const FUTURE_SKEW_MS: u64 = 300_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Bounds,
    Invalid,
    Signature,
    Binding,
    Clock,
    Stale,
    Chain,
    Receipt,
    MissingMaterial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decimal(pub u64);
impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        let number = text.parse::<u64>().map_err(serde::de::Error::custom)?;
        if text != number.to_string() {
            return Err(serde::de::Error::custom("noncanonical decimal"));
        }
        Ok(Self(number))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub world_id: String,
    pub channel_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Target {
    #[serde(rename = "CHARACTER")]
    Character { character_id: String },
    #[serde(rename = "ACCOUNT")]
    Account { account_ref: String },
    #[serde(rename = "SANCTION")]
    Sanction { sanction_id: String },
    #[serde(rename = "REPORT")]
    Report { report_id: String },
    #[serde(rename = "REPORTS")]
    Reports { state: ReportState },
    #[serde(rename = "ROSTER")]
    Roster { roster_sha256: String },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReportState {
    Open,
    Closed,
    Expired,
}

/// The ALPHA request codec. Other child commands are added with their accepted
/// complete typed parameters by those allocated owners, never as opaque JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Action {
    #[serde(rename = "INSPECT")]
    Inspect { character_id: String },
    #[serde(rename = "KICK")]
    Kick { target: Target },
    #[serde(rename = "MUTE")]
    Mute {
        character_id: String,
        duration_ms: Decimal,
    },
    #[serde(rename = "UNMUTE")]
    Unmute { sanction_id: String },
    #[serde(rename = "REPORTS_LIST")]
    ReportsList { state: ReportState },
    #[serde(rename = "REPORT_CLOSE")]
    ReportClose { report_id: String },
    #[serde(rename = "REPORT_CLEAR_TEXT")]
    ReportClearText { report_id: String },
    #[serde(rename = "ROSTER_ACTIVATE")]
    RosterActivate { roster_sha256: String },
}
impl Action {
    pub fn kind(&self) -> ActionKind {
        match self {
            Self::Inspect { .. } => ActionKind::Inspect,
            Self::Kick { .. } => ActionKind::Kick,
            Self::Mute { .. } => ActionKind::Mute,
            Self::Unmute { .. } => ActionKind::Unmute,
            Self::ReportsList { .. } => ActionKind::ReportsList,
            Self::ReportClose { .. } => ActionKind::ReportClose,
            Self::ReportClearText { .. } => ActionKind::ReportClearText,
            Self::RosterActivate { .. } => ActionKind::RosterActivate,
        }
    }
    pub fn target(&self) -> Target {
        match self {
            Self::Inspect { character_id } | Self::Mute { character_id, .. } => Target::Character {
                character_id: character_id.clone(),
            },
            Self::Kick { target } => target.clone(),
            Self::Unmute { sanction_id } => Target::Sanction {
                sanction_id: sanction_id.clone(),
            },
            Self::ReportsList { state } => Target::Reports { state: *state },
            Self::ReportClose { report_id } | Self::ReportClearText { report_id } => {
                Target::Report {
                    report_id: report_id.clone(),
                }
            }
            Self::RosterActivate { roster_sha256 } => Target::Roster {
                roster_sha256: roster_sha256.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u8,
    pub operation_id: String,
    pub environment: String,
    pub roster_sha256: String,
    pub issued_at: Decimal,
    pub control_seq: Decimal,
    pub previous_link_sha256: String,
    pub scope: Option<Scope>,
    pub reason: Reason,
    pub action: Action,
}
impl Request {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        jcs::encode(self, MAX_REQUEST_BYTES).map_err(|_| Error::Invalid)
    }
    fn validate(&self) -> Result<(), Error> {
        if self.version != 1
            || !uuid(&self.operation_id)
            || !token(&self.environment)
            || !digest_text(&self.roster_sha256)
            || !digest_text(&self.previous_link_sha256)
            || self.control_seq.0 == 0
        {
            return Err(Error::Invalid);
        }
        // Only the typed, restart-pinned roster activation is environment-wide,
        // and it never names a scope: the chain applies it to every later link.
        if self.scope.is_none() != matches!(&self.action, Action::RosterActivate { .. }) {
            return Err(Error::Invalid);
        }
        if let Some(scope) = &self.scope
            && (!uuid(&scope.world_id)
                || !uuid(&scope.channel_id)
                || scope.world_id == scope.channel_id)
        {
            return Err(Error::Invalid);
        }
        let valid = match &self.action {
            Action::Inspect { character_id } => uuid(character_id),
            Action::Mute {
                character_id,
                duration_ms,
            } => uuid(character_id) && duration_ms.0 > 0,
            Action::Kick {
                target: Target::Character { character_id },
            } => uuid(character_id),
            Action::Kick {
                target: Target::Account { account_ref },
            } => digest_text(account_ref),
            Action::Kick { .. } => false,
            Action::Unmute { sanction_id } => uuid(sanction_id),
            Action::ReportClose { report_id } | Action::ReportClearText { report_id } => {
                uuid(report_id)
            }
            Action::ReportsList { .. } => true,
            Action::RosterActivate { roster_sha256 } => {
                roster_sha256 == &self.roster_sha256 && self.reason == Reason::Roster
            }
        };
        if !valid {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Disposition {
    Committed,
    Refused,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub disposition: Disposition,
    pub result: ResultCode,
    pub action_id: String,
    pub transaction_id: String,
    /// Authoritative execution timestamp, covered by the operator's receipt
    /// outcome digest; it also anchors audit-expiry verification after pruning.
    pub occurred_at: Decimal,
}
impl Outcome {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, Error> {
        if !uuid(&self.action_id) || !uuid(&self.transaction_id) || self.occurred_at.0 == 0 {
            return Err(Error::Invalid);
        }
        jcs::encode(self, MAX_OUTCOME_BYTES).map_err(|_| Error::Invalid)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub version: u8,
    pub environment: String,
    pub roster_sha256: String,
    pub operation_id: String,
    pub request_sha256: String,
    pub outcome_sha256: String,
    pub disposition: Disposition,
    pub issued_at: Decimal,
    pub control_seq: Decimal,
    pub previous_link_sha256: String,
}

/// The transport envelope has no executable unsigned sibling fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedBytes {
    pub principal: String,
    pub bytes: Vec<u8>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message", content = "signed", deny_unknown_fields)]
pub enum ControlMessage {
    Request(SignedBytes),
    Receipt(SignedBytes),
}
pub const CONTROL_PREFIX: &[u8] = b"OTERYN-STAFF-AUDIT/1\n";
pub const MAX_CONTROL_BYTES: usize = MAX_REQUEST_BYTES * 4 + MAX_SIGNATURE_BYTES + 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustConfig {
    pub environment: String,
    pub allowed_signers_file: PathBuf,
    pub allowed_signers_sha256: String,
    pub newest_activation_sha256: String,
}
impl TrustConfig {
    pub fn load(&self) -> Result<VerifierTrust, Error> {
        VerifierTrust::load(
            self.environment.clone(),
            &self.allowed_signers_file,
            self.allowed_signers_sha256.clone(),
            self.newest_activation_sha256.clone(),
        )
    }
}

/// Independently reviewed off-host checkout record; never reconstructed from
/// links or their DB rows. Config/history paths are restart-only inputs.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationConfig {
    pub control_seq: Decimal,
    pub trust: TrustConfig,
}
pub struct Activation {
    pub sequence: u64,
    pub trust: VerifierTrust,
}
pub struct History {
    activations: Vec<Activation>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Request,
    Receipt,
    Activation,
}
#[derive(Debug, Clone)]
pub struct ChainLink {
    pub sequence: u64,
    pub previous: String,
    pub digest: String,
    pub roster: String,
    pub fingerprint: String,
    pub signed: SignedBytes,
    pub kind: LinkKind,
}
pub enum VerifiedLink {
    Request(VerifiedRequest),
    Receipt(VerifiedReceipt),
}
pub struct HistoryCursor<'a> {
    history: &'a History,
    active: usize,
    next: Option<u64>,
    previous: Option<String>,
    wanted_digest: Option<String>,
    matched_request: Option<VerifiedRequest>,
}
/// Terminal lookup evidence only. This type grants no current signer authority
/// and cannot be supplied to begin_in_transaction for a new execution.
pub struct HistoricalRequest {
    request: VerifiedRequest,
}
impl HistoricalRequest {
    pub fn request(&self) -> &VerifiedRequest {
        &self.request
    }
}
impl HistoryCursor<'_> {
    pub fn push(&mut self, link: &ChainLink) -> Result<VerifiedLink, Error> {
        if self.next.is_none() {
            if self
                .history
                .activations
                .first()
                .is_none_or(|a| a.sequence != link.sequence)
            {
                return Err(Error::MissingMaterial);
            }
            self.active = 0;
            self.next = Some(link.sequence);
            self.previous = Some(link.previous.clone());
        }
        if self.next != Some(link.sequence)
            || self.previous.as_deref() != Some(&link.previous)
            || sha256(&link.signed.bytes) != link.digest
        {
            return Err(Error::Chain);
        }
        if let Some(next) = self.history.activations.get(self.active + 1)
            && next.sequence == link.sequence
        {
            self.active += 1;
        }
        let activation = &self.history.activations[self.active];
        let verified = verify_link(
            &activation.trust,
            link,
            activation.sequence == link.sequence,
        )?;
        self.next = Some(link.sequence.checked_add(1).ok_or(Error::Chain)?);
        self.previous = Some(link.digest.clone());
        if let VerifiedLink::Request(request) = &verified
            && self.wanted_digest.as_deref() == Some(request.digest())
        {
            self.matched_request = Some(request.clone());
        }
        Ok(verified)
    }
    pub fn finish(&self, head_sequence: u64, head_digest: &str) -> Result<(), Error> {
        if self.next != head_sequence.checked_add(1)
            || self.previous.as_deref() != Some(head_digest)
            || self.active + 1 != self.history.activations.len()
        {
            return Err(Error::Chain);
        }
        Ok(())
    }
    pub fn finish_reconciliation(
        self,
        head_sequence: u64,
        head_digest: &str,
    ) -> Result<HistoricalRequest, Error> {
        self.finish(head_sequence, head_digest)?;
        Ok(HistoricalRequest {
            request: self.matched_request.ok_or(Error::MissingMaterial)?,
        })
    }
}
/// One link under the activation in effect at its sequence; `is_activation`
/// says whether that activation was recorded at exactly this sequence.
fn verify_link(
    trust: &VerifierTrust,
    link: &ChainLink,
    is_activation: bool,
) -> Result<VerifiedLink, Error> {
    if sha256(&link.signed.bytes) != link.digest {
        return Err(Error::Chain);
    }
    if link.roster != trust.roster_digest()
        || (link.kind == LinkKind::Activation) != is_activation
        || (is_activation && link.digest != trust.activation_digest())
    {
        return Err(Error::Binding);
    }
    if link.kind == LinkKind::Receipt {
        let value = trust.verify_receipt(link.signed.clone())?;
        if value.receipt.control_seq.0 != link.sequence
            || value.receipt.previous_link_sha256 != link.previous
            || value.fingerprint() != link.fingerprint
        {
            return Err(Error::Binding);
        }
        Ok(VerifiedLink::Receipt(value))
    } else {
        let value = trust.verify_request(link.signed.clone())?;
        if value.request.control_seq.0 != link.sequence
            || value.request.previous_link_sha256 != link.previous
            || value.fingerprint() != link.fingerprint
        {
            return Err(Error::Binding);
        }
        let activation_kind = matches!(value.request.action, Action::RosterActivate { .. });
        if activation_kind != (link.kind == LinkKind::Activation) {
            return Err(Error::Binding);
        }
        Ok(VerifiedLink::Request(value))
    }
}

/// The current head link was appended under the current activation: the head
/// row names that activation and the chain places it at or below the head.
/// A new link may extend the head only after the head itself verifies, so a
/// row inserted directly with runtime credentials can never become a
/// predecessor of operator-signed work. Earlier links were checked when they
/// were the head; `audit verify` still checks the whole retained chain.
pub fn verify_head_link(trust: &VerifierTrust, link: &ChainLink) -> Result<(), Error> {
    verify_link(trust, link, link.digest == trust.activation_digest()).map(|_| ())
}

impl History {
    /// The reviewed activation currently in effect (the last one).
    pub fn current(&self) -> &VerifierTrust {
        &self.activations[self.activations.len() - 1].trust
    }
    pub fn cursor(&self) -> HistoryCursor<'_> {
        HistoryCursor {
            history: self,
            active: 0,
            next: None,
            previous: None,
            wanted_digest: None,
            matched_request: None,
        }
    }
    pub fn cursor_for_request(&self, digest: String) -> Result<HistoryCursor<'_>, Error> {
        if !digest_text(&digest) {
            return Err(Error::Invalid);
        }
        let mut cursor = self.cursor();
        cursor.wanted_digest = Some(digest);
        Ok(cursor)
    }
    pub fn receipt_at(&self, signed: SignedBytes, sequence: u64) -> Result<VerifiedReceipt, Error> {
        let activation = self
            .activations
            .iter()
            .rev()
            .find(|a| a.sequence <= sequence)
            .ok_or(Error::MissingMaterial)?;
        let value = activation.trust.verify_receipt(signed)?;
        if value.receipt.control_seq.0 != sequence {
            return Err(Error::Binding);
        }
        Ok(value)
    }
    pub fn load(configs: Vec<ActivationConfig>, current: &VerifierTrust) -> Result<Self, Error> {
        let activations = configs
            .into_iter()
            .map(|c| {
                Ok(Activation {
                    sequence: c.control_seq.0,
                    trust: c.trust.load()?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Self::from_reviewed(activations, current)
    }
    pub fn from_reviewed(
        activations: Vec<Activation>,
        current: &VerifierTrust,
    ) -> Result<Self, Error> {
        if activations.is_empty()
            || activations.len() > 1024
            || activations
                .iter()
                .any(|a| a.sequence == 0 || a.trust.environment() != current.environment())
            || activations
                .windows(2)
                .any(|a| a[0].sequence >= a[1].sequence)
            || activations.last().is_none_or(|a| {
                a.trust.activation_digest() != current.activation_digest()
                    || a.trust.roster_digest() != current.roster_digest()
            })
        {
            return Err(Error::Binding);
        }
        Ok(Self { activations })
    }
    pub fn verify(
        &self,
        links: &[ChainLink],
        head_sequence: u64,
        head_digest: &str,
    ) -> Result<Vec<VerifiedLink>, Error> {
        let mut cursor = self.cursor();
        let values = links
            .iter()
            .map(|link| cursor.push(link))
            .collect::<Result<Vec<_>, _>>()?;
        cursor.finish(head_sequence, head_digest)?;
        Ok(values)
    }
}

#[derive(Debug, Clone)]
pub struct VerifiedRequest {
    request: Request,
    signed: SignedBytes,
    fingerprint: String,
    digest: String,
    activation: String,
}
impl VerifiedRequest {
    pub fn request(&self) -> &Request {
        &self.request
    }
    pub fn signed(&self) -> &SignedBytes {
        &self.signed
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn activation_digest(&self) -> &str {
        &self.activation
    }
}
#[derive(Debug, Clone)]
pub struct VerifiedReceipt {
    receipt: Receipt,
    signed: SignedBytes,
    fingerprint: String,
}
impl VerifiedReceipt {
    pub fn receipt(&self) -> &Receipt {
        &self.receipt
    }
    pub fn signed(&self) -> &SignedBytes {
        &self.signed
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

/// Constructed at startup from reviewed restart-only config, never a submitted
/// request or database row. Original roster path is not reused during verify.
pub struct VerifierTrust {
    environment: String,
    roster_sha256: String,
    activation_sha256: String,
    roster: Vec<u8>,
}
impl VerifierTrust {
    pub fn load(
        environment: String,
        roster_file: &Path,
        roster_sha256: String,
        activation_sha256: String,
    ) -> Result<Self, Error> {
        let bytes = crate::node::secure_file::read_checked(
            roster_file,
            crate::node::secure_file::FileClass::Trusted,
            crate::node::secure_file::effective_uid(),
            MAX_ROSTER_BYTES,
        )
        .map_err(|_| Error::Binding)?;
        Self::from_reviewed_bytes(environment, bytes, roster_sha256, activation_sha256)
    }
    fn from_reviewed_bytes(
        environment: String,
        roster: Vec<u8>,
        roster_sha256: String,
        activation_sha256: String,
    ) -> Result<Self, Error> {
        if !token(&environment)
            || !digest_text(&roster_sha256)
            || !digest_text(&activation_sha256)
            || roster.is_empty()
            || roster.len() > MAX_ROSTER_BYTES
            || sha256(&roster) != roster_sha256
        {
            return Err(Error::Binding);
        }
        Ok(Self {
            environment,
            roster_sha256,
            activation_sha256,
            roster,
        })
    }
    pub fn environment(&self) -> &str {
        &self.environment
    }
    pub fn roster_digest(&self) -> &str {
        &self.roster_sha256
    }
    pub fn activation_digest(&self) -> &str {
        &self.activation_sha256
    }

    pub fn verify_request(&self, signed: SignedBytes) -> Result<VerifiedRequest, Error> {
        let fingerprint = self.verify(&signed, REQUEST_NAMESPACE)?;
        let request: Request =
            jcs::decode(&signed.bytes, MAX_REQUEST_BYTES).map_err(|_| Error::Invalid)?;
        request.validate()?;
        if request.environment != self.environment || request.roster_sha256 != self.roster_sha256 {
            return Err(Error::Binding);
        }
        Ok(VerifiedRequest {
            digest: sha256(&signed.bytes),
            request,
            signed,
            fingerprint,
            activation: self.activation_sha256.clone(),
        })
    }
    pub fn verify_receipt(&self, signed: SignedBytes) -> Result<VerifiedReceipt, Error> {
        let fingerprint = self.verify(&signed, RECEIPT_NAMESPACE)?;
        let receipt: Receipt =
            jcs::decode(&signed.bytes, MAX_REQUEST_BYTES).map_err(|_| Error::Invalid)?;
        if receipt.version != 1
            || receipt.environment != self.environment
            || receipt.roster_sha256 != self.roster_sha256
            || !uuid(&receipt.operation_id)
            || !digest_text(&receipt.request_sha256)
            || !digest_text(&receipt.outcome_sha256)
            || !digest_text(&receipt.previous_link_sha256)
            || receipt.control_seq.0 == 0
        {
            return Err(Error::Binding);
        }
        Ok(VerifiedReceipt {
            receipt,
            signed,
            fingerprint,
        })
    }
    fn verify(&self, signed: &SignedBytes, namespace: &str) -> Result<String, Error> {
        if signed.bytes.is_empty()
            || signed.bytes.len() > MAX_REQUEST_BYTES
            || signed.signature.len() > MAX_SIGNATURE_BYTES
            || !token(&signed.principal)
        {
            return Err(Error::Bounds);
        }
        let scratch = Scratch::new()?;
        scratch.write("allowed", &self.roster)?;
        scratch.write("signature", signed.signature.as_bytes())?;
        let mut child = Command::new("/usr/bin/ssh-keygen")
            .args(["-Y", "verify", "-f"])
            .arg(scratch.path.join("allowed"))
            .args(["-I", &signed.principal, "-n", namespace, "-s"])
            .arg(scratch.path.join("signature"))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| Error::Signature)?;
        let write = child
            .stdin
            .take()
            .ok_or(Error::Signature)?
            .write_all(&signed.bytes);
        let status = child.wait().map_err(|_| Error::Signature)?;
        if write.is_err() || !status.success() {
            return Err(Error::Signature);
        }
        fingerprint(&signed.signature)
    }
}

pub fn synchronized_now_ms() -> Result<u64, Error> {
    let clock = oteryn_ntp_clock::read_synchronized().ok_or(Error::Clock)?;
    u64::try_from(clock.now_us / 1_000).map_err(|_| Error::Clock)
}
pub fn check_freshness(request: &Request, now: Result<u64, Error>) -> Result<(), Error> {
    let now = now?;
    if request.issued_at.0 > now.saturating_add(FUTURE_SKEW_MS)
        || now > request.issued_at.0.saturating_add(EXECUTION_WINDOW_MS)
    {
        return Err(Error::Stale);
    }
    Ok(())
}

/// Forensic binding shared by reads and mutations. An unsigned DB timestamp
/// cannot move execution outside the first-execution window of the signed intent.
pub fn check_execution_stamp(request: &Request, occurred_at: Decimal) -> Result<(), Error> {
    if occurred_at.0 < request.issued_at.0.saturating_sub(FUTURE_SKEW_MS)
        || occurred_at.0 > request.issued_at.0.saturating_add(EXECUTION_WINDOW_MS)
    {
        return Err(Error::Binding);
    }
    Ok(())
}
pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn digest_text(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn token(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 128
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._@:-".contains(&b))
}
pub fn uuid(text: &str) -> bool {
    crate::node::uuid_bytes(text).is_some_and(|b| b[6] >> 4 == 7 && b[8] & 0xc0 == 0x80)
}

fn fingerprint(pem: &str) -> Result<String, Error> {
    let encoded = pem
        .strip_prefix("-----BEGIN SSH SIGNATURE-----\n")
        .and_then(|s| s.strip_suffix("-----END SSH SIGNATURE-----\n"))
        .ok_or(Error::Signature)?;
    let binary = STANDARD
        .decode(encoded.lines().collect::<String>())
        .map_err(|_| Error::Signature)?;
    if binary.get(..10) != Some(b"SSHSIG\0\0\0\x01") {
        return Err(Error::Signature);
    }
    let size = u32::from_be_bytes(
        binary
            .get(10..14)
            .ok_or(Error::Signature)?
            .try_into()
            .map_err(|_| Error::Signature)?,
    ) as usize;
    let key = binary
        .get(14..14usize.checked_add(size).ok_or(Error::Bounds)?)
        .ok_or(Error::Signature)?;
    Ok(format!(
        "SHA256:{}",
        STANDARD_NO_PAD.encode(Sha256::digest(key))
    ))
}

pub(crate) struct Scratch {
    pub(crate) path: PathBuf,
}
impl Scratch {
    pub(crate) fn new() -> Result<Self, Error> {
        use std::os::unix::fs::DirBuilderExt;
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::Invalid)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "oteryn-staff-{}-{epoch}-{nonce}",
            std::process::id()
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .map_err(|_| Error::Signature)?;
        Ok(Self { path })
    }
    pub(crate) fn write(&self, name: &str, bytes: &[u8]) -> Result<(), Error> {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(self.path.join(name))
            .map_err(|_| Error::Signature)?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| Error::Signature)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if let Ok(files) = std::fs::read_dir(&self.path) {
            for file in files.flatten() {
                let _ = std::fs::remove_file(file.path());
            }
        }
        let _ = std::fs::remove_dir(&self.path);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    pub(crate) struct Signer {
        pub(crate) scratch: Scratch,
        pub(crate) roster: Vec<u8>,
    }
    impl Default for Signer {
        fn default() -> Self {
            Self::new()
        }
    }
    impl Signer {
        pub fn new() -> Self {
            let scratch = Scratch::new().expect("private fixture");
            assert!(
                Command::new("/usr/bin/ssh-keygen")
                    .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                    .arg(scratch.path.join("key"))
                    .status()
                    .expect("OpenSSH")
                    .success()
            );
            let public =
                std::fs::read_to_string(scratch.path.join("key.pub")).expect("public fixture key");
            let key = public
                .split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");
            let roster =
                format!("operator namespaces=\"{REQUEST_NAMESPACE},{RECEIPT_NAMESPACE}\" {key}\n")
                    .into_bytes();
            Self { scratch, roster }
        }
        pub fn trust(&self, environment: &str, activation: &str) -> VerifierTrust {
            VerifierTrust::from_reviewed_bytes(
                environment.into(),
                self.roster.clone(),
                sha256(&self.roster),
                activation.into(),
            )
            .expect("independent fixture trust")
        }
        pub fn sign(&self, bytes: Vec<u8>, namespace: &str) -> SignedBytes {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let name = format!(
                "message-{}",
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            );
            self.scratch.write(&name, &bytes).expect("fixture request");
            assert!(
                Command::new("/usr/bin/ssh-keygen")
                    .args(["-Y", "sign", "-f"])
                    .arg(self.scratch.path.join("key"))
                    .args(["-n", namespace])
                    .arg(self.scratch.path.join(&name))
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .expect("upstream sign")
                    .success()
            );
            SignedBytes {
                bytes,
                signature: std::fs::read_to_string(self.scratch.path.join(format!("{name}.sig")))
                    .expect("signature"),
                principal: "operator".into(),
            }
        }
        pub fn request(&self, operation: u8, sequence: u64, previous: &str) -> Request {
            Request {
                version: 1,
                operation_id: format!("00000000-0000-7000-8000-{operation:012x}"),
                environment: "alpha-test".into(),
                roster_sha256: sha256(&self.roster),
                issued_at: Decimal(1_000_000),
                control_seq: Decimal(sequence),
                previous_link_sha256: previous.into(),
                scope: Some(Scope {
                    world_id: "00000000-0000-7000-8000-000000000010".into(),
                    channel_id: "00000000-0000-7000-8000-000000000011".into(),
                }),
                reason: Reason::Support,
                action: Action::Inspect {
                    character_id: "00000000-0000-7000-8000-000000000001".into(),
                },
            }
        }
        pub fn activation(&self) -> (VerifierTrust, SignedBytes) {
            let mut request = self.request(1, 1, &"0".repeat(64));
            request.scope = None;
            request.reason = Reason::Roster;
            request.action = Action::RosterActivate {
                roster_sha256: sha256(&self.roster),
            };
            let signed = self.sign(
                request.canonical_bytes().expect("activation JCS"),
                REQUEST_NAMESPACE,
            );
            (self.trust("alpha-test", &sha256(&signed.bytes)), signed)
        }
    }
    #[test]
    fn ordinary_requests_require_signed_world_and_channel_scope() {
        let signer = Signer::new();
        let trust = signer.trust("alpha-test", &"a".repeat(64));
        let id = "00000000-0000-7000-8000-000000000001";
        let actions = [
            (
                "inspect",
                Action::Inspect {
                    character_id: id.into(),
                },
            ),
            (
                "kick character",
                Action::Kick {
                    target: Target::Character {
                        character_id: id.into(),
                    },
                },
            ),
            (
                "kick account",
                Action::Kick {
                    target: Target::Account {
                        account_ref: "c".repeat(64),
                    },
                },
            ),
            (
                "mute",
                Action::Mute {
                    character_id: id.into(),
                    duration_ms: Decimal(1),
                },
            ),
            (
                "unmute",
                Action::Unmute {
                    sanction_id: id.into(),
                },
            ),
            (
                "reports open",
                Action::ReportsList {
                    state: ReportState::Open,
                },
            ),
            (
                "reports closed",
                Action::ReportsList {
                    state: ReportState::Closed,
                },
            ),
            (
                "reports expired",
                Action::ReportsList {
                    state: ReportState::Expired,
                },
            ),
            (
                "report close",
                Action::ReportClose {
                    report_id: id.into(),
                },
            ),
            (
                "report clear text",
                Action::ReportClearText {
                    report_id: id.into(),
                },
            ),
        ];
        let mut unexpected = Vec::new();
        for (name, action) in actions {
            let mut request = signer.request(2, 2, &"b".repeat(64));
            request.action = action;
            let valid = trust
                .verify_request(signer.sign(
                    request.canonical_bytes().expect("scoped request JCS"),
                    REQUEST_NAMESPACE,
                ))
                .expect("signed scoped request");
            assert_eq!(valid.request().scope, request.scope, "{name}");

            for operator in [
                "missing scope",
                "invalid world",
                "invalid channel",
                "equal world/channel",
            ] {
                let mut invalid = request.clone();
                match operator {
                    "missing scope" => invalid.scope = None,
                    "invalid world" => {
                        invalid.scope.as_mut().expect("scope").world_id = "invalid".into()
                    }
                    "equal world/channel" => {
                        let scope = invalid.scope.as_mut().expect("scope");
                        scope.channel_id = scope.world_id.clone();
                    }
                    _ => invalid.scope.as_mut().expect("scope").channel_id = "invalid".into(),
                }
                if !matches!(invalid.canonical_bytes(), Err(Error::Invalid)) {
                    unexpected.push(format!("{name}: {operator}: prepare"));
                }
                // Sign the adversarial JCS directly so preparation cannot hide
                // a verifier failure behind an invalid or absent signature.
                let signed = signer.sign(
                    jcs::encode(&invalid, MAX_REQUEST_BYTES).expect("adversarial JCS"),
                    REQUEST_NAMESPACE,
                );
                if !matches!(trust.verify_request(signed), Err(Error::Invalid)) {
                    unexpected.push(format!("{name}: {operator}: verify"));
                }
            }
        }
        // The accepted first link activates the environment's independently
        // pinned roster; it is the existing typed exception to world scope.
        let (activation_trust, activation) = signer.activation();
        let activation = activation_trust
            .verify_request(activation)
            .expect("environment roster activation");
        assert!(activation.request().scope.is_none());
        assert!(matches!(
            activation.request().action,
            Action::RosterActivate { .. }
        ));
        // A signature naming one world/channel never authorizes the
        // environment-wide roster rotation, at preparation or verification.
        let mut scoped = activation.request().clone();
        scoped.scope = Some(Scope {
            world_id: "00000000-0000-7000-8000-000000000010".into(),
            channel_id: "00000000-0000-7000-8000-000000000011".into(),
        });
        if !matches!(scoped.canonical_bytes(), Err(Error::Invalid)) {
            unexpected.push("scoped activation: prepare".into());
        }
        let signed = signer.sign(
            jcs::encode(&scoped, MAX_REQUEST_BYTES).expect("adversarial JCS"),
            REQUEST_NAMESPACE,
        );
        if !matches!(activation_trust.verify_request(signed), Err(Error::Invalid)) {
            unexpected.push("scoped activation: verify".into());
        }
        assert!(
            unexpected.is_empty(),
            "scope validation failures: {unexpected:?}"
        );
    }
    #[test]
    fn missing_signature_unknown_principal_and_wrong_namespace_are_independent() {
        let signer = Signer::new();
        let trust = signer.trust("alpha-test", &"a".repeat(64));
        let bytes = signer
            .request(2, 2, &"b".repeat(64))
            .canonical_bytes()
            .expect("canonical");
        let valid = signer.sign(bytes.clone(), REQUEST_NAMESPACE);
        assert!(trust.verify_request(valid.clone()).is_ok());
        let mut missing = valid.clone();
        missing.signature.clear();
        assert!(matches!(
            trust.verify_request(missing),
            Err(Error::Signature)
        ));
        let mut unknown = valid;
        unknown.principal = "other".into();
        assert!(matches!(
            trust.verify_request(unknown),
            Err(Error::Signature)
        ));
        assert!(matches!(
            trust.verify_request(signer.sign(bytes, RECEIPT_NAMESPACE)),
            Err(Error::Signature)
        ));
    }
    #[test]
    fn environment_roster_and_parameter_binding_have_no_unsigned_sibling() {
        let signer = Signer::new();
        let activation = "a".repeat(64);
        let trust = signer.trust("alpha-test", &activation);
        let request = signer.request(2, 2, &"b".repeat(64));
        let signed = signer.sign(request.canonical_bytes().expect("JCS"), REQUEST_NAMESPACE);
        assert!(matches!(
            signer
                .trust("other-environment", &activation)
                .verify_request(signed.clone()),
            Err(Error::Binding)
        ));
        let mut roster = signer.roster.clone();
        roster.push(b'\n');
        let altered = VerifierTrust::from_reviewed_bytes(
            "alpha-test".into(),
            roster.clone(),
            sha256(&roster),
            activation,
        )
        .expect("independent roster revision");
        assert!(matches!(
            altered.verify_request(signed.clone()),
            Err(Error::Binding)
        ));
        let mut tampered = signed;
        tampered.bytes = String::from_utf8(tampered.bytes)
            .expect("UTF8")
            .replace("000000000001", "000000000003")
            .into_bytes();
        assert!(matches!(
            trust.verify_request(tampered),
            Err(Error::Signature)
        ));
    }
    #[test]
    fn authenticated_unknown_duplicate_missing_and_noncanonical_fields_refuse() {
        let signer = Signer::new();
        let trust = signer.trust("alpha-test", &"a".repeat(64));
        let canonical = String::from_utf8(
            signer
                .request(2, 2, &"b".repeat(64))
                .canonical_bytes()
                .expect("JCS"),
        )
        .expect("UTF8");
        for bytes in [
            canonical.replacen('{', "{\"unsigned\":0,", 1),
            canonical.replacen('{', "{\"version\":1,", 1),
            canonical.replacen("\"version\":1", "\"version\":2", 1),
            format!(" {canonical}"),
        ] {
            assert!(
                trust
                    .verify_request(signer.sign(bytes.into_bytes(), REQUEST_NAMESPACE))
                    .is_err()
            );
        }
        let missing = canonical
            .replace("\"version\":1,", "")
            .replace(",\"version\":1", "");
        assert!(
            trust
                .verify_request(signer.sign(missing.into_bytes(), REQUEST_NAMESPACE))
                .is_err()
        );
    }
    #[test]
    fn first_execution_clock_bounds_and_unsynchronized_clock_fail_closed() {
        let signer = Signer::new();
        let request = signer.request(2, 2, &"b".repeat(64));
        assert!(check_freshness(&request, Ok(1_600_000)).is_ok());
        assert_eq!(check_freshness(&request, Ok(1_600_001)), Err(Error::Stale));
        assert!(check_freshness(&request, Ok(700_000)).is_ok());
        assert_eq!(check_freshness(&request, Ok(699_999)), Err(Error::Stale));
        assert_eq!(
            check_freshness(&request, Err(Error::Clock)),
            Err(Error::Clock)
        );
    }
    #[test]
    fn chain_gaps_forks_digest_substitution_and_unrecorded_activation_fail() {
        let signer = Signer::new();
        let (trust, activation) = signer.activation();
        let history = History::from_reviewed(
            vec![Activation {
                sequence: 1,
                trust: signer.trust("alpha-test", trust.activation_digest()),
            }],
            &trust,
        )
        .expect("offhost history");
        let checked = trust
            .verify_request(activation.clone())
            .expect("initial activation");
        let initial = ChainLink {
            sequence: 1,
            previous: "0".repeat(64),
            digest: sha256(&activation.bytes),
            roster: trust.roster_digest().into(),
            fingerprint: checked.fingerprint().into(),
            signed: activation,
            kind: LinkKind::Activation,
        };
        let request = signer.request(2, 2, &initial.digest);
        let signed = signer.sign(request.canonical_bytes().expect("JCS"), REQUEST_NAMESPACE);
        let checked = trust.verify_request(signed.clone()).expect("request");
        let second = ChainLink {
            sequence: 2,
            previous: initial.digest.clone(),
            digest: sha256(&signed.bytes),
            roster: trust.roster_digest().into(),
            fingerprint: checked.fingerprint().into(),
            signed,
            kind: LinkKind::Request,
        };
        assert!(
            history
                .verify(&[initial.clone(), second.clone()], 2, &second.digest)
                .is_ok()
        );
        assert!(
            history
                .verify(std::slice::from_ref(&second), 2, &second.digest)
                .is_err()
        );
        assert!(
            history
                .verify(
                    &[initial.clone(), second.clone(), second.clone()],
                    2,
                    &second.digest
                )
                .is_err()
        );
        let mut wrong_kind = second.clone();
        wrong_kind.kind = LinkKind::Activation;
        assert!(
            history
                .verify(&[initial.clone(), wrong_kind], 2, &second.digest)
                .is_err()
        );
        let mut changed = second.clone();
        changed.previous = "f".repeat(64);
        assert!(
            history
                .verify(&[initial.clone(), changed], 2, &second.digest)
                .is_err()
        );
        let mut activation_request = signer.request(3, 2, &initial.digest);
        activation_request.scope = None;
        activation_request.reason = Reason::Roster;
        activation_request.action = Action::RosterActivate {
            roster_sha256: trust.roster_digest().into(),
        };
        let signed = signer.sign(
            activation_request.canonical_bytes().expect("JCS"),
            REQUEST_NAMESPACE,
        );
        let checked = trust
            .verify_request(signed.clone())
            .expect("valid signature");
        let unrecorded = ChainLink {
            kind: LinkKind::Activation,
            signed: signed.clone(),
            digest: sha256(&signed.bytes),
            fingerprint: checked.fingerprint().into(),
            ..second
        };
        assert!(
            history
                .verify(&[initial, unrecorded.clone()], 2, &unrecorded.digest)
                .is_err()
        );
    }
    #[test]
    fn receipts_cannot_replace_recorded_activation_anchors() {
        let old = Signer::new();
        let new = Signer::new();
        let (old_trust, first) = old.activation();
        let checked = old_trust
            .verify_request(first.clone())
            .expect("first activation");
        let initial = ChainLink {
            sequence: 1,
            previous: "0".repeat(64),
            digest: sha256(&first.bytes),
            roster: old_trust.roster_digest().into(),
            fingerprint: checked.fingerprint().into(),
            signed: first,
            kind: LinkKind::Activation,
        };
        let mut next = new.request(3, 2, &initial.digest);
        next.scope = None;
        next.reason = Reason::Roster;
        next.action = Action::RosterActivate {
            roster_sha256: sha256(&new.roster),
        };
        let activated = new.sign(
            next.canonical_bytes().expect("next activation"),
            REQUEST_NAMESPACE,
        );
        let current = new.trust("alpha-test", &sha256(&activated.bytes));
        let checked = current
            .verify_request(activated.clone())
            .expect("next activation");
        let activation = ChainLink {
            sequence: 2,
            previous: initial.digest.clone(),
            digest: sha256(&activated.bytes),
            roster: current.roster_digest().into(),
            fingerprint: checked.fingerprint().into(),
            signed: activated,
            kind: LinkKind::Activation,
        };
        let mut accepted_sequences = Vec::new();
        for (signer, trust, previous, sequence, prefix) in [
            (&old, &old_trust, "0".repeat(64), 1, vec![]),
            (
                &new,
                &current,
                initial.digest.clone(),
                2,
                vec![initial.clone()],
            ),
        ] {
            let mut activations = Vec::new();
            if sequence == 2 {
                activations.push(Activation {
                    sequence: 1,
                    trust: old.trust("alpha-test", old_trust.activation_digest()),
                });
            }
            activations.push(Activation {
                sequence,
                trust: signer.trust("alpha-test", trust.activation_digest()),
            });
            let history = History::from_reviewed(activations, trust).expect("reviewed history");
            let genuine = if sequence == 1 {
                initial.clone()
            } else {
                activation.clone()
            };
            let mut valid = prefix.clone();
            valid.push(genuine.clone());
            assert!(history.verify(&valid, sequence, &genuine.digest).is_ok());
            let make_receipt = |receipt_sequence, previous: String| {
                let receipt = Receipt {
                    version: 1,
                    environment: trust.environment().into(),
                    roster_sha256: trust.roster_digest().into(),
                    operation_id: "00000000-0000-7000-8000-000000000004".into(),
                    request_sha256: "a".repeat(64),
                    outcome_sha256: "b".repeat(64),
                    disposition: Disposition::Committed,
                    issued_at: Decimal(1_000_000),
                    control_seq: Decimal(receipt_sequence),
                    previous_link_sha256: previous.clone(),
                };
                let signed = signer.sign(
                    jcs::encode(&receipt, MAX_REQUEST_BYTES).expect("receipt JCS"),
                    RECEIPT_NAMESPACE,
                );
                let checked = trust
                    .verify_receipt(signed.clone())
                    .expect("valid receipt signature");
                ChainLink {
                    sequence: receipt_sequence,
                    previous,
                    digest: sha256(&signed.bytes),
                    roster: trust.roster_digest().into(),
                    fingerprint: checked.fingerprint().into(),
                    signed,
                    kind: LinkKind::Receipt,
                }
            };
            let ordinary = make_receipt(sequence + 1, genuine.digest.clone());
            let mut with_receipt = valid;
            with_receipt.push(ordinary.clone());
            assert!(
                history
                    .verify(&with_receipt, sequence + 1, &ordinary.digest)
                    .is_ok()
            );
            let substituted = make_receipt(sequence, previous);
            let mut chain = prefix;
            chain.push(substituted.clone());
            if history
                .verify(&chain, sequence, &substituted.digest)
                .is_ok()
            {
                accepted_sequences.push(sequence);
            }
        }
        assert!(
            accepted_sequences.is_empty(),
            "receipts replaced independently recorded activations at sequences {accepted_sequences:?}"
        );
    }
    #[test]
    fn removed_key_cannot_append_after_independently_recorded_activation() {
        let old = Signer::new();
        let new = Signer::new();
        let (old_trust, first) = old.activation();
        let mut next = new.request(3, 2, &sha256(&first.bytes));
        next.scope = None;
        next.reason = Reason::Roster;
        next.action = Action::RosterActivate {
            roster_sha256: sha256(&new.roster),
        };
        let activated = new.sign(
            next.canonical_bytes().expect("new activation"),
            REQUEST_NAMESPACE,
        );
        let current = new.trust("alpha-test", &sha256(&activated.bytes));
        let history = History::from_reviewed(
            vec![
                Activation {
                    sequence: 1,
                    trust: old_trust,
                },
                Activation {
                    sequence: 2,
                    trust: new.trust("alpha-test", current.activation_digest()),
                },
            ],
            &current,
        )
        .expect("reviewed offhost revisions");
        let make_link = |signed: SignedBytes, trust: &VerifierTrust| {
            let request = trust
                .verify_request(signed.clone())
                .expect("signature against its named revision");
            ChainLink {
                sequence: request.request().control_seq.0,
                previous: request.request().previous_link_sha256.clone(),
                digest: sha256(&signed.bytes),
                roster: request.request().roster_sha256.clone(),
                fingerprint: request.fingerprint().into(),
                signed,
                kind: if matches!(request.request().action, Action::RosterActivate { .. }) {
                    LinkKind::Activation
                } else {
                    LinkKind::Request
                },
            }
        };
        let initial = make_link(first, &old.trust("alpha-test", &"a".repeat(64)));
        let activation = make_link(activated, &current);
        assert!(
            history
                .verify(
                    &[initial.clone(), activation.clone()],
                    2,
                    &activation.digest
                )
                .is_ok()
        );
        // Entire earlier-prefix deletion must not be excused by a later
        // recorded activation that happens to remain in the database.
        assert!(
            history
                .verify(std::slice::from_ref(&activation), 2, &activation.digest)
                .is_err()
        );
        let removed = old.sign(
            old.request(4, 3, &activation.digest)
                .canonical_bytes()
                .expect("old-key request"),
            REQUEST_NAMESPACE,
        );
        let forged = make_link(removed, &old.trust("alpha-test", &"a".repeat(64)));
        assert!(
            history
                .verify(&[initial, activation, forged.clone()], 3, &forged.digest)
                .is_err()
        );
    }
    #[test]
    fn receipt_signature_namespace_and_principal_remain_distinct_from_intent() {
        let signer = Signer::new();
        let trust = signer.trust("alpha-test", &"a".repeat(64));
        let value = Receipt {
            version: 1,
            environment: "alpha-test".into(),
            roster_sha256: trust.roster_digest().into(),
            operation_id: signer.request(2, 2, &"b".repeat(64)).operation_id,
            request_sha256: "c".repeat(64),
            outcome_sha256: "d".repeat(64),
            disposition: Disposition::Committed,
            issued_at: Decimal(1_000_000),
            control_seq: Decimal(3),
            previous_link_sha256: "e".repeat(64),
        };
        let bytes = jcs::encode(&value, MAX_REQUEST_BYTES).expect("receipt JCS");
        assert!(
            trust
                .verify_receipt(signer.sign(bytes.clone(), RECEIPT_NAMESPACE))
                .is_ok()
        );
        assert!(
            trust
                .verify_receipt(signer.sign(bytes.clone(), REQUEST_NAMESPACE))
                .is_err()
        );
        let mut signed = signer.sign(bytes, RECEIPT_NAMESPACE);
        signed.principal = "other".into();
        assert!(trust.verify_receipt(signed).is_err());
    }
}
