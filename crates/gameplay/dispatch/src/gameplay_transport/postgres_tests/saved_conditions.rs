//! C08: persistent conditions across logout and a fresh login on PostgreSQL (Crystal 201189f8
//! `iologindata_save_player.cpp:264` saves them, `iologindata_load_player.cpp:287` loads them,
//! `player.cpp:11565` re-adds them). The real composed logout stores them in the fenced final
//! last-position upsert; the real first entry of a fresh session restores them only from the row
//! that the logout capture wrote.
use super::fixture::{Harness, debug, id, run};
use super::tutorial_vitals_fixture::admit;
use super::{Dependencies, TestResult, prepare_current_session, reset, track};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::LogoutReply;
use oteryn_gameplay_domain::domain::condition::{
    ConditionValues, ConflictKey, DotElement, StatusKind,
};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind,
};
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
};

const DRUNK_MS: u32 = 600_000;
const POISON_TOTAL: u32 = 50;
const COOLDOWN_US: u64 = 600_000_000;

/// Make the admitted actor drunk for `DRUNK_MS` at the owner's clock, poisoned by a creature for
/// `POISON_TOTAL` and with a running `exura` cooldown of `COOLDOWN_US`.
async fn make_drunk(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let state = states
        .get_mut(
            &runtime,
            dependencies.actor,
            dependencies.admitted.game_session_id,
        )
        .ok_or("spell state")?;
    let definition = ConditionDefinition::new(
        "drunk",
        1,
        ConditionValues::TimedStatus {
            kind: StatusKind::Drunk,
            duration_ms: DRUNK_MS,
        },
    )
    .ok_or("drunk")?;
    let root = GameplayDecisionRoot::from_bytes([0; 32]);
    let facts = ApplicationFacts {
        now: owner.owner_now().get(),
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
    };
    state
        .conditions
        .apply(
            &definition,
            None,
            ConditionSourceKind::Creature,
            &[],
            &facts,
        )
        .map_err(debug)?;
    let poison = ConditionDefinition::new(
        "poison",
        1,
        ConditionValues::DamageOverTime {
            element: DotElement::Poison,
            total_min: POISON_TOTAL,
            total_max: POISON_TOTAL,
            per_tick: 5,
            interval_ms: 4_000,
            delayed: true,
        },
    )
    .ok_or("poison")?;
    state
        .conditions
        .apply(
            &poison,
            Some("creature:fixture".to_owned()),
            ConditionSourceKind::Creature,
            &[],
            &facts,
        )
        .map_err(debug)?;
    state.cooldowns.spells.insert(
        "exura".to_owned(),
        SemanticTimeMicros::from_micros(owner.owner_now().get() + COOLDOWN_US),
    );
    Ok(())
}

/// C08: the restored poison (its remaining total and saved creature provenance) and the
/// re-armed `exura` cooldown (the time it has left) of the admitted actor.
async fn restored_poison_and_cooldown(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
) -> TestResult<(Option<(u32, Option<String>)>, Option<u64>)> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states
        .get(
            &runtime,
            dependencies.actor,
            dependencies.admitted.game_session_id,
        )
        .ok_or("spell state")?;
    let poison = state
        .conditions
        .get(ConflictKey::Element(DotElement::Poison))
        .map(|poison| (poison.remaining_total(), poison.provenance().source.clone()));
    let cooldown = state
        .cooldowns
        .spells
        .get("exura")
        .and_then(|deadline| deadline.get().checked_sub(owner.owner_now().get()));
    Ok((poison, cooldown))
}

async fn drunk_remaining_ms(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
) -> TestResult<Option<i32>> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states
        .get(
            &runtime,
            dependencies.actor,
            dependencies.admitted.game_session_id,
        )
        .ok_or("spell state")?;
    Ok(state
        .conditions
        .get(ConflictKey::Status(StatusKind::Drunk))
        .and_then(|drunk| drunk.remaining_duration_ms(owner.owner_now().get())))
}

/// Logout: the release stores the condition in the fenced final write. Returns the text.
async fn logout_saves(admin: &str) -> TestResult<String> {
    reset();
    let harness =
        Harness::create_with_admission_seed(admin.to_owned(), "c08_saved_logout", true, false)
            .await?;
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let dependencies = Dependencies::new()?;
    let owner = dependencies.owner(&harness, &authority);
    dependencies.initialize(&owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(dependencies.actor)
        .map_err(debug)?;
    track(&owner, &dependencies)?;
    make_drunk(&owner, &dependencies).await?;
    // A periodic write stores no conditions: only the logout capture carries them.
    let written = owner.write_due_last_positions(true).await;
    let periodic: Option<Option<String>> =
        sqlx::query_scalar("SELECT conditions::text FROM game_character_last_positions")
            .fetch_optional(&harness.pool)
            .await?;
    assert_eq!(periodic.flatten(), None, "periodic writes={written}");
    assert_eq!(
        owner.logout_session(dependencies.admitted).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );
    let (saved, lease, sequence): (Option<String>, String, String) = sqlx::query_as(
        "SELECT conditions::text, character_lease_generation::text, write_sequence::text \
           FROM game_character_last_positions",
    )
    .fetch_one(&harness.pool)
    .await?;
    let saved = saved.ok_or("the logout saved no conditions")?;
    let entries: Vec<serde_json::Value> = serde_json::from_str(&saved)?;
    assert_eq!(entries.len(), 3, "{saved}");
    let poison = entries
        .iter()
        .find(|entry| entry["key"] == "poison")
        .ok_or("no saved poison")?;
    assert_eq!(poison["values"]["type"], "damage_over_time");
    assert_eq!(poison["values"]["total"], POISON_TOTAL);
    let cooldown = entries
        .iter()
        .find(|entry| entry["key"] == "exura")
        .ok_or("no saved cooldown")?;
    assert_eq!(cooldown["values"]["type"], "spell_cooldown");
    let entries = entries
        .iter()
        .filter(|entry| entry["key"] == "drunk")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1, "{saved}");
    assert_eq!(entries[0]["key"], "drunk");
    assert_eq!(entries[0]["values"]["type"], "timed_status");
    assert_eq!(entries[0]["values"]["kind"], "drunk");
    let remaining = entries[0]["values"]["duration_ms"]
        .as_u64()
        .ok_or("duration")?;
    assert!(
        remaining > 0 && remaining <= u64::from(DRUNK_MS),
        "{remaining}"
    );
    eprintln!(
        "PG_MARKER c08-logout-saved-conditions lease={lease} sequence={sequence} remaining_ms={remaining}"
    );
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await?;
    Ok(saved)
}

/// Fresh login: the first entry of a new session restores the conditions of the stored row
/// that decides it (seeded here with the text the logout above stored).
async fn login_restores(admin: &str, saved: &str) -> TestResult<Option<i32>> {
    reset();
    let harness =
        Harness::create_with_admission_seed(admin.to_owned(), "c08_saved_login", false, false)
            .await
            .map_err(|e| format!("create: {e}"))?;
    let request = prepare_current_session(&harness)
        .await
        .map_err(|e| format!("prepare: {e}"))?;
    sqlx::query("INSERT INTO game_character_last_positions(character_id,world_id,tile_x,tile_y,tile_floor,coordinate_frame_digest,map_revision_digest,server_artifact_digest,character_lease_generation,write_sequence,health,mana,rest,conditions) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,100,200,7,$3,$3,$3,1,1,NULL,NULL,NULL,$4::text::jsonb)")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind([0u8;32].as_slice()).bind(saved)
        .execute(&harness.pool).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let dependencies = Dependencies::new()?;
    let owner = dependencies.owner(&harness, &authority);
    admit(&owner, &dependencies, &request)
        .await
        .map_err(|e| format!("admit: {e}"))?;
    let remaining = drunk_remaining_ms(&owner, &dependencies).await?;
    eprintln!("PG_MARKER c08-login-restored-conditions remaining_ms={remaining:?}");
    let (poison, cooldown) = restored_poison_and_cooldown(&owner, &dependencies).await?;
    assert_eq!(
        poison,
        Some((
            POISON_TOTAL,
            Some(crate::spell::saved_conditions::SAVED_DAMAGE_SOURCE.to_owned())
        ))
    );
    assert!(
        cooldown.is_some_and(|us| us > 0 && us <= COOLDOWN_US),
        "{cooldown:?}"
    );
    eprintln!(
        "PG_MARKER c08-login-restored-damage-and-cooldown poison={poison:?} cooldown_us={cooldown:?}"
    );
    // The retry of the same first entry keeps the live state: nothing is re-applied.
    admit(&owner, &dependencies, &request)
        .await
        .map_err(|e| format!("admit retry: {e}"))?;
    let retried = drunk_remaining_ms(&owner, &dependencies).await?;
    assert!(
        matches!((retried, remaining), (Some(after), Some(before)) if after <= before),
        "{retried:?} {remaining:?}"
    );
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await?;
    Ok(remaining)
}

#[test]
fn c08_conditions_survive_logout_and_fresh_login_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("c08-saved-conditions", || {
        run(async |admin| {
            let saved = logout_saves(&admin).await?;
            let restored = login_restores(&admin, &saved)
                .await?
                .ok_or("the fresh login restored no drunk condition")?;
            assert!(
                restored > 0 && restored <= i32::try_from(DRUNK_MS)?,
                "{restored}"
            );
            Ok(())
        })
    })
}
