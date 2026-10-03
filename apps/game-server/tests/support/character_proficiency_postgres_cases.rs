// Handwritten scratch qualification; publish only with the complete PROF-1 gate.
use crate::bestiary_postgres_harness as fixture;
use crate::domain::weapon_proficiency::{ProficiencySelectionShape, ProficiencyThresholdClass};
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_proficiency::{
    CommittedProficiencyChange, DurableProficiencyState as State, MAX_PROFICIENCY_PROGRESS,
    ProficiencyCause as Cause, ProficiencyChangeRequest as Request,
    ProficiencyCommitOutcome as Outcome, ProficiencyDefinitions, ProficiencyLineCandidate as Line,
    ProficiencyOccurrence, ProficiencyWriteRefusal as Refusal, ResolvedProficiencyDefinition,
};
use crate::durability::character_progression::{
    CharacterProgressionError as Error, CurrentCharacterGameplayFence,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::runtime_scope_assignment::{BootstrapSecret, NodeIncarnationProof};
use crate::foundation::{ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
use fixture::{
    CHARACTER, Harness, TestResult, configured_admin, debug, fence, id, register, runtime,
};
use std::sync::Arc;

const SWORD: &str = "oteryn:item.tibia.i3295";
const AXE: &str = "oteryn:item.tibia.i3200";
const SWORD_DEF: &str = "oteryn:proficiency.tibia.p1";
const AXE_DEF: &str = "oteryn:proficiency.tibia.p2";
const UNLOCK: u64 = 1_750;
const DIGEST: [u8; 32] = [1; 32];
const NULLS: [Option<u8>; 3] = [None; 3];

#[derive(Clone, Copy, Default)]
enum Fault {
    #[default]
    None,
    Content,
    Digest,
    Count,
    Canonical,
    Definition,
    Revision,
    Missing,
    Shape,
    Poison,
}
#[derive(Clone, Copy, Default)]
struct Definitions(Fault);
impl Definitions {
    fn checked(&self) {
        assert!(
            !matches!(self.0, Fault::Poison),
            "replay queried current policy"
        );
    }
}
fn shape() -> TestResult<ProficiencySelectionShape> {
    ProficiencySelectionShape::new(ProficiencyThresholdClass::Standard, &[3, 2, 1])
        .map_err(|e| debug(e).into())
}
impl ProficiencyDefinitions for Definitions {
    fn content_revision(&self) -> &str {
        self.checked();
        if matches!(self.0, Fault::Content) {
            "content-2"
        } else {
            "content-1"
        }
    }
    fn digest(&self) -> [u8; 32] {
        self.checked();
        if matches!(self.0, Fault::Digest) {
            [2; 32]
        } else {
            DIGEST
        }
    }
    fn active_weapon_count(&self) -> usize {
        self.checked();
        if matches!(self.0, Fault::Count) { 1 } else { 2 }
    }
    fn resolve_weapon(&self, item: &str) -> Option<ResolvedProficiencyDefinition> {
        self.checked();
        let (canonical, definition) = match item {
            SWORD => (SWORD, SWORD_DEF),
            AXE => (AXE, AXE_DEF),
            _ => return None,
        };
        Some(ResolvedProficiencyDefinition {
            canonical_item_key: if matches!(self.0, Fault::Canonical) {
                AXE
            } else {
                canonical
            }
            .into(),
            definition_key: if matches!(self.0, Fault::Definition) {
                AXE_DEF
            } else {
                definition
            }
            .into(),
            definition_revision: if matches!(self.0, Fault::Revision) {
                "definition-2"
            } else {
                "definition-1"
            }
            .into(),
            shape: shape().ok()?,
        })
    }
    fn retained_definition(
        &self,
        item: &str,
        definition: &str,
        revision: &str,
    ) -> Option<ProficiencySelectionShape> {
        self.checked();
        if matches!(self.0, Fault::Missing)
            || revision != "definition-1"
            || !matches!((item, definition), (SWORD, SWORD_DEF) | (AXE, AXE_DEF))
        {
            return None;
        }
        if matches!(self.0, Fault::Shape) {
            ProficiencySelectionShape::new(ProficiencyThresholdClass::Knight, &[3, 2, 1]).ok()
        } else {
            shape().ok()
        }
    }
}
fn state(item: &str, progress: u64, choices: [Option<u8>; 3]) -> TestResult<State> {
    let definition = match item {
        SWORD => SWORD_DEF,
        AXE => AXE_DEF,
        _ => return Err("unknown fixture Item".into()),
    };
    State::new(item, definition, "definition-1", progress, choices.to_vec())
        .map_err(|e| debug(e).into())
}
fn line(cause: Cause, before: State, after: State) -> TestResult<Line> {
    Line::new(cause, before, after).map_err(|e| debug(e).into())
}
fn request(
    tag: u8,
    cause: Cause,
    lines: Vec<Line>,
    track: Option<u64>,
    digest: [u8; 32],
) -> TestResult<Request> {
    Request::new(
        ProficiencyOccurrence::from_bytes(id(tag)).map_err(debug)?,
        cause,
        lines,
        track
            .map(CharacterRevision::new)
            .transpose()
            .map_err(debug)?,
        digest,
    )
    .map_err(|e| debug(e).into())
}
fn training(tag: u8, item: &str, before: u64, after: u64) -> TestResult<Request> {
    request(
        tag,
        Cause::Training,
        vec![line(
            Cause::Training,
            state(item, before, NULLS)?,
            state(item, after, NULLS)?,
        )?],
        None,
        DIGEST,
    )
}
fn selection(tag: u8, progress: u64, choices: [Option<u8>; 3], track: u64) -> TestResult<Request> {
    request(
        tag,
        Cause::PerkSelection,
        vec![line(
            Cause::PerkSelection,
            state(SWORD, progress, NULLS)?,
            state(SWORD, progress, choices)?,
        )?],
        Some(track),
        DIGEST,
    )
}
fn two_tracks(sword_before: u64) -> TestResult<Request> {
    request(
        61,
        Cause::Training,
        vec![
            training(61, SWORD, sword_before, UNLOCK)?.lines()[0].clone(),
            training(61, AXE, 0, UNLOCK)?.lines()[0].clone(),
        ],
        None,
        DIGEST,
    )
}
async fn snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object( \
        'root',(SELECT jsonb_agg(to_jsonb(r) ORDER BY character_id) FROM game_character_roots r), \
        'progression',(SELECT jsonb_agg(to_jsonb(s) ORDER BY character_id) FROM game_character_progression_state s), \
        'headers',(SELECT jsonb_agg(to_jsonb(h) ORDER BY proficiency_occurrence_id) FROM game_character_proficiency_receipts h), \
        'lines',(SELECT jsonb_agg(to_jsonb(l) ORDER BY proficiency_occurrence_id,item_key) FROM game_character_proficiency_receipt_lines l), \
        'tracks',(SELECT jsonb_agg(to_jsonb(t) ORDER BY character_id,item_key) FROM game_character_proficiency t))::text")
        .fetch_one(&h.pool).await?)
}
struct Actor<'h, 'f, 's> {
    h: &'h Harness,
    authority: &'h ReconciledCharacterAuthority<'f, 's>,
}
struct Attempt<'a> {
    node: &'a NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: Request,
    source: Arc<dyn ProficiencyDefinitions>,
}
impl<'a> Attempt<'a> {
    fn fault(mut self, fault: Fault) -> Self {
        self.source = Arc::new(Definitions(fault));
        self
    }
    fn source(mut self, source: impl ProficiencyDefinitions + 'static) -> Self {
        self.source = Arc::new(source);
        self
    }
    fn fence(mut self, fence: CurrentCharacterGameplayFence) -> Self {
        self.fence = fence;
        self
    }
    fn node(mut self, node: &'a NodeIncarnationProof) -> Self {
        self.node = node;
        self
    }
}
enum Failure {
    Authority,
    Revision,
    Context,
    Invalid,
    Conflict,
    Missing,
    Refused(Refusal),
}
impl Actor<'_, '_, '_> {
    fn at(&self, revision: u64, request: Request) -> TestResult<Attempt<'_>> {
        Ok(Attempt {
            node: &self.h.node,
            fence: fence(revision)?,
            request,
            source: Arc::new(Definitions::default()),
        })
    }
    async fn call(&self, attempt: Attempt<'_>) -> Result<Outcome, Error> {
        self.h
            .root
            .commit_character_proficiency(
                self.authority,
                attempt.node,
                attempt.fence,
                attempt.request,
                attempt.source,
            )
            .await
    }
    async fn commit(
        &self,
        revision: u64,
        request: Request,
    ) -> TestResult<CommittedProficiencyChange> {
        let result = self
            .call(self.at(revision, request)?)
            .await
            .map_err(debug)?;
        match result {
            Outcome::Committed(receipt) => Ok(receipt),
            other => Err(debug(other).into()),
        }
    }
    async fn reject(&self, name: &str, attempt: Attempt<'_>, expected: Failure) -> TestResult {
        let before = snapshot(self.h).await?;
        let result = self.call(attempt).await;
        let matched = match (&expected, &result) {
            (Failure::Authority, Err(Error::AuthorityRejected))
            | (Failure::Revision, Err(Error::CharacterRevisionMismatch))
            | (Failure::Context, Err(Error::ProgressionContextMismatch))
            | (Failure::Invalid, Err(Error::InvalidInput))
            | (Failure::Conflict, Err(Error::ConflictingOccurrence))
            | (Failure::Missing, Err(Error::MissingProgressionState)) => true,
            (Failure::Refused(wanted), Ok(Outcome::Refused(actual))) => wanted == actual,
            _ => false,
        };
        assert!(matched, "{name}: unexpected result {result:?}");
        assert_eq!(
            snapshot(self.h).await?,
            before,
            "{name}: durability changed"
        );
        Ok(())
    }
    async fn reconcile(
        &self,
        revision: u64,
        request: Request,
    ) -> Result<Option<CommittedProficiencyChange>, Error> {
        self.h
            .root
            .reconcile_character_proficiency(
                self.authority,
                CharacterId::from_bytes(id(CHARACTER)).map_err(|_| Error::InvalidInput)?,
                CharacterRevision::new(revision).map_err(|_| Error::InvalidInput)?,
                request,
            )
            .await
    }
}
fn run<F>(tag: &'static str, initialized: bool, body: F) -> TestResult
where
    F: AsyncFnOnce(&Actor<'_, '_, '_>) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, initialized).await?;
        if initialized {
            contention::assert_seed_context(&h).await?;
        }
        let result = {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            body(&Actor {
                h: &h,
                authority: &authority,
            })
            .await
        };
        h.cleanup().await?;
        result
    })
}

/// D325: migrations 0030..0053 applied in order keep both the PROF-1 and the FAMILIAR-1
/// receipt kinds in the shared progression guard; interleaved writes commit one chain.
#[test]
fn proficiency_and_familiar_writes_share_the_0053_progression_guard() -> TestResult {
    use crate::durability::character_familiar::{
        DurableFamiliarState, FamiliarStateOccurrence, FamiliarStateOutcome, FamiliarStateRequest,
    };
    run("prof1_familiar_guard", true, async |a| {
        let familiar = |tag: u8, before: DurableFamiliarState, after: DurableFamiliarState| {
            Ok::<_, Box<dyn std::error::Error>>(FamiliarStateRequest {
                occurrence: FamiliarStateOccurrence::from_bytes(id(tag)).map_err(debug)?,
                before,
                after,
                content_revision: "content-1".into(),
                policy_revision: "policy-1".into(),
                policy_digest: [1; 32],
            })
        };
        let summoned = DurableFamiliarState {
            selected_look: 991,
            granted_looks: vec![991],
            saved_expiry_unix: 1_800_001_800,
            lifecycle_epoch: 1,
            familiar_definition: Some("creature:knight/familiar".into()),
            familiar_revision: Some("r20".into()),
            profile_revision: "spell-p2-r20".into(),
            ..DurableFamiliarState::default()
        };
        let mut logout = summoned.clone();
        logout.last_logout_unix = 1_800_000_010;
        let root = &a.h.root;
        let first = root
            .commit_character_familiar_state(
                a.authority,
                &a.h.node,
                fence(1)?,
                familiar(121, DurableFamiliarState::default(), summoned.clone())?,
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(first, FamiliarStateOutcome::Committed(_)),
            "{first:?}"
        );
        let receipt = a.commit(2, two_tracks(0)?).await?;
        assert_eq!(receipt.committed_character_revision.get(), 3);
        let last = root
            .commit_character_familiar_state(
                a.authority,
                &a.h.node,
                fence(3)?,
                familiar(122, summoned, logout)?,
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(last, FamiliarStateOutcome::Committed(_)),
            "{last:?}"
        );
        assert_eq!(a.h.root_revision().await?, "4");
        assert_eq!(a.h.count("game_character_proficiency_receipts").await?, 1);
        assert_eq!(a.h.count("game_character_familiar_receipts").await?, 2);
        // A fresh authority re-verifies the whole mixed chain, both kinds included.
        let restarted = DurabilityRoot::connect_test_runtime(&a.h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let seal = a.h.recovery.seal_current().map_err(debug)?;
        restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        Ok(())
    })
}

#[test]
fn public_proficiency_write_read_replay_and_recovery_are_exact() -> TestResult {
    run("prof1_roundtrip", true, async |a| {
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let root = &a.h.root;
        assert!(
            root.read_character_proficiency_state(a.authority, character)
                .await
                .map_err(debug)?
                .is_empty()
        );
        assert!(ProficiencyOccurrence::from_bytes([7; 16]).is_err());
        let award = two_tracks(0)?;
        let receipt = a.commit(1, award.clone()).await?;
        assert_eq!(
            (
                receipt.original_character_revision.get(),
                receipt.committed_character_revision.get(),
                receipt.lines.len()
            ),
            (1, 2, 2)
        );
        assert_eq!(a.h.count("game_character_proficiency_receipts").await?, 1);
        assert_eq!(a.h.root_revision().await?, "2");
        assert_eq!(
            a.h.count("game_character_proficiency_receipt_lines")
                .await?,
            2
        );
        let tracks = root
            .read_character_proficiency_state(a.authority, character)
            .await
            .map_err(debug)?;
        assert_eq!(tracks.len(), 2);
        for track in tracks {
            assert_eq!(track.state, state(track.state.item_key(), UNLOCK, NULLS)?);
            assert_eq!(
                (
                    track.committed_character_revision.get(),
                    track.last_proficiency_occurrence_id
                ),
                (2, id(61))
            );
        }
        let unchanged = snapshot(a.h).await?;
        assert_eq!(
            a.call(a.at(1, award.clone())?.fault(Fault::Poison))
                .await
                .map_err(debug)?,
            Outcome::AlreadyCommitted(receipt.clone())
        );
        assert_eq!(
            a.reconcile(1, award.clone()).await.map_err(debug)?,
            Some(receipt.clone())
        );
        let changed_line = request(
            61,
            Cause::Training,
            vec![
                training(61, SWORD, 0, UNLOCK + 1)?.lines()[0].clone(),
                training(61, AXE, 0, UNLOCK)?.lines()[0].clone(),
            ],
            None,
            DIGEST,
        )?;
        for changed in [
            request(61, Cause::Training, award.lines().to_vec(), None, [2; 32])?,
            changed_line,
        ] {
            a.reject(
                "occurrence binding",
                a.at(1, changed.clone())?,
                Failure::Conflict,
            )
            .await?;
            assert!(matches!(
                a.reconcile(1, changed).await,
                Err(Error::ConflictingOccurrence)
            ));
        }
        a.reject(
            "bound global revision",
            a.at(2, award.clone())?,
            Failure::Conflict,
        )
        .await?;
        assert_eq!(
            a.reconcile(
                1,
                request(79, Cause::Training, award.lines().to_vec(), None, DIGEST)?
            )
            .await
            .map_err(debug)?,
            None
        );
        assert_eq!(snapshot(a.h).await?, unchanged);
        a.reject(
            "locked slot",
            a.at(2, selection(62, UNLOCK, [None, Some(0), None], 2)?)?,
            Failure::Invalid,
        )
        .await?;
        a.reject(
            "stale global revision",
            a.at(1, training(63, SWORD, UNLOCK, UNLOCK + 1)?)?,
            Failure::Revision,
        )
        .await?;
        a.commit(2, selection(62, UNLOCK, [Some(2), None, None], 2)?)
            .await?;
        let restarted = DurabilityRoot::connect_test_runtime(&a.h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let seal = a.h.recovery.seal_current().map_err(debug)?;
        let authority = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let tracks = restarted
            .read_character_proficiency_state(&authority, character)
            .await
            .map_err(debug)?;
        for (item, choices, revision) in [(SWORD, [Some(2), None, None], 3), (AXE, NULLS, 2)] {
            let track = tracks
                .iter()
                .find(|t| t.state.item_key() == item)
                .ok_or("missing track")?;
            assert_eq!(track.state, state(item, UNLOCK, choices)?);
            assert_eq!(track.committed_character_revision.get(), revision);
        }
        let progression: (i64, i64, i16, i64) = sqlx::query_as("SELECT level,total_experience,harmony,serene_forced_remaining_micros FROM game_character_progression_state").fetch_one(&a.h.pool).await?;
        assert_eq!(progression, (50, 1000, 0, 0));
        Ok(())
    })
}

// HANDWRITTEN BATCH BOUNDARIES
mod boundaries {
    // Handwritten scratch semantic/restored-history cases; shares the public API fixture only.
    use super::*;
    use crate::durability::DurabilityError;
    use crate::durability::character_authority::CharacterAuthorityError;

    #[test]
    fn public_proficiency_semantics_and_track_revision_are_independently_resolved() -> TestResult {
        run("prof1_semantics", true, async |a| {
            let cap = u64::from(shape()?.track_shape().final_progress());
            a.reject(
                "Mastery cap",
                a.at(1, training(64, SWORD, 0, cap + 1)?)?,
                Failure::Invalid,
            )
            .await?;
            for (name, fault, expected) in [
                (
                    "definition revision",
                    Fault::Revision,
                    Failure::Refused(Refusal::DefinitionRevisionMismatch),
                ),
                (
                    "canonical Item",
                    Fault::Canonical,
                    Failure::Refused(Refusal::DefinitionRevisionMismatch),
                ),
                (
                    "definition binding",
                    Fault::Definition,
                    Failure::Refused(Refusal::DefinitionRevisionMismatch),
                ),
                ("retained shape", Fault::Shape, Failure::Context),
                (
                    "retained definition",
                    Fault::Missing,
                    Failure::Refused(Refusal::UnknownTrackDefinition),
                ),
            ] {
                a.reject(
                    name,
                    a.at(1, training(64, SWORD, 0, UNLOCK)?)?.fault(fault),
                    expected,
                )
                .await?;
            }
            let migration = request(
                64,
                Cause::Migration,
                vec![line(
                    Cause::Migration,
                    state(SWORD, 0, NULLS)?,
                    State::new(SWORD, SWORD_DEF, "definition-2", 0, NULLS.to_vec())
                        .map_err(debug)?,
                )?],
                None,
                DIGEST,
            )?;
            a.reject(
                "missing explicit declaration",
                a.at(1, migration)?,
                Failure::Refused(Refusal::MigrationDeclarationUnavailable),
            )
            .await?;
            let award = training(64, SWORD, 0, 100_000)?;
            let receipt = a.commit(1, award.clone()).await?;
            a.reject(
                "track revision",
                a.at(2, selection(65, 100_000, [Some(2), None, None], 1)?)?,
                Failure::Refused(Refusal::StaleTrackRevision),
            )
            .await?;
            a.reject(
                "stale before",
                a.at(2, training(65, SWORD, 0, 100_001)?)?,
                Failure::Refused(Refusal::TrackStateMismatch),
            )
            .await?;
            a.reject(
                "actual third-slot perk count",
                a.at(2, selection(65, 100_000, [None, None, Some(1)], 2)?)?,
                Failure::Invalid,
            )
            .await?;
            let unchanged = snapshot(a.h).await?;
            a.h.root
                .revoke_node_registration(a.h.node.fact())
                .await
                .map_err(debug)?;
            assert_eq!(
                a.call(a.at(1, award.clone())?.fault(Fault::Poison))
                    .await
                    .map_err(debug)?,
                Outcome::AlreadyCommitted(receipt.clone())
            );
            assert_eq!(a.reconcile(1, award).await.map_err(debug)?, Some(receipt));
            a.reject(
                "replay did not restore node",
                a.at(2, training(65, SWORD, 100_000, cap)?)?,
                Failure::Authority,
            )
            .await?;
            assert_eq!(snapshot(a.h).await?, unchanged);
            Ok(())
        })
    }

    // Independently restore coherent retained history, not a new accepted training award.
    // Current controller/session/node authority still comes exclusively from Harness.
    async fn seed_retained(h: &Harness, progress: u64) -> TestResult {
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let binding = training(61, SWORD, 0, progress)?
            .command_binding(character, CharacterRevision::new(1).map_err(debug)?);
        let mut tx = h.pool.begin().await?;
        for sql in [
            "UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid",
            "UPDATE game_character_progression_state SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid",
        ] {
            sqlx::query(sql)
                .bind(id(CHARACTER).as_slice())
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("INSERT INTO game_character_proficiency_receipts( \
            proficiency_occurrence_id,character_id,original_character_revision,committed_character_revision, \
            cause,command_binding,policy_digest,level_before,level_after,experience_before,experience_after, \
            profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision, \
            declaration_revision,policy_revision,reward_revision,committed_at) \
            SELECT encode($2,'hex')::uuid,character_id,1,2,'training',$3,$4,level,level,total_experience,total_experience, \
            profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision, \
            declaration_revision,policy_revision,reward_revision,4 FROM game_character_progression_state \
            WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).bind(id(61).as_slice()).bind(binding.as_slice())
            .bind(DIGEST.as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_proficiency_receipt_lines( \
            proficiency_occurrence_id,character_id,committed_character_revision,cause,item_key, \
            definition_key_before,definition_revision_before,definition_key_after,definition_revision_after, \
            progress_before,progress_after,selections_before,selections_after) \
            VALUES(encode($2,'hex')::uuid,encode($1,'hex')::uuid,2,'training',$3,$4,'definition-1', \
            $4,'definition-1',0,$5,ARRAY[NULL,NULL,NULL]::smallint[],ARRAY[NULL,NULL,NULL]::smallint[])")
            .bind(id(CHARACTER).as_slice()).bind(id(61).as_slice()).bind(SWORD).bind(SWORD_DEF)
            .bind(i64::try_from(progress)?).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_proficiency(character_id,item_key,definition_key,definition_revision, \
            progress,selections,committed_character_revision,last_proficiency_occurrence_id) \
            VALUES(encode($1,'hex')::uuid,$3,$4,'definition-1',$5,ARRAY[NULL,NULL,NULL]::smallint[],2,encode($2,'hex')::uuid)")
            .bind(id(CHARACTER).as_slice()).bind(id(61).as_slice()).bind(SWORD).bind(SWORD_DEF)
            .bind(i64::try_from(progress)?).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    #[test]
    fn public_selection_retains_u64_bigint_progress_and_null_slots() -> TestResult {
        for (name, progress) in [
            ("prof1_u64", u64::from(u32::MAX) + 1),
            ("prof1_bigint", MAX_PROFICIENCY_PROGRESS),
        ] {
            run(name, true, async move |initial| {
                seed_retained(initial.h, progress).await?;
                // Reopen after restored history: a prior empty-state admission is insufficient.
                let seal = initial.h.recovery.seal_current().map_err(debug)?;
                let authority = initial
                    .h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let a = Actor {
                    h: initial.h,
                    authority: &authority,
                };
                let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
                let loaded =
                    a.h.root
                        .read_character_proficiency_state(a.authority, character)
                        .await
                        .map_err(debug)?;
                assert_eq!(loaded.len(), 1);
                assert_eq!(loaded[0].state, state(SWORD, progress, NULLS)?);
                a.commit(2, selection(62, progress, [None, None, Some(0)], 2)?)
                    .await?;
                let loaded =
                    a.h.root
                        .read_character_proficiency_state(a.authority, character)
                        .await
                        .map_err(debug)?;
                assert_eq!(
                    loaded[0].state,
                    state(SWORD, progress, [None, None, Some(0)])?
                );
                assert_eq!(loaded[0].committed_character_revision.get(), 3);
                assert!(state(SWORD, MAX_PROFICIENCY_PROGRESS + 1, NULLS).is_err());
                if progress < MAX_PROFICIENCY_PROGRESS {
                    let award = request(
                        63,
                        Cause::Training,
                        vec![line(
                            Cause::Training,
                            state(SWORD, progress, [None, None, Some(0)])?,
                            state(SWORD, progress + 1, [None, None, Some(0)])?,
                        )?],
                        None,
                        DIGEST,
                    )?;
                    a.reject(
                        "retained above-cap cannot increase",
                        a.at(3, award)?,
                        Failure::Invalid,
                    )
                    .await?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    #[test]
    fn public_proficiency_bootstrap_only_and_mismatched_progression_context_write_nothing()
    -> TestResult {
        for (name, initialized) in [
            ("prof1_missing", false),
            ("prof1_context", false),
            ("prof1_interpretation", true),
        ] {
            run(name, initialized, async move |a| {
                if name == "prof1_context" {
                    sqlx::query("INSERT INTO game_character_progression_state( \
                        character_id,character_revision,level,total_experience,profile_revision,ruleset_revision,content_revision, \
                        simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) \
                        VALUES(encode($1,'hex')::uuid,1,50,1000,'profile-2','ruleset-1','content-1','simulation-1','evidence-1', \
                        'declaration-1','policy-1','reward-1')")
                        .bind(id(CHARACTER).as_slice()).execute(&a.h.pool).await?;
                }
                if initialized {
                    sqlx::query("INSERT INTO game_character_interpretations VALUES (2,'profile-2','ruleset-1','content-1','starter-1',2)")
                        .execute(&a.h.pool).await?;
                }
                let expected = if name == "prof1_missing" {
                    Failure::Missing
                } else {
                    Failure::Context
                };
                a.reject(name, a.at(1, training(63, SWORD, 0, UNLOCK)?)?, expected)
                    .await
            })?;
        }
        Ok(())
    }

    const ORPHAN_HEADER: &str = "INSERT INTO game_character_proficiency_receipts( \
        proficiency_occurrence_id,character_id,original_character_revision,committed_character_revision,cause, \
        command_binding,policy_digest,level_before,level_after,experience_before,experience_after,profile_revision, \
        ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision, \
        reward_revision,committed_at) SELECT '4b020304-0506-7008-800a-0b0c0d0e0f4b'::uuid, \
        '2c020304-0506-7008-800a-0b0c0d0e0f2c'::uuid,original_character_revision,committed_character_revision,cause, \
        command_binding,policy_digest,level_before,level_after,experience_before,experience_after,profile_revision, \
        ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision, \
        reward_revision,committed_at FROM game_character_proficiency_receipts;";
    const ORPHAN_LINE: &str = "INSERT INTO game_character_proficiency_receipt_lines( \
        proficiency_occurrence_id,character_id,committed_character_revision,cause,item_key,definition_key_before, \
        definition_revision_before,definition_key_after,definition_revision_after,progress_before,progress_after, \
        selections_before,selections_after) SELECT proficiency_occurrence_id, \
        '2c020304-0506-7008-800a-0b0c0d0e0f2c'::uuid,committed_character_revision,cause,'oteryn:item.tibia.i3200', \
        definition_key_before,definition_revision_before,definition_key_after,definition_revision_after, \
        progress_before,progress_after,selections_before,selections_after FROM game_character_proficiency_receipt_lines;";
    const ORPHAN_ROW: &str = "INSERT INTO game_character_proficiency(character_id,item_key,definition_key, \
        definition_revision,progress,selections,committed_character_revision,last_proficiency_occurrence_id) \
        SELECT '2c020304-0506-7008-800a-0b0c0d0e0f2c'::uuid,item_key,definition_key,definition_revision,progress, \
        selections,committed_character_revision,last_proficiency_occurrence_id FROM game_character_proficiency;";

    async fn inject_occurrence(tx: &mut sqlx::PgConnection, occurrence: &str) -> TestResult {
        sqlx::raw_sql("ALTER TABLE game_character_proficiency_receipts DROP CONSTRAINT game_character_proficiency_rece_proficiency_occurrence_id_check; \
            ALTER TABLE game_character_proficiency DROP CONSTRAINT game_character_proficiency_last_proficiency_occurrence_id_check;")
            .execute(&mut *tx).await?;
        for sql in [
            "UPDATE game_character_proficiency_receipts SET proficiency_occurrence_id=$1::text::uuid",
            "UPDATE game_character_proficiency_receipt_lines SET proficiency_occurrence_id=$1::text::uuid",
            "UPDATE game_character_proficiency SET last_proficiency_occurrence_id=$1::text::uuid",
        ] {
            sqlx::query(sql).bind(occurrence).execute(&mut *tx).await?;
        }
        sqlx::raw_sql("ALTER TABLE game_character_proficiency_receipts ADD CONSTRAINT game_character_proficiency_rece_proficiency_occurrence_id_check \
            CHECK(game_character_is_uuid_v7(proficiency_occurrence_id)) NOT VALID; \
            ALTER TABLE game_character_proficiency ADD CONSTRAINT game_character_proficiency_last_proficiency_occurrence_id_check \
            CHECK(game_character_is_uuid_v7(last_proficiency_occurrence_id)) NOT VALID;")
            .execute(&mut *tx).await?;
        Ok(())
    }

    #[test]
    fn public_admission_scans_orphan_header_line_row_and_malformed_occurrence() -> TestResult {
        for (name, fault, malformed) in [
            ("prof1_orphan_header", Some(ORPHAN_HEADER), None),
            ("prof1_orphan_line", Some(ORPHAN_LINE), None),
            ("prof1_orphan_row", Some(ORPHAN_ROW), None),
            (
                "prof1_nil",
                None,
                Some("00000000-0000-0000-0000-000000000000"),
            ),
            (
                "prof1_v4",
                None,
                Some("3d020304-0506-4008-800a-0b0c0d0e0f3d"),
            ),
        ] {
            run(name, true, async move |a| {
                a.commit(1, training(61, SWORD, 0, UNLOCK)?).await?;
                let seal = a.h.recovery.seal_current().map_err(debug)?;
                // Successful eight-kind admission is mandatory before the single fault.
                drop(
                    a.h.root
                        .open_character_authority(&seal)
                        .await
                        .map_err(debug)?,
                );
                let mut tx = a.h.pool.begin().await?;
                // Own disposable DB only: runtime cannot bypass these constraints.
                sqlx::raw_sql("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                if let Some(sql) = fault {
                    sqlx::raw_sql(sql).execute(&mut *tx).await?;
                }
                if let Some(id) = malformed {
                    inject_occurrence(&mut tx, id).await?;
                }
                tx.commit().await?;
                match a.h.root.open_character_authority(&seal).await {
                    Err(CharacterAuthorityError::Unavailable(
                        DurabilityError::InvalidStoredState | DurabilityError::Unavailable,
                    )) => {}
                    Err(other) => {
                        return Err(
                            format!("{name}: unexpected non-integrity error {other:?}").into()
                        );
                    }
                    Ok(_) => {
                        return Err(format!(
                            "{name}: restored invalid WP relation acquired authority"
                        )
                        .into());
                    }
                }
                assert_eq!(a.h.root_revision().await?, "2");
                Ok(())
            })?;
        }
        Ok(())
    }

    #[test]
    fn public_proficiency_writer_uses_independent_current_fences_and_policy() -> TestResult {
        run("prof1_fences", true, async |a| {
            for name in [
                "connection",
                "lease",
                "session",
                "character",
                "ownership",
                "scope",
                "zero lease",
            ] {
                let mut changed = fence(1)?;
                match name {
                    "connection" => {
                        changed.connection_generation =
                            ConnectionGeneration::new(2).map_err(debug)?
                    }
                    "lease" => changed.character_lease_generation = 2,
                    "session" => {
                        changed.game_session_id = GameSessionId::decode(&id(51)).map_err(debug)?
                    }
                    "character" => {
                        changed.character_id = CharacterId::from_bytes(id(44)).map_err(debug)?
                    }
                    "ownership" => {
                        changed.scope_ownership_generation =
                            ScopeOwnershipGeneration::new(2).map_err(debug)?
                    }
                    "scope" => {
                        changed.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                            crate::foundation::WorldId::decode(&id(fixture::WORLD))
                                .map_err(debug)?,
                            crate::foundation::ChannelId::decode(&id(44)).map_err(debug)?,
                        )
                    }
                    _ => changed.character_lease_generation = 0,
                }
                let expected = if name == "zero lease" {
                    Failure::Invalid
                } else {
                    Failure::Authority
                };
                a.reject(
                    name,
                    a.at(1, training(63, SWORD, 0, UNLOCK)?)?.fence(changed),
                    expected,
                )
                .await?;
            }
            a.reject(
                "future revision",
                a.at(2, training(63, SWORD, 0, UNLOCK)?)?,
                Failure::Revision,
            )
            .await?;
            let forged =
                NodeIncarnationProof::new(a.h.node.fact(), BootstrapSecret::from_bytes([9; 32]));
            let other = register(&a.h.root, 2).await?;
            for (name, node) in [("forged secret", &forged), ("unassigned node", &other)] {
                a.reject(
                    name,
                    a.at(1, training(63, SWORD, 0, UNLOCK)?)?.node(node),
                    Failure::Authority,
                )
                .await?;
            }
            for (name, fault) in [("content", Fault::Content), ("digest", Fault::Digest)] {
                a.reject(
                    name,
                    a.at(1, training(63, SWORD, 0, UNLOCK)?)?.fault(fault),
                    Failure::Refused(Refusal::PolicyMismatch),
                )
                .await?;
            }
            let digest = request(
                63,
                Cause::Training,
                training(63, SWORD, 0, UNLOCK)?.lines().to_vec(),
                None,
                [2; 32],
            )?;
            a.reject(
                "request digest",
                a.at(1, digest)?,
                Failure::Refused(Refusal::PolicyMismatch),
            )
            .await?;
            a.reject(
                "before",
                a.at(1, training(63, SWORD, 1, UNLOCK)?)?,
                Failure::Refused(Refusal::TrackStateMismatch),
            )
            .await?;
            a.reject(
                "second track atomicity",
                a.at(1, two_tracks(1)?)?,
                Failure::Refused(Refusal::TrackStateMismatch),
            )
            .await?;
            a.reject(
                "source cardinality",
                a.at(1, two_tracks(0)?)?.fault(Fault::Count),
                Failure::Refused(Refusal::LineCountExceeded),
            )
            .await?;
            a.h.root
                .revoke_node_registration(a.h.node.fact())
                .await
                .map_err(debug)?;
            a.reject(
                "ended node",
                a.at(1, training(63, SWORD, 0, UNLOCK)?)?,
                Failure::Authority,
            )
            .await?;
            Ok(())
        })
    }
}

// HANDWRITTEN BATCH CONTENTION
mod contention {
    pub(super) async fn assert_seed_context(h: &Harness) -> TestResult {
        let actual: Vec<String> = sqlx::query_scalar("SELECT ARRAY[profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision] FROM game_character_progression_state")
            .fetch_one(&h.pool).await?;
        let expected = fixture::context();
        assert_eq!(
            actual,
            vec![
                expected.profile,
                expected.ruleset,
                expected.content,
                expected.simulation,
                expected.evidence,
                expected.declaration
            ]
        );
        Ok(())
    }
    // Handwritten scratch contention case. Include as a sibling of boundary cases in
    // the public helper module; no extra fixture, session, controller or policy proof.
    use super::*;
    use std::future::{Future, poll_fn};
    use std::task::Poll;

    // Tokio's workspace macros feature is disabled. This is the existing repository
    // join_two posture: poll both futures, retaining each result exactly once.
    async fn concurrent<A: Future, B: Future>(a: A, b: B) -> (A::Output, B::Output) {
        let mut a = std::pin::pin!(a);
        let mut b = std::pin::pin!(b);
        let (mut first, mut second) = (None, None);
        poll_fn(move |cx| {
            if first.is_none()
                && let Poll::Ready(value) = a.as_mut().poll(cx)
            {
                first = Some(value);
            }
            if second.is_none()
                && let Poll::Ready(value) = b.as_mut().poll(cx)
            {
                second = Some(value);
            }
            match (first.take(), second.take()) {
                (Some(a), Some(b)) => Poll::Ready((a, b)),
                (a, b) => {
                    first = a;
                    second = b;
                    Poll::Pending
                }
            }
        })
        .await
    }

    async fn counts(h: &Harness, revision: u64, count: i64) -> TestResult {
        assert_eq!(h.root_revision().await?, revision.to_string());
        let state: String = sqlx::query_scalar(
            "SELECT character_revision::text FROM game_character_progression_state",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(state, revision.to_string());
        for table in [
            "game_character_proficiency_receipts",
            "game_character_proficiency_receipt_lines",
            "game_character_proficiency",
        ] {
            assert_eq!(h.count(table).await?, count, "{table}");
        }
        Ok(())
    }

    #[test]
    fn public_proficiency_contenders_preserve_losing_delta_for_next_root_revision() -> TestResult {
        run("prof1_public_race", true, async |a| {
            let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
            let sword = a.at(1, training(71, SWORD, 0, UNLOCK)?)?;
            // Each root owns one issued semantic pass. Two independent roots exercise
            // database contention rather than the same root's intentional busy refusal.
            let other = DurabilityRoot::connect_test_runtime(&a.h.database.url)?;
            assert!(other.maintain_ready_once().await?);
            let seal = a.h.recovery.seal_current().map_err(debug)?;
            let other_authority = other.open_character_authority(&seal).await.map_err(debug)?;
            let axe = other.commit_character_proficiency(
                &other_authority,
                &a.h.node,
                fence(1)?,
                training(72, AXE, 0, UNLOCK)?,
                Arc::new(Definitions::default()),
            );
            let results = concurrent(a.call(sword), axe).await;
            let (winner, loser, receipt, losing_occurrence) = match results {
                (Ok(Outcome::Committed(receipt)), Err(Error::CharacterRevisionMismatch)) => {
                    (SWORD, AXE, receipt, 72)
                }
                (Err(Error::CharacterRevisionMismatch), Ok(Outcome::Committed(receipt))) => {
                    (AXE, SWORD, receipt, 71)
                }
                other => {
                    let message =
                        format!("expected one commit and one stale global revision: {other:?}");
                    return Err(message.into());
                }
            };
            assert_eq!(
                (
                    receipt.original_character_revision.get(),
                    receipt.committed_character_revision.get()
                ),
                (1, 2)
            );
            counts(a.h, 2, 1).await?;
            let tracks =
                a.h.root
                    .read_character_proficiency_state(a.authority, character)
                    .await
                    .map_err(debug)?;
            assert_eq!(tracks.len(), 1);
            assert_eq!(tracks[0].state, state(winner, UNLOCK, NULLS)?);
            assert!(!tracks.iter().any(|track| track.state.item_key() == loser));
            let absent: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_proficiency_receipts WHERE proficiency_occurrence_id=encode($1,'hex')::uuid")
                .bind(id(losing_occurrence).as_slice()).fetch_one(&a.h.pool).await?;
            assert_eq!(absent, 0, "losing occurrence left no receipt");

            // The caller retained the losing delta: new occurrence, new current root,
            // unchanged seed-before and exactly one credit for each weapon.
            let retry = a.commit(2, training(73, loser, 0, UNLOCK)?).await?;
            assert_eq!(
                (
                    retry.original_character_revision.get(),
                    retry.committed_character_revision.get()
                ),
                (2, 3)
            );
            counts(a.h, 3, 2).await?;
            let tracks =
                a.h.root
                    .read_character_proficiency_state(a.authority, character)
                    .await
                    .map_err(debug)?;
            assert_eq!(tracks.len(), 2);
            for (item, revision) in [(winner, 2), (loser, 3)] {
                let track = tracks
                    .iter()
                    .find(|track| track.state.item_key() == item)
                    .ok_or("missing retry track")?;
                assert_eq!(track.state, state(item, UNLOCK, NULLS)?);
                assert_eq!(track.committed_character_revision.get(), revision);
            }
            Ok(())
        })
    }
}

// HANDWRITTEN BATCH EXPLICIT_MIGRATION
// Handwritten semantic consumer fixture. This does not qualify content rollout.
mod explicit_migration {
    use super::*;
    use crate::durability::DurabilityError::{InvalidStoredState, Unavailable};
    use crate::durability::character_authority::CharacterAuthorityError as AdmissionError;
    use crate::durability::character_proficiency::{
        DeclaredProficiencyMigration, ProficiencyLevelMigration as Rule,
        StoredProficiencyTrack as Track,
    };
    use Cause::{Migration, PerkSelection, Training};
    use Outcome::{AlreadyCommitted, Committed};
    use ProficiencySelectionShape as Shape;
    use ProficiencyThresholdClass::Standard;
    const OLD: &str = "definition-1";
    const NEW: &str = "definition-2";
    const BEFORE: [Option<u8>; 3] = [Some(2), Some(1), Some(0)];
    const PROGRESS: u64 = 100_000;
    const MIGRATION_DIGEST: [u8; 32] = [2; 32];
    #[derive(Clone, Copy, Debug)]
    enum Fault {
        None,
        Missing,
        Content,
        Digest,
        OldShape,
        NewShape,
        Map,
        CurrentRevision,
        CurrentContent,
        CurrentDigest,
    }
    #[derive(Clone, Copy)]
    struct Source {
        stage: u8,
        fault: Fault,
    }
    impl Source {
        fn view(stage: u8) -> Self {
            Self {
                stage,
                fault: Fault::None,
            }
        }
        fn fault(mut self, fault: Fault) -> Self {
            self.fault = fault;
            self
        }
    }
    fn choices(item: &str) -> Vec<Option<u8>> {
        if item == SWORD {
            vec![Some(0), Some(1)]
        } else {
            vec![Some(2), Some(0), None, None]
        }
    }
    fn actual_shape(item: &str) -> Option<Shape> {
        let counts: &[u8] = match item {
            SWORD => &[2, 2],
            AXE => &[3, 1, 2, 1],
            _ => return None,
        };
        Shape::new(Standard, counts).ok()
    }
    impl ProficiencyDefinitions for Source {
        fn content_revision(&self) -> &str {
            if self.stage == 2 || matches!(self.fault, Fault::CurrentContent) {
                "content-2"
            } else {
                "content-1"
            }
        }
        fn digest(&self) -> [u8; 32] {
            if matches!(self.fault, Fault::CurrentDigest) {
                [8; 32]
            } else if self.stage == 1 {
                MIGRATION_DIGEST
            } else {
                [9; 32]
            }
        }
        fn active_weapon_count(&self) -> usize {
            2
        }
        fn resolve_weapon(&self, item: &str) -> Option<ResolvedProficiencyDefinition> {
            let mut resolved = Definitions::default().resolve_weapon(item)?;
            resolved.definition_revision =
                if self.stage == 1 && !matches!(self.fault, Fault::CurrentRevision) {
                    NEW
                } else {
                    "definition-3"
                }
                .into();
            resolved.shape = actual_shape(item)?;
            Some(resolved)
        }
        fn retained_definition(&self, item: &str, key: &str, revision: &str) -> Option<Shape> {
            if revision == NEW && matches!((item, key), (SWORD, SWORD_DEF) | (AXE, AXE_DEF)) {
                actual_shape(item)
            } else {
                Definitions::default().retained_definition(item, key, revision)
            }
        }
        fn retained_migration(
            &self,
            content: &str,
            digest: &[u8; 32],
            item: &str,
            key: &str,
            old: &str,
            new: &str,
        ) -> Option<DeclaredProficiencyMigration> {
            // Fixed retained context and exact authored revisions; never echo supplied after-state.
            let context = if matches!(self.fault, Fault::Content) {
                "content-other"
            } else {
                "content-1"
            };
            let retained_digest = if matches!(self.fault, Fault::Digest) {
                [3; 32]
            } else {
                MIGRATION_DIGEST
            };
            if content != context
                || digest != &retained_digest
                || (old, new) != (OLD, NEW)
                || matches!(self.fault, Fault::Missing)
                || !matches!((item, key), (SWORD, SWORD_DEF) | (AXE, AXE_DEF))
            {
                return None;
            }
            let old_shape = if matches!(self.fault, Fault::OldShape) {
                Shape::new(Standard, &[2, 2, 1]).ok()?
            } else {
                shape().ok()?
            };
            let new_shape = if matches!(self.fault, Fault::NewShape) {
                shape().ok()?
            } else {
                actual_shape(item)?
            };
            let mut levels = if item == SWORD {
                vec![
                    Rule::Remap(vec![Some(1), None, Some(0)]),
                    Rule::Keep,
                    Rule::Clear,
                ]
            } else {
                vec![
                    Rule::Keep,
                    Rule::Remap(vec![None, Some(0)]),
                    Rule::Clear,
                    Rule::Clear,
                ]
            };
            if matches!(self.fault, Fault::Map) {
                levels[0] = Rule::Clear;
            }
            Some(DeclaredProficiencyMigration {
                old_shape,
                new_shape,
                levels,
            })
        }
    }
    fn retained(
        item: &str,
        revision: &str,
        progress: u64,
        choices: &[Option<u8>],
    ) -> TestResult<State> {
        let key = match item {
            SWORD => SWORD_DEF,
            AXE => AXE_DEF,
            _ => return Err("unknown fixture Item".into()),
        };
        State::new(item, key, revision, progress, choices.to_vec()).map_err(|e| debug(e).into())
    }
    fn migration(tag: u8) -> TestResult<Request> {
        let lines = [SWORD, AXE]
            .into_iter()
            .map(|item| {
                line(
                    Migration,
                    retained(item, OLD, PROGRESS, &BEFORE)?,
                    retained(item, NEW, PROGRESS, &choices(item))?,
                )
            })
            .collect::<TestResult<Vec<_>>>()?;
        request(tag, Migration, lines, None, MIGRATION_DIGEST)
    }
    async fn seed(a: &Actor<'_, '_, '_>) -> TestResult {
        let lines = [SWORD, AXE]
            .into_iter()
            .map(|item| Ok(training(100, item, 0, PROGRESS)?.lines()[0].clone()))
            .collect::<TestResult<Vec<_>>>()?;
        a.commit(1, request(100, Training, lines, None, DIGEST)?)
            .await?;
        let mut revision = 2;
        for item in [SWORD, AXE] {
            let (mut before, mut track, mut selected) = (state(item, PROGRESS, NULLS)?, 2, NULLS);
            for (slot, value) in [2, 1, 0].into_iter().enumerate() {
                selected[slot] = Some(value);
                let after = state(item, PROGRESS, selected)?;
                let intent = request(
                    100 + revision as u8,
                    PerkSelection,
                    vec![line(PerkSelection, before, after.clone())?],
                    Some(track),
                    DIGEST,
                )?;
                a.commit(revision, intent).await?;
                revision += 1;
                track = revision;
                before = after;
            }
        }
        assert_eq!(a.h.root_revision().await?, "8");
        Ok(())
    }
    async fn load(a: &Actor<'_, '_, '_>, source: Source) -> Result<Vec<Track>, Error> {
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(|_| Error::InvalidInput)?;
        a.h.root
            .read_character_proficiency_with_definitions(
                a.authority,
                character,
                None,
                Arc::new(source),
            )
            .await
            .map(|(tracks, _)| tracks)
    }
    fn invalid_read<T>(result: Result<T, Error>) -> bool {
        matches!(result, Err(Error::Unavailable(InvalidStoredState)))
    }
    fn unavailable_read<T>(result: Result<T, Error>) -> bool {
        matches!(result, Err(Error::Unavailable(Unavailable)))
    }
    fn invalid_open<T>(result: Result<T, AdmissionError>) -> bool {
        matches!(result, Err(AdmissionError::Unavailable(Unavailable)))
    }
    #[test]
    fn public_explicit_migration_retains_choices_and_recovery_evidence() -> TestResult {
        run("prof1_migration_retained", true, async |a| {
            seed(a).await?;
            let source = Source::view(1);
            let pending = migration(110)?;
            let committed = match a
                .call(a.at(8, pending.clone())?.source(source))
                .await
                .map_err(debug)?
            {
                Committed(receipt) => receipt,
                other => return Err(debug(other).into()),
            };
            assert_eq!(committed.committed_character_revision.get(), 9);
            let root = &a.h.root;
            let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
            let seal = a.h.recovery.seal_current().map_err(debug)?;
            assert!(unavailable_read(
                root.read_character_proficiency_state(a.authority, character)
                    .await
            ));
            assert!(unavailable_read(
                root.read_character_proficiency_occurrence(
                    a.authority,
                    character,
                    *pending.occurrence().as_bytes()
                )
                .await
            ));
            assert!(invalid_open(root.open_character_authority(&seal).await));
            let changed_current = Arc::new(Source::view(2));
            let (tracks, receipt) = root
                .read_character_proficiency_with_definitions(
                    a.authority,
                    character,
                    Some(pending.occurrence()),
                    changed_current.clone(),
                )
                .await
                .map_err(debug)?;
            assert_eq!(receipt, Some(committed.clone()));
            assert_eq!(tracks.len(), 2);
            for track in tracks {
                let item = track.state.item_key();
                let expected = retained(item, NEW, PROGRESS, &choices(item))?;
                assert_eq!(track.state, expected);
                assert_eq!(track.committed_character_revision.get(), 9);
            }
            let stable = snapshot(a.h).await?;
            assert_eq!(
                a.call(a.at(8, pending.clone())?.source(*changed_current))
                    .await
                    .map_err(debug)?,
                AlreadyCommitted(committed.clone())
            );
            assert_eq!(
                root.reconcile_character_proficiency_with_definitions(
                    a.authority,
                    character,
                    CharacterRevision::new(8).map_err(debug)?,
                    pending.clone(),
                    Some(changed_current.clone())
                )
                .await
                .map_err(debug)?,
                Some(committed)
            );
            let restarted = DurabilityRoot::connect_test_runtime(&a.h.database.url)?;
            assert!(restarted.maintain_ready_once().await?);
            assert!(invalid_open(
                restarted.open_character_authority(&seal).await
            ));
            let reopened = restarted
                .open_character_authority_with_proficiency_definitions(
                    &seal,
                    Some(changed_current.clone()),
                )
                .await
                .map_err(debug)?;
            let (after_restart, _) = restarted
                .read_character_proficiency_with_definitions(
                    &reopened,
                    character,
                    None,
                    changed_current,
                )
                .await
                .map_err(debug)?;
            assert_eq!(after_restart.len(), 2);
            assert_eq!(snapshot(a.h).await?, stable);
            for fault in [
                Fault::Missing,
                Fault::Content,
                Fault::Digest,
                Fault::OldShape,
                Fault::NewShape,
                Fault::Map,
            ] {
                let bad = source.fault(fault);
                let result = load(a, bad).await;
                let missing = matches!(fault, Fault::Missing | Fault::Content | Fault::Digest);
                assert!(
                    if missing {
                        unavailable_read(result)
                    } else {
                        invalid_read(result)
                    },
                    "historical {fault:?}"
                );
                let admitted = root
                    .open_character_authority_with_proficiency_definitions(
                        &seal,
                        Some(Arc::new(bad)),
                    )
                    .await;
                assert!(invalid_open(admitted), "boot {fault:?}");
                assert_eq!(snapshot(a.h).await?, stable);
            }
            // CHECK-valid byte/context mutations; USER guards are bypassed only in this disposable DB.
            for (column, value, cast) in [
                (
                    "command_binding",
                    "set_byte(command_binding,32,(get_byte(command_binding,32)+1)%256)",
                    "bytea",
                ),
                ("content_revision", "'content-other'", "text"),
            ] {
                let before: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT {column}::text FROM game_character_proficiency_receipts WHERE cause='migration'"))).fetch_one(&a.h.pool).await?;
                sqlx::raw_sql(sqlx::AssertSqlSafe(format!("ALTER TABLE game_character_proficiency_receipts DISABLE TRIGGER USER; UPDATE game_character_proficiency_receipts SET {column}={value} WHERE cause='migration'; ALTER TABLE game_character_proficiency_receipts ENABLE TRIGGER USER"))).execute(&a.h.pool).await?;
                let corrupted = snapshot(a.h).await?;
                let result = load(a, source).await;
                assert!(
                    if column == "content_revision" {
                        unavailable_read(result)
                    } else {
                        invalid_read(result)
                    },
                    "corrupt {column}"
                );
                assert_eq!(
                    snapshot(a.h).await?,
                    corrupted,
                    "read changed corrupt {column}"
                );
                let admitted = root
                    .open_character_authority_with_proficiency_definitions(
                        &seal,
                        Some(Arc::new(source)),
                    )
                    .await;
                assert!(invalid_open(admitted));
                assert_eq!(
                    snapshot(a.h).await?,
                    corrupted,
                    "open changed corrupt {column}"
                );
                sqlx::query("ALTER TABLE game_character_proficiency_receipts DISABLE TRIGGER USER")
                    .execute(&a.h.pool)
                    .await?;
                sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE game_character_proficiency_receipts SET {column}=$1::text::{cast} WHERE cause='migration'"))).bind(before).execute(&a.h.pool).await?;
                sqlx::query("ALTER TABLE game_character_proficiency_receipts ENABLE TRIGGER USER")
                    .execute(&a.h.pool)
                    .await?;
                assert_eq!(snapshot(a.h).await?, stable);
            }
            let xp: (i64, i64) = sqlx::query_as(
                "SELECT level,total_experience FROM game_character_progression_state",
            )
            .fetch_one(&a.h.pool)
            .await?;
            assert_eq!(xp, (50, 1000));
            Ok(())
        })
    }
    #[test]
    fn public_migration_refuses_independent_semantic_faults_atomically() -> TestResult {
        run("prof1_migration_refusals", true, async |a| {
            seed(a).await?;
            let source = Source::view(1);
            for fault in [
                Fault::Missing,
                Fault::Content,
                Fault::Digest,
                Fault::OldShape,
                Fault::NewShape,
                Fault::Map,
                Fault::CurrentRevision,
                Fault::CurrentContent,
                Fault::CurrentDigest,
            ] {
                let expected = match fault {
                    Fault::Missing | Fault::Content | Fault::Digest => {
                        Failure::Refused(Refusal::MigrationDeclarationUnavailable)
                    }
                    Fault::OldShape | Fault::NewShape | Fault::Map => Failure::Invalid,
                    Fault::CurrentRevision => Failure::Refused(Refusal::DefinitionRevisionMismatch),
                    _ => Failure::Refused(Refusal::PolicyMismatch),
                };
                a.reject(
                    &format!("{fault:?}"),
                    a.at(8, migration(110)?)?.source(source.fault(fault)),
                    expected,
                )
                .await?;
            }
            let pending = migration(110)?;
            for (name, selected, progress) in [
                ("wrong choice", vec![Some(1), Some(1)], PROGRESS),
                ("invalid actual index", vec![Some(2), Some(1)], PROGRESS),
                ("wrong before", vec![Some(0), Some(1)], PROGRESS + 1),
            ] {
                let mut lines = pending
                    .lines()
                    .iter()
                    .filter(|line| line.before().item_key() != SWORD)
                    .cloned()
                    .collect::<Vec<_>>();
                lines.push(line(
                    Migration,
                    retained(SWORD, OLD, progress, &BEFORE)?,
                    retained(SWORD, NEW, progress, &selected)?,
                )?);
                let changed = request(110, Migration, lines, None, MIGRATION_DIGEST)?;
                let expected = if progress == PROGRESS {
                    Failure::Invalid
                } else {
                    Failure::Refused(Refusal::TrackStateMismatch)
                };
                a.reject(name, a.at(8, changed)?.source(source), expected)
                    .await?;
            }
            // New content does not implicitly refresh an old track revision.
            let reselected = [Some(0), Some(1), Some(0)];
            for (cause, progress, selected, track) in [
                (Training, PROGRESS + 1, BEFORE, None),
                (PerkSelection, PROGRESS, reselected, Some(5)),
            ] {
                let changed = line(
                    cause,
                    retained(SWORD, OLD, PROGRESS, &BEFORE)?,
                    retained(SWORD, OLD, progress, &selected)?,
                )?;
                let pending = request(111, cause, vec![changed], track, MIGRATION_DIGEST)?;
                a.reject(
                    "stale definition",
                    a.at(8, pending)?.source(source),
                    Failure::Refused(Refusal::DefinitionRevisionMismatch),
                )
                .await?;
            }
            assert!(matches!(
                a.call(a.at(8, pending)?.source(source))
                    .await
                    .map_err(debug)?,
                Committed(_)
            ));
            Ok(())
        })
    }
    // HANDWRITTEN BATCH SOURCE_AWARE_RECOVERY
    // COPY-only handwritten insertion INSIDE existing explicit_migration, after its
    // helpers. Its private Source/seed/migration/load helpers remain private.
    async fn recovery_snapshot(h: &Harness) -> TestResult<(String, String)> {
        let admissions = sqlx::query_scalar("SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY recovery_generation),'[]'::jsonb)::text FROM game_character_recovery_admissions a")
            .fetch_one(&h.pool).await?;
        Ok((snapshot(h).await?, admissions))
    }

    #[test]
    fn public_source_aware_recovery_reconciles_exclusive_successor_after_migration() -> TestResult {
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        runtime()?.block_on(async move {
            let h = Harness::create(admin, "prof1_source_recovery", true).await?;
            let outcome = source_recovery_body(&h).await;
            h.cleanup().await?;
            outcome
        })
    }

    async fn source_recovery_body(h: &Harness) -> TestResult {
        use crate::character_recovery_fence::CharacterRecoveryStore;
        contention::assert_seed_context(h).await?;
        let root = &h.root;
        let source = Source::view(1);
        // End this complete borrow scope before acquiring exclusive recovery.
        {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = root.open_character_authority(&seal).await.map_err(debug)?;
            let actor = Actor {
                h,
                authority: &authority,
            };
            seed(&actor).await?;
            let result = actor
                .call(actor.at(8, migration(110)?)?.source(source))
                .await
                .map_err(debug)?;
            let Committed(receipt) = result else {
                return Err(debug(result).into());
            };
            assert_eq!(receipt.committed_character_revision.get(), 9);
            assert_eq!(load(&actor, source).await.map_err(debug)?.len(), 2);
            drop(
                root.open_character_authority_with_proficiency_definitions(
                    &seal,
                    Some(Arc::new(source)),
                )
                .await
                .map_err(debug)?,
            );
        }
        let before = recovery_snapshot(h).await?;
        assert_eq!(h.root_revision().await?, "9");
        assert_eq!(h.count("game_character_recovery_admissions").await?, 1);

        // Independent real retained predecessor from another scope. A valid
        // migration source does not replace the database's external proof.
        let parent = std::env::temp_dir().join(format!(
            "oteryn-prof1-foreign-parent-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&parent)?;
        let name = h
            .database
            .url
            .rsplit('/')
            .next()
            .ok_or("fixture database name")?;
        let directory = parent.join(name);
        std::fs::create_dir(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700))?;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
        }
        let foreign = CharacterRecoveryStore::open(&directory, "character-other", "game-ops")
            .map_err(debug)?;
        drop(foreign.authorize_fresh_store(id(170), 100).map_err(debug)?);
        let foreign_transition = foreign.begin_recovery(1, id(171), 200).map_err(debug)?;
        assert!(matches!(
            root.reconcile_character_recovery_with_proficiency_definitions(
                &foreign_transition,
                Some(Arc::new(source))
            )
            .await,
            Err(AdmissionError::Unavailable(Unavailable))
        ));
        assert_eq!(
            recovery_snapshot(h).await?,
            before,
            "foreign predecessor admitted"
        );
        drop(foreign_transition);
        drop(foreign);
        std::fs::remove_dir_all(directory)?;

        let transition = h.recovery.begin_recovery(1, id(180), 200).map_err(debug)?;
        assert_eq!(transition.record().recovery_generation, 2);
        assert_eq!(transition.record().predecessor_generation, 1);
        assert!(matches!(
            root.reconcile_character_recovery(&transition).await,
            Err(AdmissionError::Unavailable(Unavailable))
        ));
        assert_eq!(
            recovery_snapshot(h).await?,
            before,
            "no-source successor insert did not roll back"
        );
        for fault in [Fault::Missing, Fault::Map] {
            assert!(
                matches!(
                    root.reconcile_character_recovery_with_proficiency_definitions(
                        &transition,
                        Some(Arc::new(source.fault(fault)))
                    )
                    .await,
                    Err(AdmissionError::Unavailable(Unavailable))
                ),
                "{fault:?}"
            );
            assert_eq!(
                recovery_snapshot(h).await?,
                before,
                "{fault:?}: successor insert did not roll back"
            );
        }
        root.reconcile_character_recovery_with_proficiency_definitions(
            &transition,
            Some(Arc::new(source)),
        )
        .await
        .map_err(debug)?;
        let admitted = recovery_snapshot(h).await?;
        assert_eq!(admitted.0, before.0, "recovery changed roots or WP history");
        assert_ne!(admitted.1, before.1);
        let generations: Vec<String> = sqlx::query_scalar("SELECT recovery_generation::text FROM game_character_recovery_admissions ORDER BY recovery_generation")
            .fetch_all(&h.pool).await?;
        assert_eq!(generations, ["1", "2"]);
        root.reconcile_character_recovery_with_proficiency_definitions(
            &transition,
            Some(Arc::new(source)),
        )
        .await
        .map_err(debug)?;
        assert_eq!(
            recovery_snapshot(h).await?,
            admitted,
            "idempotent recovery changed admission"
        );
        drop(transition);

        let seal = h.recovery.seal_current().map_err(debug)?;
        assert_eq!(seal.record().recovery_generation, 2);
        assert_eq!(seal.record().recovery_event_id, id(180));
        let authority = root
            .open_character_authority_with_proficiency_definitions(&seal, Some(Arc::new(source)))
            .await
            .map_err(debug)?;
        let actor = Actor {
            h,
            authority: &authority,
        };
        let tracks = load(&actor, source).await.map_err(debug)?;
        assert_eq!(tracks.len(), 2);
        for item in [SWORD, AXE] {
            let track = tracks
                .iter()
                .find(|t| t.state.item_key() == item)
                .ok_or("missing recovered track")?;
            assert_eq!(track.state, retained(item, NEW, PROGRESS, &choices(item))?);
            assert_eq!(track.committed_character_revision.get(), 9);
        }
        assert_eq!(h.root_revision().await?, "9");
        assert_eq!(
            recovery_snapshot(h).await?,
            admitted,
            "sealed generation-two read changed durable state"
        );
        Ok(())
    }
}

// HANDWRITTEN BATCH VERIFIED_BINDING_REGRESSION
// COPY-only handwritten regression; append this inline module to the approved
// canonical support. Uses its actual Actor/Harness; no extra fixture or authority.
mod retained_binding_integrity {
    use super::*;
    use crate::durability::DurabilityError;
    use crate::durability::character_authority::CharacterAuthorityError;

    async fn write_header_bytes(
        h: &Harness,
        occurrence: u8,
        column: &str,
        bytes: &[u8],
    ) -> TestResult {
        assert!(matches!(column, "command_binding" | "policy_digest"));
        let mut tx = h.pool.begin().await?;
        sqlx::query("ALTER TABLE game_character_proficiency_receipts DISABLE TRIGGER USER")
            .execute(&mut *tx)
            .await?;
        let sql = format!(
            "UPDATE game_character_proficiency_receipts SET {column}=$2 WHERE proficiency_occurrence_id=encode($1,'hex')::uuid"
        );
        let changed = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id(occurrence).as_slice())
            .bind(bytes)
            .execute(&mut *tx)
            .await?;
        assert_eq!(changed.rows_affected(), 1);
        sqlx::query("ALTER TABLE game_character_proficiency_receipts ENABLE TRIGGER USER")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    #[test]
    fn public_training_and_selection_retained_bindings_reject_check_valid_corruption() -> TestResult
    {
        run("prof1_binding_integrity", true, async |a| {
            a.commit(1, two_tracks(0)?).await?;
            a.commit(2, selection(62, UNLOCK, [Some(2), None, None], 2)?)
                .await?;
            // Axe's predecessor is track revision 2 while the current global revision is 3.
            a.commit(
                3,
                request(
                    63,
                    Cause::PerkSelection,
                    vec![line(
                        Cause::PerkSelection,
                        state(AXE, UNLOCK, NULLS)?,
                        state(AXE, UNLOCK, [Some(1), None, None])?,
                    )?],
                    Some(2),
                    DIGEST,
                )?,
            )
            .await?;
            let root = &a.h.root;
            let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
            let seal = a.h.recovery.seal_current().map_err(debug)?;
            let baseline = snapshot(a.h).await?;
            let mut failures = Vec::new();
            for (cause, occurrence) in [("training", 61), ("perk_selection", 62)] {
                for (name, column, index) in [
                    ("binding hash", "command_binding", 32),
                    ("binding version", "command_binding", 0),
                    ("policy digest", "policy_digest", 31),
                ] {
                    // Mandatory positive read/admission precedes each independent one-byte fault.
                    let tracks = root
                        .read_character_proficiency_state(a.authority, character)
                        .await
                        .map_err(debug)?;
                    assert_eq!(tracks.len(), 2);
                    for (item, choices, revision) in [
                        (SWORD, [Some(2), None, None], 3),
                        (AXE, [Some(1), None, None], 4),
                    ] {
                        let track = tracks
                            .iter()
                            .find(|t| t.state.item_key() == item)
                            .ok_or("missing control track")?;
                        assert_eq!(track.state, state(item, UNLOCK, choices)?);
                        assert_eq!(track.committed_character_revision.get(), revision);
                    }
                    drop(root.open_character_authority(&seal).await.map_err(debug)?);
                    assert_eq!(snapshot(a.h).await?, baseline);
                    let sql = format!(
                        "SELECT {column} FROM game_character_proficiency_receipts WHERE proficiency_occurrence_id=encode($1,'hex')::uuid AND cause=$2"
                    );
                    let before: Vec<u8> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
                        .bind(id(occurrence).as_slice())
                        .bind(cause)
                        .fetch_one(&a.h.pool)
                        .await?;
                    assert_eq!(
                        before.len(),
                        if column == "command_binding" { 33 } else { 32 }
                    );
                    let mut changed = before.clone();
                    if index == 0 {
                        assert_eq!(before[0], 1);
                        changed[0] = 2;
                    } else {
                        changed[index] = changed[index].wrapping_add(1);
                    }
                    write_header_bytes(a.h, occurrence, column, &changed).await?;
                    let corrupted = snapshot(a.h).await?;
                    assert_ne!(corrupted, baseline);
                    let read = root
                        .read_character_proficiency_state(a.authority, character)
                        .await;
                    if !matches!(
                        read,
                        Err(Error::Unavailable(DurabilityError::InvalidStoredState))
                    ) {
                        failures.push(format!(
                            "{cause}/{name}: read did not return InvalidStoredState"
                        ));
                    }
                    assert_eq!(
                        snapshot(a.h).await?,
                        corrupted,
                        "{cause}/{name}: read mutated history"
                    );
                    let open = root.open_character_authority(&seal).await;
                    if !matches!(
                        open,
                        Err(CharacterAuthorityError::Unavailable(
                            DurabilityError::Unavailable
                        ))
                    ) {
                        failures.push(format!(
                            "{cause}/{name}: open did not return Unavailable(Unavailable)"
                        ));
                    }
                    assert_eq!(
                        snapshot(a.h).await?,
                        corrupted,
                        "{cause}/{name}: open mutated history"
                    );
                    write_header_bytes(a.h, occurrence, column, &before).await?;
                    assert_eq!(
                        snapshot(a.h).await?,
                        baseline,
                        "{cause}/{name}: byte-exact restore"
                    );
                }
            }
            assert!(
                failures.is_empty(),
                "all six cases ran and restored their baseline: {failures:?}"
            );
            assert_eq!(a.h.root_revision().await?, "4");
            Ok(())
        })
    }
}

/// CHAR-REV-SEQ-1: a proficiency change runs in the Character's revision slot. A write that
/// bypassed the sequencer makes the next one fail closed with nothing written (the binding
/// includes the revision, so it is not retried); the next request reloads the cursor.
#[test]
fn a_sequenced_proficiency_change_fails_closed_after_a_bypass_writer() -> TestResult {
    run("prof_sequenced", true, async |a| {
        let (root, node) = (&a.h.root, &a.h.node);
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer
            .acquire(CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?)
            .await;
        let definitions =
            || -> Arc<dyn ProficiencyDefinitions> { Arc::new(Definitions::default()) };
        // The caller's fence revision is replaced by the slot's cursor.
        let outcome = slot
            .commit_proficiency(
                root,
                a.authority,
                node,
                fence(1)?,
                training(70, SWORD, 0, UNLOCK)?,
                definitions(),
                None,
            )
            .await
            .map_err(debug)?;
        let Outcome::Committed(receipt) = outcome else {
            return Err(format!("unexpected outcome: {outcome:?}").into());
        };
        assert_eq!(receipt.committed_character_revision.get(), 2);

        // r3 an axe award that bypassed the sequencer.
        a.commit(2, training(71, AXE, 0, UNLOCK)?).await?;
        let before = snapshot(a.h).await?;
        let outcome = slot
            .commit_proficiency(
                root,
                a.authority,
                node,
                fence(1)?,
                training(72, SWORD, UNLOCK, UNLOCK + 10)?,
                definitions(),
                None,
            )
            .await;
        assert!(
            matches!(outcome, Err(Error::CharacterRevisionMismatch)),
            "{outcome:?}"
        );
        assert_eq!(snapshot(a.h).await?, before, "no retry committed");

        let outcome = slot
            .commit_proficiency(
                root,
                a.authority,
                node,
                fence(1)?,
                training(73, SWORD, UNLOCK, UNLOCK + 10)?,
                definitions(),
                None,
            )
            .await
            .map_err(debug)?;
        let Outcome::Committed(receipt) = outcome else {
            return Err(format!("unexpected outcome: {outcome:?}").into());
        };
        assert_eq!(receipt.original_character_revision.get(), 3);
        Ok(())
    })
}
