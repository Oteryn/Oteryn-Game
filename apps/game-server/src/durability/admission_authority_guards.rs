//! Lossless historical guard encoding. Decoding never registers an owning source.
use super::DurabilityError;
use oteryn_game_server::foundation::admission_authority_publication::*;
use oteryn_game_server::foundation::fnd04_verifier::*;
use oteryn_game_server::foundation::*;

type Result<T> = std::result::Result<T, DurabilityError>;
fn invalid<T>() -> Result<T> {
    Err(DurabilityError::InvalidStoredState)
}
pub(super) fn checked<T, E>(result: std::result::Result<T, E>) -> Result<T> {
    result.map_err(|_| DurabilityError::InvalidStoredState)
}

pub(super) struct Writer {
    pub bytes: Vec<u8>,
    maximum: usize,
}
impl Writer {
    pub fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
        }
    }
    pub fn bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(DurabilityError::InvalidStoredState)?;
        if length > self.maximum {
            return invalid();
        }
        checked(self.bytes.try_reserve_exact(bytes.len()))?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    pub fn tag(&mut self, value: u8) -> Result<()> {
        self.bytes(&[value])
    }
    pub fn boolean(&mut self, value: bool) -> Result<()> {
        self.tag(u8::from(value))
    }
    pub fn u64(&mut self, value: u64) -> Result<()> {
        self.bytes(&value.to_be_bytes())
    }
    pub fn i64(&mut self, value: i64) -> Result<()> {
        self.bytes(&value.to_be_bytes())
    }
    pub fn text(&mut self, value: &str) -> Result<()> {
        self.u64(checked(u64::try_from(value.len()))?)?;
        self.bytes(value.as_bytes())
    }
}
pub(super) struct Reader<'a> {
    remaining: &'a [u8],
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }
    pub fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let (value, rest) = self
            .remaining
            .split_at_checked(length)
            .ok_or(DurabilityError::InvalidStoredState)?;
        self.remaining = rest;
        Ok(value)
    }
    pub fn bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        checked(self.take(N)?.try_into())
    }
    pub fn tag(&mut self) -> Result<u8> {
        Ok(self.bytes::<1>()?[0])
    }
    pub fn boolean(&mut self) -> Result<bool> {
        match self.tag()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => invalid(),
        }
    }
    pub fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.bytes()?))
    }
    pub fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.bytes()?))
    }
    pub fn text(&mut self) -> Result<String> {
        let length = checked(usize::try_from(self.u64()?))?;
        // Check the complete declared field against retained input before allocating.
        Ok(checked(std::str::from_utf8(self.take(length)?))?.to_owned())
    }
    pub fn finish(self) -> Result<()> {
        if self.remaining.is_empty() {
            Ok(())
        } else {
            invalid()
        }
    }
}

pub(super) fn write_provenance(w: &mut Writer, p: &FreshEvidenceProvenanceV1) -> Result<()> {
    w.text(&p.source_authority)?;
    w.tag(match p.purpose {
        FreshEvidencePurposeV1::PlatformSecurity => 1,
        FreshEvidencePurposeV1::SigningTrust => 2,
    })?;
    w.tag(match p.scope {
        Fnd04EvidenceScope::FreshAdmission => 1,
        Fnd04EvidenceScope::ExistingActorRecovery => 2,
    })?;
    w.u64(p.source_revision)?;
    w.u64(p.accepted_source_revision)?;
    w.text(&p.decision_identity)?;
    w.text(&p.accepted_decision_identity)?;
    w.i64(p.source_observed_at)?;
    w.u64(p.clock_uncertainty_seconds)?;
    w.u64(p.publication_revision)
}
pub(super) fn read_provenance(r: &mut Reader<'_>) -> Result<FreshEvidenceProvenanceV1> {
    Ok(FreshEvidenceProvenanceV1 {
        source_authority: r.text()?,
        purpose: match r.tag()? {
            1 => FreshEvidencePurposeV1::PlatformSecurity,
            2 => FreshEvidencePurposeV1::SigningTrust,
            _ => return invalid(),
        },
        scope: match r.tag()? {
            1 => Fnd04EvidenceScope::FreshAdmission,
            2 => Fnd04EvidenceScope::ExistingActorRecovery,
            _ => return invalid(),
        },
        source_revision: r.u64()?,
        accepted_source_revision: r.u64()?,
        decision_identity: r.text()?,
        accepted_decision_identity: r.text()?,
        source_observed_at: r.i64()?,
        clock_uncertainty_seconds: r.u64()?,
        publication_revision: r.u64()?,
    })
}
pub(super) fn write_security(w: &mut Writer, s: &FreshAccountSecurityObservationV1) -> Result<()> {
    w.text(&s.account_id)?;
    w.u64(s.minimum_generation)?;
    w.boolean(s.allowed)?;
    write_provenance(w, &s.provenance)
}
pub(super) fn read_security(r: &mut Reader<'_>) -> Result<FreshAccountSecurityObservationV1> {
    Ok(FreshAccountSecurityObservationV1 {
        account_id: r.text()?,
        minimum_generation: r.u64()?,
        allowed: r.boolean()?,
        provenance: read_provenance(r)?,
    })
}
pub(super) fn write_scope(w: &mut Writer, scope: RuntimeScopeRefV1) -> Result<()> {
    match scope {
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } => {
            w.tag(1)?;
            w.bytes(world_id.as_bytes())?;
            w.bytes(channel_id.as_bytes())
        }
        RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } => {
            w.tag(2)?;
            w.bytes(world_id.as_bytes())?;
            w.bytes(&instance_id)
        }
    }
}
pub(super) fn read_scope(r: &mut Reader<'_>) -> Result<RuntimeScopeRefV1> {
    let tag = r.tag()?;
    let world = checked(WorldId::decode(&r.bytes::<16>()?))?;
    match tag {
        1 => Ok(RuntimeScopeRefV1::channel(
            world,
            checked(ChannelId::decode(&r.bytes::<16>()?))?,
        )),
        2 => checked(RuntimeScopeRefV1::instance(world, r.bytes()?)),
        _ => invalid(),
    }
}
fn write_key(w: &mut Writer, key: &AdmissionAuthorityGuardKeyV1) -> Result<()> {
    match key {
        AdmissionAuthorityGuardKeyV1::Account { account_id } => {
            w.tag(1)?;
            w.text(account_id)
        }
        AdmissionAuthorityGuardKeyV1::Character(id) => {
            w.tag(2)?;
            w.bytes(id.as_bytes())
        }
        AdmissionAuthorityGuardKeyV1::Runtime(scope) => {
            w.tag(3)?;
            write_scope(w, *scope)
        }
        AdmissionAuthorityGuardKeyV1::SigningTrust { key_id, profile } => {
            w.tag(4)?;
            w.text(key_id)?;
            w.text(profile)
        }
    }
}
fn read_key(r: &mut Reader<'_>) -> Result<AdmissionAuthorityGuardKeyV1> {
    Ok(match r.tag()? {
        1 => AdmissionAuthorityGuardKeyV1::Account {
            account_id: r.text()?,
        },
        2 => AdmissionAuthorityGuardKeyV1::Character(checked(CharacterId::decode(
            &r.bytes::<16>()?,
        ))?),
        3 => AdmissionAuthorityGuardKeyV1::Runtime(read_scope(r)?),
        4 => AdmissionAuthorityGuardKeyV1::SigningTrust {
            key_id: r.text()?,
            profile: r.text()?,
        },
        _ => return invalid(),
    })
}
fn write_state(w: &mut Writer, state: &AdmissionAuthorityGuardStateV1) -> Result<()> {
    match state {
        AdmissionAuthorityGuardStateV1::Account { security, presence } => {
            w.tag(1)?;
            write_security(w, security)?;
            w.boolean(presence.is_some())?;
            if let Some((character, session)) = presence {
                w.bytes(character.as_bytes())?;
                w.bytes(session.as_bytes())?;
            }
        }
        AdmissionAuthorityGuardStateV1::Character {
            account_id,
            world_id,
            eligible,
            lease_generation,
            holder,
        } => {
            w.tag(2)?;
            w.text(account_id)?;
            w.bytes(world_id.as_bytes())?;
            w.boolean(*eligible)?;
            w.u64(*lease_generation)?;
            w.boolean(holder.is_some())?;
            if let Some(holder) = holder {
                w.bytes(holder.as_bytes())?;
            }
        }
        AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation,
            ready,
            route_revision,
            runtime_observation_revision,
            protocol_major,
            transport_profile,
            ruleset_revision,
            content_revision,
            map_revision,
            world_policy_revision,
            offer_revision,
        } => {
            w.tag(3)?;
            w.u64(*ownership_generation)?;
            w.boolean(*ready)?;
            w.text(route_revision)?;
            w.text(runtime_observation_revision)?;
            w.u64(*protocol_major)?;
            w.u64(*transport_profile)?;
            for value in [
                ruleset_revision,
                content_revision,
                map_revision,
                world_policy_revision,
                offer_revision,
            ] {
                w.text(value)?;
            }
        }
        AdmissionAuthorityGuardStateV1::SigningTrust {
            public_key,
            trusted,
        } => {
            w.tag(4)?;
            w.bytes(public_key)?;
            w.boolean(*trusted)?;
        }
    }
    Ok(())
}
fn read_state(r: &mut Reader<'_>) -> Result<AdmissionAuthorityGuardStateV1> {
    Ok(match r.tag()? {
        1 => {
            let security = read_security(r)?;
            let presence = if r.boolean()? {
                Some((
                    checked(CharacterId::decode(&r.bytes::<16>()?))?,
                    checked(GameSessionId::decode(&r.bytes::<16>()?))?,
                ))
            } else {
                None
            };
            AdmissionAuthorityGuardStateV1::Account { security, presence }
        }
        2 => {
            let account_id = r.text()?;
            let world_id = checked(WorldId::decode(&r.bytes::<16>()?))?;
            let eligible = r.boolean()?;
            let lease_generation = r.u64()?;
            let holder = if r.boolean()? {
                Some(checked(GameSessionId::decode(&r.bytes::<16>()?))?)
            } else {
                None
            };
            AdmissionAuthorityGuardStateV1::Character {
                account_id,
                world_id,
                eligible,
                lease_generation,
                holder,
            }
        }
        3 => AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation: r.u64()?,
            ready: r.boolean()?,
            route_revision: r.text()?,
            runtime_observation_revision: r.text()?,
            protocol_major: r.u64()?,
            transport_profile: r.u64()?,
            ruleset_revision: r.text()?,
            content_revision: r.text()?,
            map_revision: r.text()?,
            world_policy_revision: r.text()?,
            offer_revision: r.text()?,
        },
        4 => AdmissionAuthorityGuardStateV1::SigningTrust {
            public_key: r.bytes()?,
            trusted: r.boolean()?,
        },
        _ => return invalid(),
    })
}
pub(super) fn write_change(
    w: &mut Writer,
    c: &AdmissionAuthorityPublicationChangeV1,
) -> Result<()> {
    write_key(w, &c.key)?;
    w.text(&c.source.authority)?;
    w.tag(match c.source.purpose {
        AdmissionPublicationPurposeV1::AccountSecurityAndPresence => 1,
        AdmissionPublicationPurposeV1::CharacterOwnershipAndLease => 2,
        AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness => 3,
        AdmissionPublicationPurposeV1::FixedFreshSigningTrust => 4,
    })?;
    w.u64(c.source.source_revision)?;
    w.text(&c.source.decision_identity)?;
    w.i64(c.source.source_observed_at)?;
    w.u64(c.source.clock_uncertainty_seconds)?;
    match c.precondition {
        AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water,
        } => {
            w.tag(1)?;
            w.boolean(restored_publication_high_water.is_some())?;
            if let Some(value) = restored_publication_high_water {
                w.u64(value)?;
            }
        }
        AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision,
        } => {
            w.tag(2)?;
            w.u64(expected_publication_revision)?;
        }
    }
    w.u64(c.publication_revision)?;
    write_state(w, &c.state)
}
pub(super) fn read_change(r: &mut Reader<'_>) -> Result<AdmissionAuthorityPublicationChangeV1> {
    let key = read_key(r)?;
    let authority = r.text()?;
    let purpose = match r.tag()? {
        1 => AdmissionPublicationPurposeV1::AccountSecurityAndPresence,
        2 => AdmissionPublicationPurposeV1::CharacterOwnershipAndLease,
        3 => AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
        4 => AdmissionPublicationPurposeV1::FixedFreshSigningTrust,
        _ => return invalid(),
    };
    let source = AdmissionPublicationSourceV1 {
        authority,
        purpose,
        source_revision: r.u64()?,
        decision_identity: r.text()?,
        source_observed_at: r.i64()?,
        clock_uncertainty_seconds: r.u64()?,
    };
    let precondition = match r.tag()? {
        1 => AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water: if r.boolean()? { Some(r.u64()?) } else { None },
        },
        2 => AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: r.u64()?,
        },
        _ => return invalid(),
    };
    Ok(AdmissionAuthorityPublicationChangeV1 {
        key,
        source,
        precondition,
        publication_revision: r.u64()?,
        state: read_state(r)?,
    })
}
pub(super) fn write_changes(
    w: &mut Writer,
    changes: &[AdmissionAuthorityPublicationChangeV1],
) -> Result<()> {
    if changes.len() > 4 {
        return invalid();
    }
    w.tag(checked(u8::try_from(changes.len()))?)?;
    for change in changes {
        write_change(w, change)?;
    }
    Ok(())
}
pub(super) fn read_changes(
    r: &mut Reader<'_>,
) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>> {
    let count = r.tag()?;
    if count > 4 {
        return invalid();
    }
    (0..count).map(|_| read_change(r)).collect()
}

/// Encode one historical guard change; this does not prepare a publication.
pub fn encode_guard(
    change: &AdmissionAuthorityPublicationChangeV1,
    maximum_bytes: usize,
) -> Result<String> {
    let mut writer = Writer::new(maximum_bytes);
    write_change(&mut writer, change)?;
    super::fresh_admission::encode_envelope(&writer.bytes, maximum_bytes)
}
/// Decode a historical guard; consumers must still compare every SQL mirror and
/// invoke their sealed Foundation predicate against independently current rows.
pub fn decode_guard(
    encoded: &str,
    maximum_bytes: usize,
) -> Result<AdmissionAuthorityPublicationChangeV1> {
    let bytes = super::fresh_admission::decode_envelope(encoded, maximum_bytes)?;
    let mut reader = Reader::new(&bytes);
    let change = read_change(&mut reader)?;
    reader.finish()?;
    Ok(change)
}

/// Storage classification only; this is not an activated owner projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardPublicationDisposition {
    Applied,
    Existing,
    Stale,
    Conflict,
}

/// Explicit caller allocation pending production executor/resource integration.
#[derive(Clone)]
pub struct AdmissionGuardStore {
    pool: sqlx::PgPool,
    maximum_guard_bytes: usize,
}

type Mirror = Vec<(&'static str, &'static str, Option<String>)>;

fn uuid_text(bytes: &[u8; 16]) -> String {
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}
fn bytea_text(bytes: &[u8]) -> String {
    let mut text = String::from("\\x");
    for byte in bytes {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 15)]));
    }
    text
}
fn key_storage(
    key: &AdmissionAuthorityGuardKeyV1,
    maximum: usize,
) -> Result<(&'static str, Mirror, Vec<u8>)> {
    let mut encoded = Writer::new(maximum);
    write_key(&mut encoded, key)?;
    let (table, fields) = match key {
        AdmissionAuthorityGuardKeyV1::Account { account_id } => (
            "game_durability_admission_account_guards",
            vec![("account_id", "uuid", Some(account_id.clone()))],
        ),
        AdmissionAuthorityGuardKeyV1::Character(id) => (
            "game_durability_admission_character_guards",
            vec![("character_id", "uuid", Some(uuid_text(id.as_bytes())))],
        ),
        AdmissionAuthorityGuardKeyV1::Runtime(scope) => {
            let mut bytes = Writer::new(maximum);
            write_scope(&mut bytes, *scope)?;
            (
                "game_durability_admission_runtime_guards",
                vec![("scope_key", "bytea", Some(bytea_text(&bytes.bytes)))],
            )
        }
        AdmissionAuthorityGuardKeyV1::SigningTrust { key_id, profile } => (
            "game_durability_admission_signing_trust_guards",
            vec![
                ("key_id", "text", Some(key_id.clone())),
                ("profile", "text", Some(profile.clone())),
            ],
        ),
    };
    Ok((table, fields, encoded.bytes))
}
fn guard_mirrors(change: &AdmissionAuthorityPublicationChangeV1, maximum: usize) -> Result<Mirror> {
    let (_, mut fields, _) = key_storage(&change.key, maximum)?;
    match &change.state {
        AdmissionAuthorityGuardStateV1::Account { presence, .. } => fields.extend([
            (
                "presence_character_id",
                "uuid",
                presence.map(|(id, _)| uuid_text(id.as_bytes())),
            ),
            (
                "holder_game_session_id",
                "uuid",
                presence.map(|(_, id)| uuid_text(id.as_bytes())),
            ),
        ]),
        AdmissionAuthorityGuardStateV1::Character {
            account_id,
            world_id,
            eligible,
            lease_generation,
            holder,
        } => fields.extend([
            ("account_id", "uuid", Some(account_id.clone())),
            ("world_id", "uuid", Some(uuid_text(world_id.as_bytes()))),
            ("eligible", "boolean", Some(eligible.to_string())),
            (
                "lease_generation",
                "numeric(20,0)",
                Some(lease_generation.to_string()),
            ),
            (
                "holder_game_session_id",
                "uuid",
                holder.map(|id| uuid_text(id.as_bytes())),
            ),
        ]),
        AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation,
            ready,
            ..
        } => fields.extend([
            (
                "ownership_generation",
                "numeric(20,0)",
                Some(ownership_generation.to_string()),
            ),
            ("ready", "boolean", Some(ready.to_string())),
        ]),
        AdmissionAuthorityGuardStateV1::SigningTrust {
            public_key,
            trusted,
        } => fields.extend([
            ("public_key", "bytea", Some(bytea_text(public_key))),
            ("trusted", "boolean", Some(trusted.to_string())),
        ]),
    }
    fields.extend([
        (
            "publication_revision",
            "numeric(20,0)",
            Some(change.publication_revision.to_string()),
        ),
        (
            "source_authority",
            "text",
            Some(change.source.authority.clone()),
        ),
        (
            "source_revision",
            "numeric(20,0)",
            Some(change.source.source_revision.to_string()),
        ),
        (
            "decision_identity",
            "text",
            Some(change.source.decision_identity.clone()),
        ),
        (
            "source_observed_at",
            "bigint",
            Some(change.source.source_observed_at.to_string()),
        ),
        (
            "clock_uncertainty_seconds",
            "numeric(20,0)",
            Some(change.source.clock_uncertainty_seconds.to_string()),
        ),
    ]);
    Ok(fields)
}
fn key_predicate(query: &mut sqlx::QueryBuilder<sqlx::Postgres>, fields: Mirror) {
    for (index, (column, kind, value)) in fields.into_iter().enumerate() {
        if index != 0 {
            query.push(" AND ");
        }
        query
            .push(column)
            .push(" = ")
            .push_bind(value)
            .push("::text::")
            .push(kind);
    }
}

impl AdmissionGuardStore {
    pub async fn connect_runtime(database_url: &str, maximum_guard_bytes: usize) -> Result<Self> {
        if maximum_guard_bytes == 0 {
            return invalid();
        }
        let pool = super::schema::connect_runtime(database_url).await?;
        Ok(Self {
            pool,
            maximum_guard_bytes,
        })
    }

    pub async fn load(
        &self,
        keys: &[AdmissionAuthorityGuardKeyV1],
    ) -> Result<Vec<Option<AdmissionAuthorityPublicationChangeV1>>> {
        if keys.len() > 4 {
            return invalid();
        }
        let mut transaction = self.pool.begin().await?;
        super::db::lock_admission_relations(&mut transaction).await?;
        let mut rows = Vec::with_capacity(keys.len());
        for key in keys {
            rows.push(self.load_locked(&mut transaction, key).await?);
        }
        transaction.commit().await?;
        Ok(rows)
    }

    async fn load_locked(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        key: &AdmissionAuthorityGuardKeyV1,
    ) -> Result<Option<AdmissionAuthorityPublicationChangeV1>> {
        use sqlx::Row;
        let (table, fields, encoded_key) = key_storage(key, self.maximum_guard_bytes)?;
        // Guard payload length before transfer in the same protected snapshot.
        // NULL marks oversized/corrupt storage, never authoritative absence.
        let mut query = sqlx::QueryBuilder::new("SELECT CASE WHEN octet_length(change_json) <= ");
        query.push_bind(checked(i64::try_from(self.maximum_guard_bytes))?).push(" THEN change_json END AS payload, CASE WHEN octet_length((to_jsonb(g) - 'change_json')::text) <= ")
            .push_bind(checked(i64::try_from(self.maximum_guard_bytes))?)
            .push(" THEN (SELECT jsonb_object_agg(key, value) FROM jsonb_each_text(to_jsonb(g) - 'change_json')) END AS mirrors FROM ").push(table).push(" g WHERE ");
        key_predicate(&mut query, fields);
        let row = query.build().fetch_optional(&mut **transaction).await?;
        let Some(row) = row else {
            let history: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_admission_guard_history WHERE guard_key = $1)").bind(encoded_key).fetch_one(&mut **transaction).await?;
            if history {
                return invalid();
            }
            return Ok(None);
        };
        let payload: Option<String> = row.try_get("payload")?;
        let payload = payload.ok_or(DurabilityError::InvalidStoredState)?;
        let change = decode_guard(&payload, self.maximum_guard_bytes)?;
        if &change.key != key {
            return invalid();
        }
        let expected: serde_json::Map<String, serde_json::Value> =
            guard_mirrors(&change, self.maximum_guard_bytes)?
                .into_iter()
                .map(|(name, _, value)| {
                    (
                        name.to_owned(),
                        value.map_or(serde_json::Value::Null, serde_json::Value::String),
                    )
                })
                .collect();
        let mirrors: serde_json::Value = row.try_get("mirrors")?;
        if mirrors != serde_json::Value::Object(expected) {
            return invalid();
        }
        let history_matches: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_admission_guard_history WHERE guard_key = $1 AND publication_revision = $2::text::numeric(20,0) AND source_authority = $3 AND source_revision = $4::text::numeric(20,0) AND decision_identity = $5 AND change_json = $6) AND NOT EXISTS (SELECT 1 FROM game_durability_admission_guard_history WHERE guard_key = $1 AND publication_revision > $2::text::numeric(20,0))")
            .bind(encoded_key).bind(change.publication_revision.to_string()).bind(&change.source.authority).bind(change.source.source_revision.to_string()).bind(&change.source.decision_identity).bind(&payload).fetch_one(&mut **transaction).await?;
        if !history_matches {
            return invalid();
        }
        Ok(Some(change))
    }

    pub async fn publish(
        &self,
        request: &AdmissionAuthorityPublicationV1,
    ) -> Result<GuardPublicationDisposition> {
        if request.changes().len() > 4 {
            return invalid();
        }
        // Encode/validate explicit per-record allocation before transaction work.
        let encoded: Vec<_> = request
            .changes()
            .iter()
            .map(|row| encode_guard(row, self.maximum_guard_bytes))
            .collect::<Result<_>>()?;
        let mut transaction = self.pool.begin().await?;
        super::db::lock_admission_relations(&mut transaction).await?;
        let mut current = Vec::with_capacity(request.changes().len());
        for change in request.changes() {
            current.push(self.load_locked(&mut transaction, &change.key).await?);
        }
        if let Err(error) = request.validate_locked(&current) {
            return Ok(if error == AdmissionAuthorityPublicationErrorV1::Stale {
                GuardPublicationDisposition::Stale
            } else {
                GuardPublicationDisposition::Conflict
            });
        }
        if current
            .iter()
            .zip(request.changes())
            .all(|(old, new)| old.as_ref() == Some(new))
        {
            transaction.commit().await?;
            return Ok(GuardPublicationDisposition::Existing);
        }
        // Classify the entire batch before effects. Permanent decision history
        // prevents an owner decision or source revision being reused later.
        for (change, old) in request.changes().iter().zip(&current) {
            if old.as_ref() == Some(change) {
                continue;
            }
            let (_, _, key) = key_storage(&change.key, self.maximum_guard_bytes)?;
            let conflict: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_admission_guard_history WHERE guard_key = $1 AND (publication_revision = $2::text::numeric(20,0) OR (source_authority = $3 AND (source_revision = $4::text::numeric(20,0) OR decision_identity = $5))))")
                .bind(key).bind(change.publication_revision.to_string()).bind(&change.source.authority).bind(change.source.source_revision.to_string()).bind(&change.source.decision_identity).fetch_one(&mut *transaction).await?;
            if conflict {
                return Ok(GuardPublicationDisposition::Conflict);
            }
        }
        for ((change, old), payload) in request.changes().iter().zip(&current).zip(&encoded) {
            if old.as_ref() == Some(change) {
                continue;
            }
            let (table, keys, key) = key_storage(&change.key, self.maximum_guard_bytes)?;
            let mut fields = guard_mirrors(change, self.maximum_guard_bytes)?;
            fields.push(("change_json", "text", Some(payload.clone())));
            let mut query = sqlx::QueryBuilder::new("INSERT INTO ");
            query.push(table).push(" (");
            for (index, (column, _, _)) in fields.iter().enumerate() {
                if index != 0 {
                    query.push(", ");
                }
                query.push(*column);
            }
            query.push(") VALUES (");
            for (index, (_, kind, value)) in fields.iter().enumerate() {
                if index != 0 {
                    query.push(", ");
                }
                query.push_bind(value.clone()).push("::text::").push(*kind);
            }
            query.push(") ON CONFLICT (");
            for (index, (column, _, _)) in keys.iter().enumerate() {
                if index != 0 {
                    query.push(", ");
                }
                query.push(*column);
            }
            query.push(") DO UPDATE SET ");
            for (index, (column, _, _)) in fields.iter().enumerate() {
                if index != 0 {
                    query.push(", ");
                }
                query.push(*column).push(" = EXCLUDED.").push(*column);
            }
            query.build().execute(&mut *transaction).await?;
            sqlx::query("INSERT INTO game_durability_admission_guard_history (guard_key, publication_revision, source_authority, source_revision, decision_identity, change_json) VALUES ($1, $2::text::numeric(20,0), $3, $4::text::numeric(20,0), $5, $6)")
                .bind(key).bind(change.publication_revision.to_string()).bind(&change.source.authority).bind(change.source.source_revision.to_string()).bind(&change.source.decision_identity).bind(payload).execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        Ok(GuardPublicationDisposition::Applied)
    }
}
