//! Disabled candidate orchestration, not an activated authentication path.
//! Activation requires reviewed OS-exclusive lock/journal and HTTPS transport bindings.
//! A durable block precedes removal/send. Lost replies, crashes and uncertain persistence
//! require browser authentication; the predecessor is never retried, even after an outage.

use crate::device_store::{DeviceSecret, DeviceStore};
use oteryn_platform_client::native_login::{NativeCharacterChoice, parse_uuid_v7};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::BTreeSet,
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Configuration,
    AuthenticationRequired,
    Vault,
    Journal,
    TransportUncertain,
    InvalidResponse,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Configuration => "invalid remembered-login configuration",
            Self::AuthenticationRequired => "browser authentication required",
            Self::Vault => "remembered-login vault unavailable",
            Self::Journal => "remembered-login journal unavailable",
            Self::TransportUncertain => "remembered-login request outcome uncertain",
            Self::InvalidResponse => "invalid remembered-login response",
        })
    }
}
impl std::error::Error for Error {}

#[derive(Clone, PartialEq, Eq)]
pub struct Namespace {
    origin: String,
    client_id: String,
}
impl Namespace {
    pub fn new(
        origin: &str,
        client_id: &str,
        explicit_development_origin: Option<&str>,
    ) -> Result<Self, Error> {
        if !uuid(client_id) {
            return Err(Error::Configuration);
        }
        let url = url::Url::parse(origin).map_err(|_| Error::Configuration)?;
        let canonical = url.origin().ascii_serialization();
        let development = url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && explicit_development_origin == Some(canonical.as_str());
        DeviceStore::new(origin, client_id, development).map_err(|_| Error::Configuration)?;
        Ok(Self {
            origin: canonical,
            client_id: client_id.to_owned(),
        })
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
}

/// Must be non-cloneable and exclusively owned across processes AND threads until drop.
pub trait Guard {
    fn namespace(&self) -> &Namespace;
    fn recovery_required(&self) -> bool;
}
pub trait NamespaceLock {
    type Guard: Guard;
    fn acquire(&mut self, namespace: &Namespace) -> Result<Self::Guard, Error>;
}
/// Atomic durable metadata, containing no secret. Errors from unblock MUST leave blocked state.
/// Reuse is refused while blocked. Concrete binding/recovery qualification is an activation gate.
pub trait Journal {
    fn namespace(&self) -> &Namespace;
    fn blocked(&mut self) -> Result<bool, Error>;
    fn block(&mut self) -> Result<(), Error>;
    fn unblock(&mut self) -> Result<(), Error>;
}
pub trait Vault {
    fn namespace(&self) -> &Namespace;
    fn load(&mut self) -> Result<Option<Credential>, Error>;
    fn delete(&mut self) -> Result<(), Error>;
    fn save(&mut self, credential: &Credential) -> Result<(), Error>;
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credential {
    protocol_version: u8,
    #[serde(deserialize_with = "secret")]
    device_credential: DeviceSecret,
    family_id: String,
    absolute_expires_at: u64,
    idle_expires_at: u64,
}
impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Credential([REDACTED])")
    }
}
impl Credential {
    pub fn expose(&self) -> &str {
        self.device_credential.expose()
    }
    fn valid_identity(&self) -> bool {
        let token = self.expose();
        self.protocol_version == 1
            && uuid(&self.family_id)
            && token.len() == 48
            && token.starts_with("otd1.")
            && token[5..]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    }
    fn valid(&self, now: u64) -> bool {
        self.valid_identity()
            && self.idle_expires_at > now
            && self.idle_expires_at <= self.absolute_expires_at
            && self.absolute_expires_at.saturating_sub(now) <= 2_592_000
            && self.idle_expires_at.saturating_sub(now) <= 604_800
    }
}
fn secret<'de, D: Deserializer<'de>>(d: D) -> Result<DeviceSecret, D::Error> {
    DeviceSecret::new(String::deserialize(d)?)
        .map_err(|_| serde::de::Error::custom("invalid opaque credential"))
}
fn uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                *b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(b)
            }
        })
        && matches!(bytes[14], b'1'..=b'8')
        && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
}

/// OS-vault-only compact envelope binds metadata to its secret. No plaintext fallback.
pub struct OsVault {
    namespace: Namespace,
    store: DeviceStore,
}
impl OsVault {
    pub fn new(namespace: Namespace) -> Result<Self, Error> {
        let store = DeviceStore::new(
            &namespace.origin,
            &namespace.client_id,
            namespace.origin.starts_with("http://"),
        )
        .map_err(|_| Error::Configuration)?;
        Ok(Self { namespace, store })
    }
}
impl Vault for OsVault {
    fn namespace(&self) -> &Namespace {
        &self.namespace
    }
    fn load(&mut self) -> Result<Option<Credential>, Error> {
        self.store
            .load()
            .map_err(|_| Error::Vault)?
            .map(|encoded| serde_json::from_str(encoded.expose()).map_err(|_| Error::Vault))
            .transpose()
    }
    fn delete(&mut self) -> Result<(), Error> {
        self.store.delete().map_err(|_| Error::Vault)
    }
    fn save(&mut self, credential: &Credential) -> Result<(), Error> {
        self.store
            .save(&encode_record(credential)?)
            .map_err(|_| Error::Vault)
    }
}
fn encode_record(credential: &Credential) -> Result<DeviceSecret, Error> {
    #[derive(Serialize)]
    struct Envelope<'a> {
        protocol_version: u8,
        device_credential: &'a str,
        family_id: &'a str,
        absolute_expires_at: u64,
        idle_expires_at: u64,
    }
    let encoded = serde_json::to_string(&Envelope {
        protocol_version: 1,
        device_credential: credential.expose(),
        family_id: &credential.family_id,
        absolute_expires_at: credential.absolute_expires_at,
        idle_expires_at: credential.idle_expires_at,
    })
    .map_err(|_| Error::Vault)?;
    DeviceSecret::new(encoded).map_err(|_| Error::Vault)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    Characters,
    Ticket,
}
pub enum Request<'a> {
    Enroll,
    Rotate(Purpose, &'a Credential),
    Revoke(&'a Credential),
}
impl Request<'_> {
    pub fn path(&self) -> &'static str {
        match self {
            Self::Enroll => "/api/v1/game-auth/device-sessions",
            Self::Rotate(Purpose::Characters, _) => {
                "/api/v1/game-auth/device-sessions/native-characters"
            }
            Self::Rotate(Purpose::Ticket, _) => "/api/v1/game-auth/device-sessions/tickets",
            Self::Revoke(_) => "/api/v1/game-auth/device-sessions/revoke",
        }
    }
    pub fn public_body(&self, client_id: &str) -> serde_json::Value {
        match self {
            Self::Enroll => serde_json::json!({"protocol_version": 1, "remember_device": true}),
            _ => serde_json::json!({"protocol_version": 1, "client_id": client_id}),
        }
    }
    /// Enrollment instead uses the transport's authenticated OAuth Bearer context.
    pub fn device_authorization(&self) -> Option<&str> {
        match self {
            Self::Enroll => None,
            Self::Rotate(_, record) | Self::Revoke(record) => Some(record.expose()),
        }
    }
}
pub struct SensitiveResponse(Zeroizing<Vec<u8>>);
impl SensitiveResponse {
    pub fn new(bytes: Vec<u8>) -> Result<Self, Error> {
        let bytes = Zeroizing::new(bytes);
        if bytes.len() > 16_384 {
            return Err(Error::InvalidResponse);
        }
        Ok(Self(bytes))
    }
}
/// One HTTPS POST to the fixed origin, no redirects/retries, with a finite network deadline.
/// Bounded, uncached, nonlogged response; remote URLs never become authority.
/// Device requests use Authorization: OterynDevice <secret>; enrollment uses OAuth Bearer.
pub trait Transport {
    /// Concrete binding MUST be the non-cloneable AccessToken of a fresh PKCE authorization.
    /// The consuming call cannot retain or retry this enrollment authority after uncertainty.
    type Enrollment;
    fn namespace(&self) -> &Namespace;
    fn enroll(&mut self, authorization: Self::Enrollment) -> Result<SensitiveResponse, Error>;
    fn send(&mut self, request: Request<'_>) -> Result<SensitiveResponse, Error>;
}

#[derive(Default)]
enum Field<T> {
    #[default]
    Missing,
    Present(T),
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Field<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Self::Present)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    protocol_version: u8,
    #[serde(deserialize_with = "secret")]
    device_credential: DeviceSecret,
    family_id: String,
    absolute_expires_at: u64,
    idle_expires_at: u64,
    #[serde(default)]
    characters: Field<Vec<NativeCharacterChoice>>,
    #[serde(default, deserialize_with = "ticket_field")]
    ticket: Field<DeviceSecret>,
    #[serde(default)]
    expires_in: Field<u64>,
    #[serde(default)]
    expires_at: Field<u64>,
}
fn ticket_field<'de, D: Deserializer<'de>>(d: D) -> Result<Field<DeviceSecret>, D::Error> {
    secret(d).map(Field::Present)
}
pub enum GameResult {
    Characters(Vec<NativeCharacterChoice>),
    Ticket {
        ticket: DeviceSecret,
        expires_in: u64,
        expires_at: u64,
    },
}
impl fmt::Debug for GameResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GameResult([REDACTED])")
    }
}
fn decode(
    body: SensitiveResponse,
    purpose: Option<Purpose>,
    now: u64,
) -> Result<(Credential, Option<GameResult>), Error> {
    let response: Response = serde_json::from_slice(&body.0).map_err(|_| Error::InvalidResponse)?;
    let credential = Credential {
        protocol_version: response.protocol_version,
        device_credential: response.device_credential,
        family_id: response.family_id,
        absolute_expires_at: response.absolute_expires_at,
        idle_expires_at: response.idle_expires_at,
    };
    if !credential.valid(now) {
        return Err(Error::InvalidResponse);
    }
    let result = match (
        purpose,
        response.characters,
        response.ticket,
        response.expires_in,
        response.expires_at,
    ) {
        (None, Field::Missing, Field::Missing, Field::Missing, Field::Missing) => None,
        (
            Some(Purpose::Characters),
            Field::Present(characters),
            Field::Missing,
            Field::Missing,
            Field::Missing,
        ) => {
            let mut ids = BTreeSet::new();
            if characters.len() > 128
                || characters.iter().any(|c| {
                    parse_uuid_v7(&c.character_id).is_none()
                        || parse_uuid_v7(&c.world_id).is_none()
                        || !ids.insert(&c.character_id)
                        || c.name.is_empty()
                        || c.name.len() > 96
                        || c.name.chars().any(char::is_control)
                        || !matches!(c.availability.as_str(), "AVAILABLE" | "UNAVAILABLE")
                })
            {
                return Err(Error::InvalidResponse);
            }
            Some(GameResult::Characters(characters))
        }
        (
            Some(Purpose::Ticket),
            Field::Missing,
            Field::Present(ticket),
            Field::Present(expires_in),
            Field::Present(expires_at),
        ) => {
            if ticket.expose().len() > 256
                || !(1..=60).contains(&expires_in)
                || expires_at <= now
                // Server relative lifetime may be floored from a subsecond instant while the
                // absolute Unix-second deadline rounds one second later. The usable budget
                // remains expires_in (capped at 60) in the eventual monotonic ticket adapter.
                || expires_at.saturating_sub(now) > expires_in.saturating_add(1)
            {
                return Err(Error::InvalidResponse);
            }
            Some(GameResult::Ticket {
                ticket,
                expires_in,
                expires_at,
            })
        }
        _ => return Err(Error::InvalidResponse),
    };
    Ok((credential, result))
}

pub struct Coordinator<V, J, L, T> {
    namespace: Namespace,
    vault: V,
    journal: J,
    lock: L,
    transport: T,
}
impl<V: Vault, J: Journal, L: NamespaceLock, T: Transport> Coordinator<V, J, L, T> {
    pub fn new(
        namespace: Namespace,
        vault: V,
        journal: J,
        lock: L,
        transport: T,
    ) -> Result<Self, Error> {
        if vault.namespace() != &namespace
            || journal.namespace() != &namespace
            || transport.namespace() != &namespace
        {
            return Err(Error::Configuration);
        }
        Ok(Self {
            namespace,
            vault,
            journal,
            lock,
            transport,
        })
    }
    fn acquire(&mut self) -> Result<L::Guard, Error> {
        let guard = self.lock.acquire(&self.namespace)?;
        if guard.namespace() != &self.namespace {
            return Err(Error::Configuration);
        }
        if guard.recovery_required() {
            self.journal.block()?;
            return Err(Error::AuthenticationRequired);
        }
        Ok(guard)
    }
    pub fn rotate(
        &mut self,
        purpose: Purpose,
        now: impl Fn() -> Result<u64, Error>,
    ) -> Result<GameResult, Error> {
        let _guard = self.acquire()?;
        if self.journal.blocked()? {
            return Err(Error::AuthenticationRequired);
        }
        let old = self.vault.load()?.ok_or(Error::AuthenticationRequired)?;
        self.journal.block()?;
        self.vault.delete()?;
        if !old.valid(now()?) {
            return Err(Error::AuthenticationRequired);
        }
        let body = self.transport.send(Request::Rotate(purpose, &old))?;
        let (next, result) = decode(body, Some(purpose), now()?)?;
        if next.family_id != old.family_id
            || next.absolute_expires_at != old.absolute_expires_at
            || next.expose() == old.expose()
        {
            return Err(Error::InvalidResponse);
        }
        self.vault.save(&next)?;
        self.journal.unblock()?;
        result.ok_or(Error::InvalidResponse)
    }
    pub fn enroll(
        &mut self,
        explicit_consent: bool,
        authorization: T::Enrollment,
        now: impl Fn() -> Result<u64, Error>,
    ) -> Result<(), Error> {
        if !explicit_consent {
            return Err(Error::AuthenticationRequired);
        }
        let _guard = self.acquire()?;
        if self.vault.load()?.is_some() {
            return Err(Error::AuthenticationRequired);
        }
        self.journal.block()?;
        let (record, _) = decode(self.transport.enroll(authorization)?, None, now()?)?;
        self.vault.save(&record)?;
        self.journal.unblock()
    }
    /// Local removal is confirmed before optional remote revocation. The durable block remains.
    /// False means remote revocation was not confirmed; no automatic retry is performed.
    pub fn forget(&mut self) -> Result<bool, Error> {
        let _guard = self.acquire()?;
        self.journal.block()?;
        let record = self.vault.load();
        self.vault.delete()?;
        let Ok(Some(record)) = record else {
            return Ok(false);
        };
        if !record.valid_identity() {
            return Ok(false);
        }
        let Ok(body) = self.transport.send(Request::Revoke(&record)) else {
            return Ok(false);
        };
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Revoked {
            protocol_version: u8,
            revoked: bool,
        }
        Ok(serde_json::from_slice::<Revoked>(&body.0)
            .is_ok_and(|r| r.protocol_version == 1 && r.revoked))
    }
}
pub fn utc_seconds() -> Result<u64, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .map_err(|_| Error::Configuration)
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    const CLIENT: &str = "11111111-1111-4111-8111-111111111111";
    const FAMILY: &str = "22222222-2222-4222-8222-222222222222";
    struct State { blocked: bool, held: bool, events: Vec<&'static str>, fault: &'static str,
        stored: Option<DeviceSecret>, reply: Option<Result<SensitiveResponse, Error>> }
    #[derive(Clone)]
    struct Fixture { namespace: Namespace, state: Rc<RefCell<State>> }
    struct Lease(Fixture);
    impl Guard for Lease {
        fn namespace(&self) -> &Namespace { &self.0.namespace }
        fn recovery_required(&self) -> bool { false }
    }
    impl Drop for Lease {
        fn drop(&mut self) { let mut state = self.0.state.borrow_mut(); state.held = false; state.events.push("unlock"); }
    }
    impl Fixture {
        fn event(&self, event: &'static str) -> Result<(), Error> {
            let mut state = self.state.borrow_mut();
            if !state.held { return Err(Error::Configuration); }
            state.events.push(event);
            Ok(())
        }
    }
    impl NamespaceLock for Fixture {
        type Guard = Lease;
        fn acquire(&mut self, _: &Namespace) -> Result<Lease, Error> {
            let mut state = self.state.borrow_mut();
            if state.held { return Err(Error::AuthenticationRequired); }
            state.held = true; state.events.push("lock");
            Ok(Lease(self.clone()))
        }
    }
    impl Journal for Fixture {
        fn namespace(&self) -> &Namespace { &self.namespace }
        fn blocked(&mut self) -> Result<bool, Error> { self.event("status")?; Ok(self.state.borrow().blocked) }
        fn block(&mut self) -> Result<(), Error> {
            self.event("block")?; let mut s = self.state.borrow_mut();
            if s.fault == "block" { return Err(Error::Journal); } s.blocked = true; Ok(())
        }
        fn unblock(&mut self) -> Result<(), Error> {
            self.event("unblock")?; let mut s = self.state.borrow_mut();
            if s.fault == "unblock" { return Err(Error::Journal); } s.blocked = false; Ok(())
        }
    }
    impl Vault for Fixture {
        fn namespace(&self) -> &Namespace { &self.namespace }
        fn load(&mut self) -> Result<Option<Credential>, Error> {
            self.event("load")?; self.state.borrow().stored.as_ref().map(|s|
                serde_json::from_str(s.expose()).map_err(|_| Error::Vault)).transpose()
        }
        fn delete(&mut self) -> Result<(), Error> {
            self.event("delete")?; let mut s = self.state.borrow_mut();
            if s.fault == "delete" { return Err(Error::Vault); } s.stored = None; Ok(())
        }
        fn save(&mut self, record: &Credential) -> Result<(), Error> {
            self.event("save")?; let mut s = self.state.borrow_mut(); s.stored = Some(encode_record(record)?);
            if s.fault == "save" { return Err(Error::Vault); } Ok(())
        }
    }
    /// Non-cloneable test stand-in; production must use the consumed PKCE AccessToken.
    struct FreshAuthorization;
    impl Transport for Fixture {
        type Enrollment = FreshAuthorization;
        fn namespace(&self) -> &Namespace { &self.namespace }
        fn enroll(&mut self, _: FreshAuthorization) -> Result<SensitiveResponse, Error> { self.send(Request::Enroll) }
        fn send(&mut self, _: Request<'_>) -> Result<SensitiveResponse, Error> {
            self.event("send")?; self.state.borrow_mut().reply.take().ok_or(Error::TransportUncertain)?
        }
    }
    fn record(letter: char, family: &str, absolute: u64) -> Result<Credential, Error> {
        let encoded = DeviceSecret::new(format!("{{\"protocol_version\":1,\"device_credential\":\"otd1.{}\",\"family_id\":\"{}\",\"absolute_expires_at\":{},\"idle_expires_at\":1500}}",
            letter.to_string().repeat(43), family, absolute)).map_err(|_| Error::Vault)?;
        serde_json::from_str(encoded.expose()).map_err(|_| Error::Vault)
    }
    fn response(record: &Credential, extra: &str) -> Result<SensitiveResponse, Error> {
        let encoded = encode_record(record)?;
        let prefix = encoded.expose().strip_suffix('}').ok_or(Error::InvalidResponse)?;
        SensitiveResponse::new(format!("{prefix}{extra}}}").into_bytes())
    }
    fn fixture(fault: &'static str, reply: Result<SensitiveResponse, Error>) -> Result<Fixture, Error> {
        Ok(Fixture { namespace: Namespace::new("https://oteryn.com", CLIENT, None)?, state: Rc::new(RefCell::new(State {
            blocked: false, held: false, events: Vec::new(), fault, stored: Some(encode_record(&record('a', FAMILY, 2000)?)?), reply: Some(reply),
        })) })
    }
    fn coordinator(fixture: &Fixture) -> Result<Coordinator<Fixture, Fixture, Fixture, Fixture>, Error> {
        Coordinator::new(fixture.namespace.clone(), fixture.clone(), fixture.clone(), fixture.clone(), fixture.clone())
    }
    #[test]
    fn rotation_holds_guard_and_commits_before_returning_result() -> Result<(), Error> {
        let fixture = fixture("", response(&record('b', FAMILY, 2000)?, ",\"characters\":[]"))?;
        let result = coordinator(&fixture)?.rotate(Purpose::Characters, || Ok(1000))?;
        assert!(matches!(result, GameResult::Characters(ref values) if values.is_empty()));
        assert_eq!(fixture.state.borrow().events, ["lock", "status", "load", "block", "delete", "send", "save", "unblock", "unlock"]);
        assert!(!fixture.state.borrow().blocked);
        Ok(())
    }
    #[test]
    fn block_and_delete_failures_never_send_a_request() -> Result<(), Error> {
        for fault in ["block", "delete"] {
            let fixture = fixture(fault, Err(Error::TransportUncertain))?;
            assert!(coordinator(&fixture)?.rotate(Purpose::Characters, || Ok(1000)).is_err());
            assert!(!fixture.state.borrow().events.contains(&"send"));
        }
        Ok(())
    }
    #[test]
    fn lost_reply_and_ambiguous_save_or_unblock_remain_blocked() -> Result<(), Error> {
        for fault in ["transport", "save", "unblock"] {
            let reply = if fault == "transport" { Err(Error::TransportUncertain) }
                else { response(&record('b', FAMILY, 2000)?, ",\"characters\":[]") };
            let fixture = fixture(fault, reply)?;
            let mut coordinator = coordinator(&fixture)?;
            assert!(coordinator.rotate(Purpose::Characters, || Ok(1000)).is_err());
            assert!(fixture.state.borrow().blocked);
            assert!(matches!(coordinator.rotate(Purpose::Characters, || Ok(1000)), Err(Error::AuthenticationRequired)));
            assert_eq!(fixture.state.borrow().events.iter().filter(|e| **e == "send").count(), 1);
        }
        Ok(())
    }
    #[test]
    fn mismatched_family_extended_deadline_and_unchanged_secret_are_not_saved() -> Result<(), Error> {
        for next in [record('b', CLIENT, 2000)?, record('b', FAMILY, 2001)?, record('a', FAMILY, 2000)?] {
            let fixture = fixture("", response(&next, ",\"characters\":[]"))?;
            assert!(matches!(coordinator(&fixture)?.rotate(Purpose::Characters, || Ok(1000)), Err(Error::InvalidResponse)));
            assert!(fixture.state.borrow().blocked && fixture.state.borrow().stored.is_none());
        }
        Ok(())
    }
    #[test]
    fn namespace_mismatch_and_held_namespace_lock_refuse_operation() -> Result<(), Error> {
        let fixture = fixture("", Err(Error::TransportUncertain))?;
        let mut other = fixture.clone(); other.namespace = Namespace::new("https://other.oteryn.com", CLIENT, None)?;
        assert!(matches!(Coordinator::new(fixture.namespace.clone(), other, fixture.clone(), fixture.clone(), fixture.clone()), Err(Error::Configuration)));
        let mut lock = fixture.clone(); let lease = lock.acquire(&fixture.namespace)?;
        assert!(coordinator(&fixture)?.rotate(Purpose::Characters, || Ok(1000)).is_err());
        assert!(!fixture.state.borrow().events.contains(&"send")); drop(lease);
        Ok(())
    }
    #[test]
    fn strict_result_schema_and_expiry_do_not_publish_credentials() -> Result<(), Error> {
        for extra in [",\"characters\":null", ",\"characters\":[],\"account_id\":\"forbidden\"",
            ",\"ticket\":\"unused-test-ticket\",\"expires_in\":61,\"expires_at\":1061"] {
            let body = response(&record('b', FAMILY, 2000)?, extra)?;
            assert!(matches!(decode(body, Some(Purpose::Characters), 1000), Err(Error::InvalidResponse)));
        }
        assert_eq!(format!("{:?}", record('a', FAMILY, 2000)?), "Credential([REDACTED])");
        let body = response(&record('b', FAMILY, 2000)?,
            ",\"ticket\":\"unused-test-ticket\",\"expires_in\":59,\"expires_at\":1060")?;
        assert!(matches!(decode(body, Some(Purpose::Ticket), 1000)?,
            (_, Some(GameResult::Ticket { expires_in: 59, .. }))));
        for (lifetime, deadline) in [(61, 1061), (59, 1061), (59, 1000)] {
            let body = response(&record('b', FAMILY, 2000)?, &format!(
                ",\"ticket\":\"unused-test-ticket\",\"expires_in\":{lifetime},\"expires_at\":{deadline}"))?;
            assert!(matches!(decode(body, Some(Purpose::Ticket), 1000), Err(Error::InvalidResponse)));
        }
        Ok(())
    }
    #[test]
    fn forgotten_state_remains_blocked_and_enrollment_requires_explicit_fresh_authority() -> Result<(), Error> {
        let fixture = fixture("", Err(Error::TransportUncertain))?;
        let mut coordinator = coordinator(&fixture)?;
        assert!(!coordinator.forget()?);
        assert!(fixture.state.borrow().blocked && fixture.state.borrow().stored.is_none());
        assert!(matches!(coordinator.enroll(false, FreshAuthorization, || Ok(1000)), Err(Error::AuthenticationRequired)));
        assert_eq!(fixture.state.borrow().events.iter().filter(|e| **e == "send").count(), 1);
        Ok(())
    }
    #[test]
    fn enrollment_does_not_overwrite_and_lost_enrollment_consumes_its_authority() -> Result<(), Error> {
        let fixture = fixture("", Err(Error::TransportUncertain))?;
        let mut coordinator = coordinator(&fixture)?;
        assert!(matches!(coordinator.enroll(true, FreshAuthorization, || Ok(1000)), Err(Error::AuthenticationRequired)));
        assert!(!fixture.state.borrow().events.contains(&"send"));
        fixture.state.borrow_mut().stored = None;
        let fresh_authority = FreshAuthorization;
        assert!(matches!(coordinator.enroll(true, fresh_authority, || Ok(1000)), Err(Error::TransportUncertain)));
        // fresh_authority has moved and cannot be retried; a new PKCE capability is required.
        assert!(fixture.state.borrow().blocked && fixture.state.borrow().stored.is_none());
        assert_eq!(fixture.state.borrow().events.iter().filter(|e| **e == "send").count(), 1);
        Ok(())
    }
}
