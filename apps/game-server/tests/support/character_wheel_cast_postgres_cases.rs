//! The cast's real SQL Wheel reader and access adapter, on the existing PostgreSQL 17.6 harness.
//! SQL fixtures advance the actual receipt chains; current authority comes from independent
//! recovery/node/session/lease/Content owners, never from an allocation row or admission cache.
use super::*;
use crate::bestiary_postgres_harness::{
    CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, debug, fence, id,
    runtime,
};
use crate::durability::character_equipment::{
    EquipmentAuthority, RawCastDurableFacts, initialize_equipment_in_transaction,
    read_raw_cast_facts_in_transaction,
};
use crate::durability::character_wheel::{
    WHEEL_SLOTS, WheelChangeFacts, WheelChangeRequest, WheelCommitOutcome, WheelDomain,
    WheelOccurrence, WheelRuleset, WheelSlots, read_current_allocation_in_transaction,
};
use crate::foundation::{
    ChannelContentPin, ChannelId, CommandId, ConnectionGeneration, GameSessionId,
    ScopeOwnershipGeneration, WorldId,
};
use std::sync::Arc;

const DIGEST: [u8; 32] = [1; 32];
const LEVEL: i64 = 1050;
const EXPERIENCE: i64 = 5000;
const READY_CHARACTER_REVISION: u64 = 4;

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, true).await?;
        let result = body(&h).await;
        h.cleanup().await?;
        result
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}
fn advance(original: u64, level: i64, experience: i64) -> String {
    format!(
        "UPDATE game_character_roots SET character_revision = {committed} \
          WHERE character_id = {character} AND character_revision = {original}; \
         UPDATE game_character_progression_state \
            SET character_revision = {committed}, level = {level}, total_experience = {experience} \
          WHERE character_id = {character} AND character_revision = {original};",
        committed = original + 1,
        character = uuid(CHARACTER),
    )
}

/// The same exact receipt-chain fixture shape as character_wheel_postgres_cases, with enough
/// independently declared XP/levels to allocate a complete domain (1000 points).
fn award() -> String {
    format!(
        "{advance}INSERT INTO game_character_xp_receipts(\
           reward_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, level_before, level_after, \
           experience_before, experience_after, experience_awarded, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         VALUES ({occurrence}, '\\x{binding}'::bytea, '\\x{digest}'::bytea, {character}, \
           1, 2, 50, {LEVEL}, 1000, {EXPERIENCE}, {awarded}, 'profile-1', 'ruleset-1', \
           'content-1', 'simulation-1', 'evidence-1', 'declaration-1', 'policy-1', 'reward-1', 1);",
        advance = advance(1, LEVEL, EXPERIENCE),
        occurrence = uuid(90),
        binding = hex(&[90; 8]),
        digest = hex(&[90; 32]),
        character = uuid(CHARACTER),
        awarded = EXPERIENCE - 1000,
    )
}

/// Independently declared vocation choice followed by a real promotion receipt. Neither
/// eligibility nor promotion is inferred from a Wheel row.
fn build_change(seed_id: u8, original: u64, before: &str, after: &str) -> String {
    let columns = "magic_level{s}, mana_spent{s}, fist_level{s}, fist_tries{s}, club_level{s}, \
                   club_tries{s}, sword_level{s}, sword_tries{s}, axe_level{s}, axe_tries{s}, \
                   distance_level{s}, distance_tries{s}, shielding_level{s}, shielding_tries{s}, \
                   fishing_level{s}, fishing_tries{s}";
    let seed = "0, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0, 10, 0";
    let committed = original + 1;
    let character = uuid(CHARACTER);
    let occurrence = uuid(seed_id);
    let state = if before == "none" {
        format!(
            "INSERT INTO game_character_build_state(character_id, vocation, {plain}, \
               committed_character_revision, last_build_occurrence_id) \
             VALUES ({character}, '{after}', {seed}, {committed}, {occurrence});",
            plain = columns.replace("{s}", ""),
        )
    } else {
        format!(
            "UPDATE game_character_build_state SET vocation = '{after}', \
             committed_character_revision = {committed}, last_build_occurrence_id = {occurrence} \
             WHERE character_id = {character};"
        )
    };
    format!(
        "{advance}INSERT INTO game_character_build_receipts(\
           build_occurrence_id, command_binding, policy_digest, character_id, \
           original_character_revision, committed_character_revision, cause, level_before, \
           level_after, experience_before, experience_after, vocation_before, {before_cols}, \
           vocation_after, {after_cols}, stance_before, stance_after, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         VALUES ({occurrence}, '\\x{binding}'::bytea, '\\x{digest}'::bytea, {character}, \
           {original}, {committed}, '{cause}', {LEVEL}, {LEVEL}, {EXPERIENCE}, {EXPERIENCE}, \
           '{before}', {seed}, '{after}', {seed}, NULL, NULL, 'profile-1', 'ruleset-1', \
           'content-1', 'simulation-1', 'evidence-1', 'declaration-1', 'policy-1', 'reward-1', 2); \
         {state}",
        advance = advance(original, LEVEL, EXPERIENCE),
        before_cols = columns.replace("{s}", "_before"),
        after_cols = columns.replace("{s}", "_after"),
        cause = if before == "none" {
            "vocation_choice"
        } else {
            "promotion"
        },
        binding = hex(&[seed_id; 33]),
        digest = hex(&[seed_id; 32]),
    )
}

async fn prepare_current_owners(h: &Harness) -> TestResult {
    for script in [
        award(),
        build_change(91, 2, "none", "sorcerer"),
        build_change(92, 3, "sorcerer", "master_sorcerer"),
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(script))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice())
        .bind(DIGEST.as_slice()).bind([2_u8; 32].as_slice()).bind([3_u8; 32].as_slice())
        .execute(&h.pool).await?;
    // Explicit admission/setup action. None of the cast reader tests initializes equipment.
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let authority = assert_equipment_authority_in_transaction(
        &mut tx,
        &h.root,
        &recovery,
        &h.node,
        &current_fence()?,
        command(1)?,
        DIGEST,
    )
    .await
    .map_err(debug)?;
    initialize_equipment_in_transaction(&mut tx, &authority)
        .await
        .map_err(debug)?;
    tx.commit().await?;
    assert_eq!(
        h.root_revision().await?,
        READY_CHARACTER_REVISION.to_string()
    );
    Ok(())
}

fn current_fence() -> TestResult<CurrentCharacterItemFence> {
    let fixture = fence(READY_CHARACTER_REVISION)?;
    Ok(CurrentCharacterItemFence {
        character_id: fixture.character_id,
        game_session_id: fixture.game_session_id,
        connection_generation: fixture.connection_generation,
        character_lease_generation: fixture.character_lease_generation,
        runtime_scope: fixture.runtime_scope,
        scope_ownership_generation: fixture.scope_ownership_generation,
    })
}
fn command(value: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        CommandId::new(value).map_err(debug)?,
    ))
}
fn ruleset() -> TestResult<Arc<WheelRuleset>> {
    Ok(Arc::new(
        crate::wheel_gem_data::WheelGemData::embedded()
            .map_err(debug)?
            .wheel_ruleset()
            .map_err(debug)?,
    ))
}
fn current_actor(h: &Harness) -> TestResult<(ChannelRuntimeV1, ExactActorRef)> {
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let mut owner = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        // The test's control producer just issued this exact pin into the physical SQL owner.
        ChannelContentPin::from_activation(world, 1, DIGEST, [2; 32], [3; 32], [4; 32], (0, 0, 0)),
    )
    .map_err(debug)?;
    let session = GameSessionId::decode(&id(SESSION)).map_err(debug)?;
    let reservation = owner.reserve_fresh_session(session).map_err(debug)?;
    let actor = owner.commit_fresh_session(reservation).map_err(debug)?;
    owner.player_control_facts(actor, session).map_err(debug)?;
    Ok((owner, actor))
}
fn binding(actor: ExactActorRef, character_revision: u64) -> TestResult<CastFactsBinding> {
    Ok(CastFactsBinding {
        actor,
        session: GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        character: id(CHARACTER),
        character_revision,
        lease_generation: 1,
        connection_generation: 1,
        player_revision: 1,
        content_digest: DIGEST,
        equipment_revision: 1,
    })
}

/// The independently controlled external owner advertises Premium and a deliberately wrong
/// admission-cache Wheel map. The real SQL adapter must replace, never trust, that map.
struct ExternalAccess {
    premium_binding: CastFactsBinding,
    valid_until: u64,
}
impl owner_registration::Registered for ExternalAccess {}
impl CurrentSpellAccessOwner for ExternalAccess {
    fn account_id(&self) -> Option<[u8; 16]> {
        Some(id(40))
    }
    fn read_current(
        &self,
        _: &CastFactsBinding,
        _: u64,
    ) -> Result<AccessProjections, AccessFactsError> {
        Ok(AccessProjections {
            premium: Some(CurrentProjection {
                binding: self.premium_binding.clone(),
                authority_revision: 10,
                valid_until_micros: self.valid_until,
                value: true,
            }),
            wheel: Some(CurrentProjection {
                binding: self.premium_binding.clone(),
                authority_revision: 999,
                valid_until_micros: u64::MAX,
                value: BTreeMap::from([("Beam Mastery".into(), 3)]),
            }),
            ..AccessProjections::default()
        })
    }
}
fn project(
    read: &crate::durability::character_wheel::CurrentWheelCastRead,
    raw: &RawCastDurableFacts,
    ruleset: &WheelRuleset,
    binding: &CastFactsBinding,
    now: u64,
) -> TestResult<AccessProjections> {
    WheelCastAccess {
        other: &ExternalAccess {
            premium_binding: binding.clone(),
            valid_until: 100,
        },
        read,
        raw,
        ruleset,
    }
    .read_current(binding, now)
    .map_err(|error| debug(error).into())
}
async fn runtime_tx(h: &Harness) -> TestResult<Transaction<'_, Postgres>> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}
async fn equipment_authority(
    tx: &mut Transaction<'_, Postgres>,
    h: &Harness,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<EquipmentAuthority> {
    assert_equipment_authority_in_transaction(
        tx,
        &h.root,
        recovery,
        &h.node,
        &current_fence()?,
        command(2)?,
        DIGEST,
    )
    .await
    .map_err(|error| debug(error).into())
}
async fn snapshot(h: &Harness) -> TestResult<Vec<String>> {
    let mut output = Vec::new();
    for relation in [
        "game_character_roots",
        "game_character_progression_state",
        "game_character_xp_receipts",
        "game_character_build_state",
        "game_character_build_receipts",
        "game_character_wheel_state",
        "game_character_wheel_slots",
        "game_character_wheel_receipts",
        "game_wheel_ruleset_revisions",
        "game_wheel_ruleset_slot_capacities",
        "game_character_equipment_state",
        "game_character_equipment_slots",
        "game_character_equipment_receipts",
        "game_character_equipment_audit_outbox",
        "game_item_instances",
        "game_item_container_entries",
        "game_durability_reconnect_sessions",
        "game_content_activations",
    ] {
        output.push(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text), '[]'::jsonb)::text FROM {relation} t"
        ))).fetch_one(&h.pool).await?);
    }
    Ok(output)
}
async fn allocate(
    h: &Harness,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    rules: &Arc<WheelRuleset>,
) -> TestResult {
    let mut points = [0; WHEEL_SLOTS];
    for (index, slot) in rules.slots().iter().enumerate() {
        if slot.domain == WheelDomain::Green {
            points[index] = slot.capacity;
        }
    }
    let outcome = h
        .root
        .commit_character_wheel(
            recovery,
            &h.node,
            fence(READY_CHARACTER_REVISION)?,
            WheelChangeRequest {
                occurrence: WheelOccurrence::from_bytes(id(93)).map_err(debug)?,
                expected_wheel_revision: 0,
                slots: WheelSlots::new(points).map_err(debug)?,
            },
            Arc::clone(rules),
            WheelChangeFacts {
                promoted: true,
                at_temple: false,
            },
        )
        .await
        .map_err(debug)?;
    let WheelCommitOutcome::Committed(receipt) = outcome else {
        return Err(format!("expected real allocation commit, got {outcome:?}").into());
    };
    assert_eq!(
        receipt.original_character_revision.get(),
        READY_CHARACTER_REVISION
    );
    assert_eq!(
        receipt.committed_character_revision.get(),
        READY_CHARACTER_REVISION + 1
    );
    Ok(())
}

#[test]
fn current_cast_reads_no_row_then_committed_stages_and_expires_premium() -> TestResult {
    run("wheel_cast_current", async |h| {
        prepare_current_owners(h).await?;
        let rules = ruleset()?;
        let (owner, actor) = current_actor(h)?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = snapshot(h).await?;
        let mut tx = runtime_tx(h).await?;
        let authority = equipment_authority(&mut tx, h, &recovery).await?;
        let raw = read_raw_cast_facts_in_transaction(&mut tx, &authority)
            .await
            .map_err(debug)?;
        let empty = read_current_allocation_in_transaction(&mut tx, &authority, &rules)
            .await
            .map_err(debug)?;
        assert_eq!(raw.level, u32::try_from(LEVEL)?);
        assert_eq!(raw.build.vocation(), "master_sorcerer");
        assert_eq!(raw.account, id(40));
        assert_eq!(empty.allocation().wheel_revision, 0);
        assert!(empty.allocation().wheel_ruleset_revision.is_none());
        assert!(empty.allocation().current);
        assert_eq!(empty.allocation().slots, WheelSlots::ZERO);
        let current = binding(actor, READY_CHARACTER_REVISION)?;
        owner
            .player_control_facts(actor, current.session)
            .map_err(debug)?;
        let zero = project(&empty, &raw, &rules, &current, 99)?
            .wheel
            .ok_or("current zero")?;
        assert_eq!(zero.authority_revision, READY_CHARACTER_REVISION);
        assert!(zero.value.values().all(|stage| *stage == 0));
        assert!(!zero.value.is_empty());
        assert!(
            project(&empty, &raw, &rules, &current, 100)?
                .wheel
                .is_none()
        );
        tx.commit().await?;
        assert_eq!(
            snapshot(h).await?,
            before,
            "no-row cast reader wrote hidden initialization"
        );

        allocate(h, &recovery, &rules).await?;
        let before = snapshot(h).await?;
        let mut tx = runtime_tx(h).await?;
        let authority = equipment_authority(&mut tx, h, &recovery).await?;
        let raw = read_raw_cast_facts_in_transaction(&mut tx, &authority)
            .await
            .map_err(debug)?;
        let read = read_current_allocation_in_transaction(&mut tx, &authority, &rules)
            .await
            .map_err(debug)?;
        let current = binding(actor, READY_CHARACTER_REVISION + 1)?;
        assert_eq!(read.allocation().wheel_revision, 1);
        let projected = project(&read, &raw, &rules, &current, 99)?
            .wheel
            .ok_or("allocated stage")?;
        let green = &rules
            .vocation("master_sorcerer")
            .ok_or("rules")?
            .revelations[0];
        assert_eq!(projected.value[green], 3);
        assert_eq!(projected.authority_revision, READY_CHARACTER_REVISION + 1);
        assert!(project(&read, &raw, &rules, &current, 100)?.wheel.is_none());
        for changed_field in 0..5 {
            let mut independent_current = current.clone();
            match changed_field {
                0 => independent_current.equipment_revision += 1,
                1 => independent_current.character_revision += 1,
                2 => independent_current.lease_generation += 1,
                3 => independent_current.connection_generation += 1,
                _ => independent_current.content_digest = [9; 32],
            }
            assert!(
                project(&read, &raw, &rules, &independent_current, 99)?
                    .wheel
                    .is_none()
            );
        }
        // Retained pre-write data cannot qualify under the independently advanced Character owner.
        assert!(
            project(&empty, empty.raw(), &rules, &current, 99)?
                .wheel
                .is_none()
        );
        tx.commit().await?;
        assert_eq!(snapshot(h).await?, before, "stage read wrote durable state");
        Ok(())
    })
}

#[test]
fn cast_equipment_fences_reject_independently_stale_owners_and_other_physical_transactions()
-> TestResult {
    run("wheel_cast_fences", async |h| {
        prepare_current_owners(h).await?;
        let rules = ruleset()?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let expected = current_fence()?;
        let before = snapshot(h).await?;
        let mut cases = Vec::new();
        let mut stale = expected;
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        cases.push(stale);
        let mut stale = expected;
        stale.character_lease_generation = 2;
        cases.push(stale);
        let mut stale = expected;
        stale.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        cases.push(stale);
        let mut stale = expected;
        stale.game_session_id = GameSessionId::decode(&id(51)).map_err(debug)?;
        cases.push(stale);
        for stale in cases {
            let mut tx = runtime_tx(h).await?;
            assert!(
                assert_equipment_authority_in_transaction(
                    &mut tx,
                    &h.root,
                    &recovery,
                    &h.node,
                    &stale,
                    command(2)?,
                    DIGEST,
                )
                .await
                .is_err()
            );
            tx.commit().await?;
            assert_eq!(snapshot(h).await?, before);
        }
        let mut tx = runtime_tx(h).await?;
        assert!(
            assert_equipment_authority_in_transaction(
                &mut tx,
                &h.root,
                &recovery,
                &h.node,
                &expected,
                command(2)?,
                [9; 32],
            )
            .await
            .is_err(),
            "independently active Content must agree"
        );
        tx.commit().await?;
        assert_eq!(snapshot(h).await?, before);
        let mut first = runtime_tx(h).await?;
        let authority = equipment_authority(&mut first, h, &recovery).await?;
        let mut other = runtime_tx(h).await?;
        assert!(
            read_current_allocation_in_transaction(&mut other, &authority, &rules)
                .await
                .is_err()
        );
        other.commit().await?;
        first.commit().await?;
        assert_eq!(
            snapshot(h).await?,
            before,
            "rejected cast authority leaked a durable write"
        );
        Ok(())
    })
}

#[test]
fn current_cast_withholds_reset_ruleset_and_cannot_relabel_an_old_sql_read() -> TestResult {
    run("wheel_cast_ruleset", async |h| {
        prepare_current_owners(h).await?;
        let rules = ruleset()?;
        let (_, actor) = current_actor(h)?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        allocate(h, &recovery, &rules).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query(
            "INSERT INTO game_wheel_ruleset_revisions VALUES ('wheel-cast-reset-r2', 2, 'RESET')",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_wheel_ruleset_slot_capacities SELECT 'wheel-cast-reset-r2',slot,capacity FROM game_wheel_ruleset_slot_capacities WHERE wheel_ruleset_revision=$1")
            .bind(rules.revision()).execute(&mut *tx).await?;
        tx.commit().await?;
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../rulesets/progression/wheel-of-destiny/wheel.json"
        ))?;
        let reset = WheelRuleset::from_catalogue("wheel-cast-reset-r2", &document["data"])
            .map_err(debug)?;
        let before = snapshot(h).await?;
        let mut tx = runtime_tx(h).await?;
        let authority = equipment_authority(&mut tx, h, &recovery).await?;
        let raw = read_raw_cast_facts_in_transaction(&mut tx, &authority)
            .await
            .map_err(debug)?;
        let read = read_current_allocation_in_transaction(&mut tx, &authority, &reset)
            .await
            .map_err(debug)?;
        let current = binding(actor, READY_CHARACTER_REVISION + 1)?;
        assert!(!read.allocation().current);
        assert_eq!(read.ruleset_revision(), reset.revision());
        assert_eq!(
            read.allocation().wheel_ruleset_revision.as_deref(),
            Some(rules.revision())
        );
        assert!(project(&read, &raw, &reset, &current, 99)?.wheel.is_none());
        assert!(
            project(&read, &raw, &rules, &current, 99).is_err(),
            "read cannot be relabelled with another ruleset"
        );
        tx.commit().await?;
        assert_eq!(
            snapshot(h).await?,
            before,
            "cast must not reset an allocation"
        );
        Ok(())
    })
}

#[test]
fn corrupt_wheel_read_keeps_real_base_character_equipment_facts_and_rolls_back_injection()
-> TestResult {
    run("wheel_cast_corrupt", async |h| {
        prepare_current_owners(h).await?;
        let rules = ruleset()?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        allocate(h, &recovery, &rules).await?;
        let before = snapshot(h).await?;
        // Isolated migration-admin fault injection inside a transaction that is always rolled back.
        let mut tx = h.pool.begin().await?;
        sqlx::query("UPDATE game_character_wheel_slots SET points=points-1 WHERE character_id=encode($1,'hex')::uuid AND slot=1")
            .bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let authority = equipment_authority(&mut tx, h, &recovery).await?;
        let raw_before = read_raw_cast_facts_in_transaction(&mut tx, &authority)
            .await
            .map_err(debug)?;
        let read = read_current_allocation_in_transaction(&mut tx, &authority, &rules).await;
        assert!(matches!(&read, Err(DurabilityError::InvalidStoredState)));
        assert!(retain_wheel_read(read).map_err(debug)?.is_none());
        // A corrupt Wheel is a semantic unavailable perk, not a failed transaction/base owner.
        let raw_after = read_raw_cast_facts_in_transaction(&mut tx, &authority)
            .await
            .map_err(debug)?;
        assert_eq!(raw_after.equipment, raw_before.equipment);
        assert_eq!(raw_after.build, raw_before.build);
        assert_eq!(raw_after.fence, raw_before.fence);
        assert_eq!(raw_after.level, u32::try_from(LEVEL)?);
        let still_live: bool = sqlx::query_scalar("SELECT true")
            .fetch_one(&mut *tx)
            .await?;
        assert!(still_live);
        tx.rollback().await?;
        assert_eq!(
            snapshot(h).await?,
            before,
            "corruption fixture escaped its rollback"
        );
        Ok(())
    })
}
