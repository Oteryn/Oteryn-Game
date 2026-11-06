//! Completion/Animus boundaries after actual production USE and native Instance entry.
//! All existing event, rollback, survivor and exact replay assertions remain enabled.

use super::super::TestResult;
use super::{Harness, debug, id};
use crate::domain::soulpit::{EncounterEffect, EncounterPhase};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::soulpit::{SoulpitAction, SoulpitStepOutcome, SoulpitStepRequest};
use crate::foundation::RuntimeScopeRefV1;
use sqlx::Row;

/// Produced only after the shared production fixture admits the actual source actor,
/// consumes its core and binds the fresh native destination Instance session.
struct AdmittedSoulpit {
    handoff_id: [u8; 16],
    destination: CurrentCharacterGameplayFence,
}

#[test]
fn soulpit_survivor_completion_and_animus_commit_together_postgres() -> TestResult {
    super::super::soulpit_production::completion_atomicity_case(false)
}

#[test]
fn soulpit_failure_after_animus_insert_rolls_back_completion_postgres() -> TestResult {
    super::super::soulpit_production::completion_atomicity_case(true)
}

/// Linked to the real USE dispatch and physical Instance handoff fixture. The
/// production source/destination owners remain live throughout these durable
/// boundary assertions; no COMMITTED handoff or destination session is fabricated.
pub(in super::super) async fn assert_shared_completion(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    destination: crate::gameplay_transport::connection::AdmittedSession,
    handoff_id: [u8; 16],
    inject_failure: bool,
) -> TestResult {
    let began = std::time::Instant::now();
    let marker = if inject_failure {
        "SOULPIT_ANIMUS_ROLLBACK"
    } else {
        "SOULPIT_ANIMUS_COMMIT"
    };
    let authority = owner.character;
    let fence = crate::gameplay_transport::character_gameplay_fence::current(
        owner,
        destination.game_session_id,
        true,
    )
    .await
    .ok_or("real destination gameplay fence")?;
    let admitted = AdmittedSoulpit {
        handoff_id,
        destination: fence,
    };
    assert!(matches!(
        admitted.destination.runtime_scope,
        RuntimeScopeRefV1::Instance { .. }
    ));
    let initial = h
        .root
        .read_soulpit_encounter(
            authority,
            &h.node,
            admitted.handoff_id,
            admitted.destination.runtime_scope,
            admitted.destination.scope_ownership_generation,
        )
        .await
        .map_err(debug)?
        .ok_or("shared fixture did not commit its Soulpit entry")?;
    assert_eq!(
        initial.revision, 0,
        "fixture must start at the committed entry"
    );
    let initial_snapshot = initial.encounter.snapshot();
    assert_eq!(initial_snapshot.phase, EncounterPhase::Preparing);
    assert_eq!(initial_snapshot.wave, 1);
    assert_eq!(initial_snapshot.surviving_slots, vec![0]);
    let mut lifecycle = Lifecycle {
        h,
        authority,
        admitted,
        species: initial.species,
        revision: initial.revision,
        next_occurrence: 150,
    };
    assert_eq!(
        lifecycle.reward_counts().await?,
        (0, 0),
        "fixture species must be uncompleted"
    );
    let completion = lifecycle
        .clear_four_waves(initial_snapshot.started_at_micros)
        .await?;
    let before = lifecycle.database_snapshot().await?;
    let before_events = lifecycle.event_count().await?;
    assert_eq!(
        before_events, 39,
        "all four waves must be written before completion"
    );
    let live = lifecycle.stored().await?;
    assert_eq!(live.encounter.snapshot().phase, EncounterPhase::Fighting);
    assert_eq!(live.encounter.snapshot().wave, 4);
    assert_eq!(live.encounter.snapshot().monsters_alive, 0);

    if inject_failure {
        install_failure_after_animus(h).await?;
        let _error = h
            .root
            .advance_soulpit(authority, &h.node, completion)
            .await
            .expect_err("survivor receipt accepted the injected failure");
        // Database error text is deliberately redacted by the production owner.
        // Sequence advancement survives rollback and proves the rejecting trigger
        // observed this transaction's freshly inserted Animus row exactly once.
        let probe: (i64, bool) =
            sqlx::query_as("SELECT last_value,is_called FROM soulpit_test_after_animus")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            probe,
            (1, true),
            "failure did not follow the new Animus insert"
        );
        assert_eq!(
            lifecycle.database_snapshot().await?,
            before,
            "completion failure retained projection, event, survivor receipt or Animus changes"
        );
        assert_eq!(lifecycle.reward_counts().await?, (0, 0));
        assert_eq!(lifecycle.event_count().await?, before_events);
        let retained = lifecycle.stored().await?;
        assert_eq!(retained.revision, live.revision);
        assert_eq!(retained.encounter, live.encounter);
        sqlx::query("DROP TRIGGER soulpit_test_reject_survivor ON game_soulpit_survival_rewards")
            .execute(&h.pool)
            .await?;
        sqlx::query("DROP FUNCTION soulpit_test_reject_survivor()")
            .execute(&h.pool)
            .await?;
        sqlx::query("DROP SEQUENCE soulpit_test_after_animus")
            .execute(&h.pool)
            .await?;
    } else {
        let written = h
            .root
            .advance_soulpit(authority, &h.node, completion.clone())
            .await
            .map_err(debug)?;
        assert!(!written.replayed);
        assert_eq!(written.revision, live.revision + 1);
        assert_eq!(
            written.effect,
            EncounterEffect::Completed {
                surviving_slots: vec![0]
            }
        );
        assert_eq!(
            written.encounter.snapshot().phase,
            EncounterPhase::Completed
        );
        lifecycle.assert_committed_together(&completion).await?;
        assert_eq!(lifecycle.reward_counts().await?, (1, 1));
        assert_eq!(lifecycle.event_count().await?, before_events + 1);
        let committed = lifecycle.database_snapshot().await?;
        let replay = h
            .root
            .advance_soulpit(authority, &h.node, completion)
            .await
            .map_err(debug)?;
        assert!(replay.replayed);
        assert_eq!(replay.revision, written.revision);
        assert_eq!(replay.effect, written.effect);
        assert_eq!(replay.encounter, written.encounter);
        assert_eq!(
            lifecycle.database_snapshot().await?,
            committed,
            "historical completion replay changed an event, reward or mastery"
        );
    }
    eprintln!("{marker} POSTGRES_CASE_COMPLETED");
    eprintln!("{marker} duration={:?}", began.elapsed());
    Ok(())
}

struct Lifecycle<'a, 'root, 'recovery> {
    h: &'a Harness,
    authority: &'a ReconciledCharacterAuthority<'root, 'recovery>,
    admitted: AdmittedSoulpit,
    species: String,
    revision: u64,
    next_occurrence: u8,
}

impl Lifecycle<'_, '_, '_> {
    fn request(
        &mut self,
        action: SoulpitAction,
        now_micros: u64,
    ) -> TestResult<SoulpitStepRequest> {
        let occurrence = id(self.next_occurrence);
        self.next_occurrence = self
            .next_occurrence
            .checked_add(1)
            .ok_or("fixture occurrence overflow")?;
        Ok(SoulpitStepRequest {
            handoff_id: self.admitted.handoff_id,
            occurrence,
            runtime_scope: self.admitted.destination.runtime_scope,
            scope_ownership_generation: self.admitted.destination.scope_ownership_generation,
            expected_revision: self.revision,
            now_micros,
            action,
            survivors: vec![self.admitted.destination],
        })
    }

    async fn step(&mut self, action: SoulpitAction, now: u64) -> TestResult<SoulpitStepOutcome> {
        let request = self.request(action, now)?;
        let result = self
            .h
            .root
            .advance_soulpit(self.authority, &self.h.node, request)
            .await
            .map_err(debug)?;
        assert!(!result.replayed);
        assert_eq!(result.revision, self.revision + 1);
        self.revision = result.revision;
        Ok(result)
    }

    /// Exercise every durable spawn/death/cadence transition; the legitimate shared
    /// owner fixture supplies session authority, not a handcrafted COMMITTED row.
    async fn clear_four_waves(&mut self, started: u64) -> TestResult<SoulpitStepRequest> {
        for wave in 1_u8..=4 {
            let wave_start = started
                .checked_add(u64::from(wave - 1) * 5_000_000)
                .ok_or("fixture clock overflow")?;
            let spawned_at = wave_start
                .checked_add(4_000_000)
                .ok_or("fixture clock overflow")?;
            let defeated_at = wave_start
                .checked_add(4_500_000)
                .ok_or("fixture clock overflow")?;
            let next_tick = wave_start
                .checked_add(5_000_000)
                .ok_or("fixture clock overflow")?;
            let spawn = self.step(SoulpitAction::Advance, spawned_at).await?;
            assert_eq!(
                spawn.effect,
                EncounterEffect::SpawnWave {
                    wave,
                    stacks: crate::domain::soulpit::wave_stacks(wave).map_err(debug)?,
                }
            );
            let monster_ids =
                [1_u64, 2, 3, 4, 5, 6, 7].map(|slot| Some(u64::from(wave) * 100 + slot));
            let acknowledged = self
                .step(SoulpitAction::Spawned { wave, monster_ids }, spawned_at)
                .await?;
            assert_eq!(acknowledged.effect, EncounterEffect::NoChange);
            for monster_id in monster_ids.into_iter().flatten() {
                let observed = self
                    .step(SoulpitAction::Defeated { wave, monster_id }, defeated_at)
                    .await?;
                assert_eq!(observed.effect, EncounterEffect::NoChange);
            }
            if wave == 4 {
                return self.request(SoulpitAction::Advance, next_tick);
            }
            let advanced = self.step(SoulpitAction::Advance, next_tick).await?;
            assert_eq!(
                advanced.effect,
                EncounterEffect::WaveStarted { wave: wave + 1 }
            );
        }
        Err("fourth wave did not produce a completion request".into())
    }

    async fn stored(&self) -> TestResult<crate::durability::soulpit::SoulpitStoredEncounter> {
        self.h
            .root
            .read_soulpit_encounter(
                self.authority,
                &self.h.node,
                self.admitted.handoff_id,
                self.admitted.destination.runtime_scope,
                self.admitted.destination.scope_ownership_generation,
            )
            .await
            .map_err(debug)?
            .ok_or_else(|| "committed fight disappeared".into())
    }

    async fn event_count(&self) -> TestResult<i64> {
        Ok(sqlx::query_scalar(
            "SELECT count(*) FROM game_soulpit_events WHERE handoff_id=encode($1,'hex')::uuid",
        )
        .bind(self.admitted.handoff_id.as_slice())
        .fetch_one(&self.h.pool)
        .await?)
    }

    async fn reward_counts(&self) -> TestResult<(i64, i64)> {
        Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_soulpit_survival_rewards WHERE handoff_id=encode($1,'hex')::uuid), (SELECT count(*) FROM game_character_animus_masteries WHERE character_id=encode($2,'hex')::uuid AND species_key=$3)")
            .bind(self.admitted.handoff_id.as_slice()).bind(self.admitted.destination.character_id.as_bytes().as_slice())
            .bind(&self.species).fetch_one(&self.h.pool).await?)
    }

    async fn database_snapshot(&self) -> TestResult<serde_json::Value> {
        Ok(sqlx::query_scalar("SELECT jsonb_build_object( \
            'fight',(SELECT to_jsonb(f) FROM game_soulpit_encounters f WHERE f.handoff_id=encode($1,'hex')::uuid), \
            'events',(SELECT coalesce(jsonb_agg(to_jsonb(e) ORDER BY e.revision),'[]'::jsonb) FROM game_soulpit_events e WHERE e.handoff_id=encode($1,'hex')::uuid), \
            'rewards',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY r.character_id),'[]'::jsonb) FROM game_soulpit_survival_rewards r WHERE r.handoff_id=encode($1,'hex')::uuid), \
            'animus',(SELECT to_jsonb(m) FROM game_character_animus_masteries m WHERE m.character_id=encode($2,'hex')::uuid AND m.species_key=$3))")
            .bind(self.admitted.handoff_id.as_slice()).bind(self.admitted.destination.character_id.as_bytes().as_slice())
            .bind(&self.species).fetch_one(&self.h.pool).await?)
    }

    async fn assert_committed_together(&self, request: &SoulpitStepRequest) -> TestResult {
        let row = sqlx::query("SELECT f.terminal,f.revision,e.effect, \
            (f.updated_xact_id=e.created_xact_id AND e.created_xact_id=r.created_xact_id \
             AND m.xmin::text=mod(r.created_xact_id::text::numeric,4294967296)::text) AS same_transaction \
            FROM game_soulpit_encounters f JOIN game_soulpit_events e ON e.handoff_id=f.handoff_id AND e.occurrence_id=f.last_occurrence \
            JOIN game_soulpit_survival_rewards r ON r.handoff_id=f.handoff_id AND r.occurrence_id=e.occurrence_id AND r.species_key=f.species_key \
            JOIN game_character_animus_masteries m ON m.character_id=r.character_id AND m.species_key=r.species_key \
            WHERE f.handoff_id=encode($1,'hex')::uuid AND e.occurrence_id=encode($2,'hex')::uuid \
              AND r.character_id=encode($3,'hex')::uuid AND f.species_key=$4")
            .bind(self.admitted.handoff_id.as_slice()).bind(request.occurrence.as_slice())
            .bind(self.admitted.destination.character_id.as_bytes().as_slice()).bind(&self.species)
            .fetch_one(&self.h.pool).await?;
        assert!(row.try_get::<bool, _>("terminal")?);
        assert_eq!(
            row.try_get::<i64, _>("revision")?,
            i64::try_from(request.expected_revision + 1)?
        );
        assert_eq!(
            row.try_get::<serde_json::Value, _>("effect")?,
            serde_json::json!({"Completed":{"surviving_slots":[0]}})
        );
        assert!(
            row.try_get::<bool, _>("same_transaction")?,
            "completion projection, event, survivor receipt and new Animus row were not one transaction"
        );
        Ok(())
    }
}

async fn install_failure_after_animus(h: &Harness) -> TestResult {
    // A new rejecting test trigger adds a failure; every production guard remains enabled.
    // xmin's low 32 bits identify the insert's XID even if the database has crossed an epoch.
    sqlx::query("CREATE SEQUENCE soulpit_test_after_animus")
        .execute(&h.pool)
        .await?;
    sqlx::query("GRANT USAGE ON SEQUENCE soulpit_test_after_animus TO PUBLIC")
        .execute(&h.pool)
        .await?;
    sqlx::query(
        "CREATE FUNCTION soulpit_test_reject_survivor() RETURNS trigger LANGUAGE plpgsql AS $$ \
        BEGIN \
          IF NOT EXISTS (SELECT 1 FROM game_character_animus_masteries m \
              WHERE m.character_id=NEW.character_id AND m.species_key=NEW.species_key \
                AND m.xmin::text=mod(pg_current_xact_id()::text::numeric,4294967296)::text) THEN \
            RAISE EXCEPTION 'soulpit atomicity probe did not observe the new Animus insert'; \
          END IF; \
          PERFORM nextval('soulpit_test_after_animus'); \
          RAISE EXCEPTION 'soulpit atomicity injected failure after Animus insert'; \
        END $$",
    )
    .execute(&h.pool)
    .await?;
    sqlx::query("CREATE TRIGGER soulpit_test_reject_survivor BEFORE INSERT ON game_soulpit_survival_rewards \
        FOR EACH ROW EXECUTE FUNCTION soulpit_test_reject_survivor()")
        .execute(&h.pool).await?;
    Ok(())
}

/// Real admitted Instance, native weapon definition and valid candidate shape;
/// refusals must come from the specific scope/current-revision authority fences.
pub(in super::super) async fn prove_instance_proficiency_fences(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    destination: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use crate::durability::character_proficiency::{
        DurableProficiencyState, ProficiencyCause, ProficiencyChangeRequest,
        ProficiencyDefinitions, ProficiencyLineCandidate, ProficiencyOccurrence,
    };
    use crate::durability::character_progression::CharacterProgressionError;
    let definitions = owner
        .proficiency_catalogue
        .clone()
        .ok_or("native proficiency catalogue")?;
    let resolved = definitions
        .resolve_weapon("oteryn:item.tibia.i3264")
        .ok_or("native admitted Soulpit weapon proficiency")?;
    let before = DurableProficiencyState::new(
        resolved.canonical_item_key,
        resolved.definition_key,
        resolved.definition_revision,
        0,
        vec![None; usize::from(resolved.shape.track_shape().perk_levels())],
    )
    .map_err(|error| format!("native training candidate preparation: {error:?}"))?;
    before
        .validate_shape(resolved.shape)
        .map_err(|error| format!("native training candidate preparation: {error:?}"))?;
    let after = DurableProficiencyState::new(
        before.item_key(),
        before.definition_key(),
        before.definition_revision(),
        1,
        before.selections().to_vec(),
    )
    .map_err(|error| format!("native training candidate preparation: {error:?}"))?;
    after
        .validate_shape(resolved.shape)
        .map_err(|error| format!("native training candidate preparation: {error:?}"))?;
    let current = owner
        .current_quest_fence(destination.game_session_id)
        .await
        .map_err(|_| "Instance proficiency current fence")?
        .ok_or("current Instance")?;
    assert!(matches!(
        current.runtime_scope,
        RuntimeScopeRefV1::Instance { .. }
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_proficiency_receipts")
        .fetch_one(&h.pool)
        .await?;
    // Candidate-shape proof only: the Instance scope guard refuses this request
    // before any SQL write; it does not claim this is a committed Channel track.
    let selection_progress = resolved.shape.track_shape().class().thresholds()[0];
    assert_eq!(
        resolved
            .shape
            .track_shape()
            .unlocked_perk_levels(selection_progress),
        1
    );
    let selection_before = DurableProficiencyState::new(
        before.item_key(),
        before.definition_key(),
        before.definition_revision(),
        u64::from(selection_progress),
        before.selections().to_vec(),
    )
    .map_err(|error| format!("selection before constructor: {error:?}"))?;
    selection_before
        .validate_shape(resolved.shape)
        .map_err(|error| format!("selection before native shape: {error:?}"))?;
    let mut selected = selection_before.selections().to_vec();
    selected[0] = Some(0);
    let selection_after = DurableProficiencyState::new(
        before.item_key(),
        before.definition_key(),
        before.definition_revision(),
        u64::from(selection_progress),
        selected,
    )
    .map_err(|error| format!("selection after constructor: {error:?}"))?;
    selection_after
        .validate_shape(resolved.shape)
        .map_err(|error| format!("selection after native shape: {error:?}"))?;
    let non_training = ProficiencyChangeRequest::new(
        ProficiencyOccurrence::from_bytes(id(220))
            .map_err(|error| format!("selection occurrence: {error:?}"))?,
        ProficiencyCause::PerkSelection,
        vec![
            ProficiencyLineCandidate::new(
                ProficiencyCause::PerkSelection,
                selection_before,
                selection_after,
            )
            .map_err(|error| format!("selection candidate: {error:?}"))?,
        ],
        Some(current.expected_character_revision),
        definitions.digest(),
    )
    .map_err(|error| format!("selection request constructor: {error:?}"))?;
    assert!(
        matches!(
            owner
                .root
                .commit_character_proficiency(
                    owner.character,
                    owner.holder,
                    current,
                    non_training,
                    definitions.clone()
                )
                .await,
            Err(CharacterProgressionError::InvalidInput)
        ),
        "non-training writers retain their existing Channel scope restriction"
    );
    let training = ProficiencyChangeRequest::new(
        ProficiencyOccurrence::from_bytes(id(221))
            .map_err(|error| format!("native training occurrence or line: {error:?}"))?,
        ProficiencyCause::Training,
        vec![
            ProficiencyLineCandidate::new(ProficiencyCause::Training, before, after)
                .map_err(|error| format!("native training occurrence or line: {error:?}"))?,
        ],
        None,
        definitions.digest(),
    )
    .map_err(|error| format!("native training candidate preparation: {error:?}"))?;
    let mut stale = current;
    stale.expected_character_revision = crate::domain::CharacterRevision::new(
        current
            .expected_character_revision
            .get()
            .checked_add(1)
            .ok_or("revision overflow")?,
    )
    .map_err(|error| format!("native training candidate preparation: {error:?}"))?;
    assert!(
        matches!(
            owner
                .root
                .commit_character_proficiency(
                    owner.character,
                    owner.holder,
                    stale,
                    training,
                    definitions
                )
                .await,
            Err(CharacterProgressionError::CharacterRevisionMismatch)
        ),
        "Instance training still proves its exact current Character revision"
    );
    let retained: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_proficiency_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        retained, count,
        "rejected Instance intents write no proficiency receipt"
    );
    Ok(())
}
