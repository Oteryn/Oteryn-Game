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
use oteryn_gameplay_domain::domain::condition::{ConditionValues, ConflictKey, StatusKind};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind,
};
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

const DRUNK_MS: u32 = 600_000;

/// Make the admitted actor drunk for `DRUNK_MS` at the owner's clock.
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
    Ok(())
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
