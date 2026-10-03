#![allow(clippy::expect_used)]
// PROF-SHAPE-1a PostgreSQL cases (PROFICIENCY-1B §13 rows 1-7, 9-11, 13, 14 as far as this
// slice reaches; the dust and orb shapes wait for PROF-SHAPE-1b).
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::weapon_proficiency::{
    ProficiencyModificationCommandKind as Kind, ProficiencyModificationOperation as Operation,
    ProficiencyModificationResult as R, ProficiencySelectionShape, ProficiencyShapingEntry,
    ProficiencyShapingRevision, ProficiencyThresholdClass,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_proficiency::{
    DeclaredProficiencyMigration, DurableProficiencyState as State, ProficiencyCause as Cause,
    ProficiencyChangeRequest as Request, ProficiencyCommitOutcome, ProficiencyDefinitions,
    ProficiencyLevelMigration, ProficiencyLineCandidate as Line, ProficiencyOccurrence,
    ProficiencyWriteRefusal, ResolvedProficiencyDefinition,
};
use crate::durability::character_proficiency_modification::{
    ProficiencyModificationBoundRevisions as Bound, ProficiencyModificationCommand as Command,
    ProficiencyModificationContext as Context, ProficiencyModificationOutcome as Outcome,
    ProficiencyShapingSource,
};
use crate::durability::character_progression::CharacterProgressionError as Error;
use std::sync::Arc;

const SWORD: &str = "oteryn:item.tibia.i3295";
const AXE: &str = "oteryn:item.tibia.i3200";
const SWORD_DEF: &str = "oteryn:proficiency.tibia.p1";
const AXE_DEF: &str = "oteryn:proficiency.tibia.p2";
const SHAPING: &str = "oteryn:proficiency-shaping.tibia.p1";
const LEVEL_THREE: u64 = 100_000;
const DIGEST: [u8; 32] = [1; 32];
const IN_ZONE: Context = Context {
    in_protection_zone: true,
    server_seed: [9; 32],
};

fn shape() -> ProficiencySelectionShape {
    ProficiencySelectionShape::new(ProficiencyThresholdClass::Standard, &[3, 2, 1])
        .expect("fixture shape")
}

#[derive(Clone, Copy)]
struct Definitions {
    revision: &'static str,
}
impl Default for Definitions {
    fn default() -> Self {
        Self {
            revision: "definition-1",
        }
    }
}
impl ProficiencyDefinitions for Definitions {
    fn content_revision(&self) -> &str {
        "content-1"
    }
    fn digest(&self) -> [u8; 32] {
        DIGEST
    }
    fn active_weapon_count(&self) -> usize {
        2
    }
    fn resolve_weapon(&self, item: &str) -> Option<ResolvedProficiencyDefinition> {
        let definition = match item {
            SWORD => SWORD_DEF,
            AXE => AXE_DEF,
            _ => return None,
        };
        Some(ResolvedProficiencyDefinition {
            canonical_item_key: item.into(),
            definition_key: definition.into(),
            definition_revision: self.revision.into(),
            shape: shape(),
        })
    }
    fn retained_migration(
        &self,
        _content_revision: &str,
        _digest: &[u8; 32],
        _item_key: &str,
        _definition_key: &str,
        old_revision: &str,
        new_revision: &str,
    ) -> Option<DeclaredProficiencyMigration> {
        ((old_revision, new_revision) == ("definition-1", "definition-2")).then(|| {
            DeclaredProficiencyMigration {
                old_shape: shape(),
                new_shape: shape(),
                levels: vec![
                    ProficiencyLevelMigration::Clear,
                    ProficiencyLevelMigration::Keep,
                    ProficiencyLevelMigration::Keep,
                ],
            }
        })
    }
    fn retained_definition(
        &self,
        _item: &str,
        _definition: &str,
        revision: &str,
    ) -> Option<ProficiencySelectionShape> {
        matches!(revision, "definition-1" | "definition-2").then(shape)
    }
}

#[derive(Clone)]
struct Shaping {
    revision: &'static str,
    entries: usize,
    clear_dust: u64,
    pool_known: bool,
}
impl Default for Shaping {
    fn default() -> Self {
        Self {
            revision: "shaping-1",
            entries: 5,
            clear_dust: 0,
            pool_known: true,
        }
    }
}
impl ProficiencyShapingSource for Shaping {
    fn active(&self, key: &str) -> Option<ProficiencyShapingRevision> {
        (key == SHAPING).then(|| ProficiencyShapingRevision {
            shaping_key: SHAPING.into(),
            revision: self.revision.into(),
            pool: self.pool_known.then(|| {
                (0..self.entries)
                    .map(|index| ProficiencyShapingEntry {
                        perk_identity: format!("perk-{index}"),
                        weight: Some(1),
                        rank_values: [true; 10],
                    })
                    .collect()
            }),
            modify_cost: [Some(0); 2],
            rank_step_cost: [Some(0); 9],
            reshape_offer_cost: Some(0),
            clear_cost: Some(self.clear_dust),
            orb_count: Some(0),
        })
    }
    fn perk_identity(
        &self,
        definition: &str,
        _revision: &str,
        _level: u8,
        index: u8,
    ) -> Option<String> {
        (definition == SWORD_DEF).then(|| format!("perk-{index}"))
    }
}

fn revision(value: u64) -> CharacterRevision {
    CharacterRevision::new(value).expect("revision")
}

fn occurrence(tag: u8) -> ProficiencyOccurrence {
    ProficiencyOccurrence::from_bytes(id(tag)).expect("occurrence")
}

fn command(tag: u8, slot: u8, kind: Kind, track: u64, shaping: &str) -> Command {
    Command::new(
        occurrence(tag),
        SWORD,
        slot,
        kind,
        revision(track),
        Bound {
            definition_revision: "definition-1".into(),
            shaping_revision: shaping.into(),
            simulation_revision: "simulation-1".into(),
        },
    )
    .expect("command")
}

fn state(progress: u64, choices: [Option<u8>; 3], definition: &str) -> State {
    State::new(SWORD, SWORD_DEF, definition, progress, choices.to_vec()).expect("state")
}

struct Actor<'h, 'f, 's> {
    h: &'h Harness,
    authority: &'h ReconciledCharacterAuthority<'f, 's>,
}

impl Actor<'_, '_, '_> {
    async fn modify_with(
        &self,
        at: u64,
        command: Command,
        context: Context,
        shaping: Shaping,
    ) -> Result<Outcome, Error> {
        self.h
            .root
            .commit_character_proficiency_modification(
                self.authority,
                &self.h.node,
                fence(at).map_err(|_| Error::InvalidInput)?,
                command,
                context,
                Arc::new(Definitions::default()),
                Arc::new(shaping),
            )
            .await
    }

    async fn modify(&self, at: u64, command: Command) -> Result<Outcome, Error> {
        self.modify_with(at, command, IN_ZONE, Shaping::default())
            .await
    }

    async fn accept(&self, at: u64, command: Command) -> TestResult<Outcome> {
        let outcome = self.modify(at, command).await.map_err(debug)?;
        assert!(matches!(outcome, Outcome::Committed(_)), "{outcome:?}");
        Ok(outcome)
    }

    async fn refuse(
        &self,
        name: &str,
        at: u64,
        command: Command,
        shaping: Shaping,
        context: Context,
        wanted: R,
    ) -> TestResult {
        let before = snapshot(self.h).await?;
        let outcome = self
            .modify_with(at, command, context, shaping)
            .await
            .map_err(debug)?;
        assert_eq!(outcome, Outcome::Refused(wanted), "{name}");
        assert_eq!(outcome.result(), wanted, "{name}");
        assert_eq!(
            snapshot(self.h).await?,
            before,
            "{name}: durability changed"
        );
        Ok(())
    }

    async fn proficiency(
        &self,
        at: u64,
        request: Request,
        definitions: Definitions,
    ) -> TestResult<ProficiencyCommitOutcome> {
        self.h
            .root
            .commit_character_proficiency(
                self.authority,
                &self.h.node,
                fence(at)?,
                request,
                Arc::new(definitions),
            )
            .await
            .map_err(|error| debug(error).into())
    }

    async fn row(
        &self,
        slot: i16,
    ) -> TestResult<
        Option<(
            Option<i16>,
            Option<i16>,
            Option<i16>,
            Option<Vec<i16>>,
            Option<String>,
        )>,
    > {
        Ok(sqlx::query_as(
            "SELECT level, entry_index, rank, pending_offer, shaping_revision \
             FROM game_character_proficiency_modifications WHERE item_key=$1 AND slot=$2",
        )
        .bind(SWORD)
        .bind(slot)
        .fetch_optional(&self.h.pool)
        .await?)
    }

    async fn track_revision(&self) -> TestResult<u64> {
        let text: String = sqlx::query_scalar(
            "SELECT committed_character_revision::text FROM game_character_proficiency WHERE item_key=$1",
        )
        .bind(SWORD)
        .fetch_one(&self.h.pool)
        .await?;
        Ok(text.parse()?)
    }

    /// Reopening the authority runs `verify_character_integrity` over every history.
    async fn verify(&self) -> TestResult {
        let seal = self.h.recovery.seal_current().map_err(debug)?;
        self.h
            .root
            .open_character_authority(&seal)
            .await
            .map(drop)
            .map_err(|error| debug(error).into())
    }
}

async fn snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object( \
        'root',(SELECT jsonb_agg(to_jsonb(r) ORDER BY character_id) FROM game_character_roots r), \
        'headers',(SELECT jsonb_agg(to_jsonb(h) ORDER BY proficiency_occurrence_id) FROM game_character_proficiency_receipts h), \
        'tracks',(SELECT jsonb_agg(to_jsonb(t) ORDER BY item_key) FROM game_character_proficiency t), \
        'rows',(SELECT jsonb_agg(to_jsonb(m) ORDER BY item_key, slot) FROM game_character_proficiency_modifications m), \
        'lines',(SELECT jsonb_agg(to_jsonb(l) ORDER BY proficiency_occurrence_id) FROM game_character_proficiency_modification_lines l), \
        'terminals',(SELECT jsonb_agg(to_jsonb(t) ORDER BY proficiency_occurrence_id) FROM game_character_proficiency_modification_terminals t))::text",
    )
    .fetch_one(&h.pool)
    .await?)
}

async fn activate(h: &Harness, revision: &str, retained: &[&str]) -> Result<(), sqlx::Error> {
    let retained: Vec<String> = retained.iter().map(|value| (*value).to_owned()).collect();
    sqlx::query("SELECT game_proficiency_shaping_activate($1, $2, $3)")
        .bind(SHAPING)
        .bind(revision)
        .bind(&retained)
        .execute(&h.pool)
        .await
        .map(drop)
}

/// Global revision 3: Sword trained to level 3 (revision 2) with perk 0 selected at level 0
/// (revision 3, the track's committed revision); shaping-1 active and retained.
async fn prepare(a: &Actor<'_, '_, '_>) -> TestResult {
    let nulls = [None; 3];
    let training = Request::new(
        occurrence(1),
        Cause::Training,
        vec![
            Line::new(
                Cause::Training,
                state(0, nulls, "definition-1"),
                state(LEVEL_THREE, nulls, "definition-1"),
            )
            .map_err(debug)?,
        ],
        None,
        DIGEST,
    )
    .map_err(debug)?;
    assert!(matches!(
        a.proficiency(1, training, Definitions::default()).await?,
        ProficiencyCommitOutcome::Committed(_)
    ));
    let selection = selection(2, [None; 3], [Some(0), None, None], 2)?;
    assert!(matches!(
        a.proficiency(2, selection, Definitions::default()).await?,
        ProficiencyCommitOutcome::Committed(_)
    ));
    activate(a.h, "shaping-1", &[]).await?;
    Ok(())
}

fn selection(
    tag: u8,
    before: [Option<u8>; 3],
    after: [Option<u8>; 3],
    track: u64,
) -> TestResult<Request> {
    Request::new(
        occurrence(tag),
        Cause::PerkSelection,
        vec![
            Line::new(
                Cause::PerkSelection,
                state(LEVEL_THREE, before, "definition-1"),
                state(LEVEL_THREE, after, "definition-1"),
            )
            .map_err(debug)?,
        ],
        Some(revision(track)),
        DIGEST,
    )
    .map_err(|error| debug(error).into())
}

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Actor<'_, '_, '_>) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, true).await?;
        let result = {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let actor = Actor {
                h: &h,
                authority: &authority,
            };
            match prepare(&actor).await {
                Ok(()) => body(&actor).await,
                Err(error) => Err(error),
            }
        };
        h.cleanup().await?;
        result
    })
}

#[test]
fn modification_lifecycle_replays_and_verifies() -> TestResult {
    run("profshape_lifecycle", async |a| {
        let modify = command(20, 1, Kind::Modify { level: 0 }, 3, "shaping-1");
        let Outcome::Committed(receipt) = a.accept(3, modify.clone()).await? else {
            unreachable!()
        };
        assert_eq!(
            (
                receipt.original_character_revision.get(),
                receipt.committed_character_revision.get(),
                receipt.operation
            ),
            (3, 4, Operation::Modify)
        );
        let after = receipt.after.clone().expect("modified");
        assert_ne!(after.entry_index, 0, "the selected perk is never drawn");
        assert_eq!((after.level, after.rank, after.pending_offer), (0, 1, None));
        assert_eq!(a.h.root_revision().await?, "4");
        assert_eq!(a.track_revision().await?, 4);
        assert_eq!(
            a.h.count("game_character_proficiency_modification_lines")
                .await?,
            1
        );
        // §6.1: a retry returns the first receipt, even under an exhausted cap or older fence.
        let unchanged = snapshot(a.h).await?;
        assert_eq!(
            a.modify(4, modify.clone()).await.map_err(debug)?,
            Outcome::AlreadyCommitted(receipt.clone())
        );
        assert_eq!(
            a.h.root
                .reconcile_character_proficiency_modification(
                    a.authority,
                    CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
                    modify.clone()
                )
                .await
                .map_err(debug)?,
            Some(Outcome::AlreadyCommitted(receipt.clone()))
        );
        let conflicting = command(20, 2, Kind::Modify { level: 0 }, 3, "shaping-1");
        assert!(matches!(
            a.modify(4, conflicting).await,
            Err(Error::ConflictingOccurrence)
        ));
        assert_eq!(snapshot(a.h).await?, unchanged);
        // RANK_UP, offer, rank refused while pending, choose, offer, decline, clear.
        a.accept(4, command(21, 1, Kind::RankUp, 4, "shaping-1"))
            .await?;
        assert_eq!(a.row(1).await?.and_then(|row| row.2), Some(2));
        a.accept(5, command(22, 1, Kind::ReshapeOffer, 5, "shaping-1"))
            .await?;
        let offer = a
            .row(1)
            .await?
            .and_then(|row| row.3)
            .expect("pending offer");
        assert_eq!(offer.len(), 3);
        assert!(!offer.contains(&i16::from(after.entry_index)));
        a.refuse(
            "rank while offered",
            6,
            command(23, 1, Kind::RankUp, 6, "shaping-1"),
            Shaping::default(),
            IN_ZONE,
            R::OfferPending,
        )
        .await?;
        a.refuse(
            "second offer",
            6,
            command(23, 1, Kind::ReshapeOffer, 6, "shaping-1"),
            Shaping::default(),
            IN_ZONE,
            R::OfferPending,
        )
        .await?;
        a.accept(
            6,
            command(
                24,
                1,
                Kind::ReshapeChoose { choice: Some(1) },
                6,
                "shaping-1",
            ),
        )
        .await?;
        let chosen = a.row(1).await?.expect("row");
        assert_eq!(
            (chosen.1, chosen.2, chosen.3),
            (Some(offer[1]), Some(2), None)
        );
        a.accept(7, command(25, 1, Kind::ReshapeOffer, 7, "shaping-1"))
            .await?;
        let Outcome::Committed(declined) = a
            .accept(
                8,
                command(26, 1, Kind::ReshapeChoose { choice: None }, 8, "shaping-1"),
            )
            .await?
        else {
            unreachable!()
        };
        assert_eq!(declined.operation, Operation::ReshapeDecline);
        assert_eq!(
            a.row(1).await?.map(|row| (row.1, row.3)),
            Some((Some(offer[1]), None))
        );
        a.accept(9, command(27, 1, Kind::Clear, 9, "shaping-1"))
            .await?;
        assert_eq!(
            a.row(1).await?.map(|row| (row.0, row.4)),
            Some((None, None)),
            "rows are never deleted"
        );
        a.accept(
            10,
            command(28, 1, Kind::Modify { level: 0 }, 10, "shaping-1"),
        )
        .await?;
        assert_eq!(
            a.h.count("game_character_proficiency_modifications")
                .await?,
            1
        );
        assert_eq!(
            a.h.count("game_character_proficiency_modification_lines")
                .await?,
            8
        );
        a.verify().await
    })
}

#[test]
fn refusals_follow_the_order_and_write_nothing() -> TestResult {
    run("profshape_refusals", async |a| {
        let modify = |tag, slot, level| command(tag, slot, Kind::Modify { level }, 3, "shaping-1");
        let unknown = Shaping {
            pool_known: false,
            ..Shaping::default()
        };
        a.refuse(
            "unevidenced pool",
            3,
            modify(30, 1, 0),
            unknown,
            IN_ZONE,
            R::NotAdmitted,
        )
        .await?;
        let orb = command(30, 1, Kind::OrbRank, 3, "shaping-1");
        a.refuse(
            "orb before 1b",
            3,
            orb,
            Shaping::default(),
            IN_ZONE,
            R::NotAdmitted,
        )
        .await?;
        a.refuse(
            "stale track revision",
            3,
            command(30, 1, Kind::Modify { level: 0 }, 2, "shaping-1"),
            Shaping::default(),
            IN_ZONE,
            R::StaleRevision,
        )
        .await?;
        let outside = Context {
            in_protection_zone: false,
            ..IN_ZONE
        };
        a.refuse(
            "outside a protection zone",
            3,
            modify(30, 1, 0),
            Shaping::default(),
            outside,
            R::NotInProtectionZone,
        )
        .await?;
        a.refuse(
            "slot 2 before Mastery",
            3,
            modify(30, 2, 0),
            Shaping::default(),
            IN_ZONE,
            R::NotUnlocked,
        )
        .await?;
        a.refuse(
            "level without a selection",
            3,
            modify(30, 1, 1),
            Shaping::default(),
            IN_ZONE,
            R::NoSelection,
        )
        .await?;
        let lonely = Shaping {
            entries: 1,
            ..Shaping::default()
        };
        a.refuse(
            "only the selected perk",
            3,
            modify(30, 1, 0),
            lonely,
            IN_ZONE,
            R::PoolTooSmall,
        )
        .await?;
        a.refuse(
            "nothing to clear",
            3,
            command(30, 1, Kind::Clear, 3, "shaping-1"),
            Shaping::default(),
            IN_ZONE,
            R::NotModified,
        )
        .await?;
        let costly = Shaping {
            clear_dust: 1_000,
            ..Shaping::default()
        };
        a.refuse(
            "a dust cost before the ledger",
            3,
            command(30, 1, Kind::Clear, 3, "shaping-1"),
            costly,
            IN_ZONE,
            R::NotAdmitted,
        )
        .await?;
        // A pool with fewer than 3 entries other than the current one refuses before any write
        // (the 3-entry case is a domain test).
        a.accept(3, modify(31, 1, 0)).await?;
        let small = Shaping {
            entries: 2,
            ..Shaping::default()
        };
        a.refuse(
            "offer from a small pool",
            4,
            command(32, 1, Kind::ReshapeOffer, 4, "shaping-1"),
            small,
            IN_ZONE,
            R::PoolTooSmall,
        )
        .await?;
        a.refuse(
            "slot already modified",
            4,
            command(32, 1, Kind::Modify { level: 0 }, 4, "shaping-1"),
            Shaping::default(),
            IN_ZONE,
            R::SlotOccupied,
        )
        .await?;
        a.verify().await
    })
}

#[test]
fn a_moved_revision_set_is_terminal_and_outside_the_chain() -> TestResult {
    run("profshape_terminal", async |a| {
        let stale = command(40, 1, Kind::Modify { level: 0 }, 3, "shaping-0");
        let unchanged_root = a.h.root_revision().await?;
        assert_eq!(
            a.modify(3, stale.clone()).await.map_err(debug)?,
            Outcome::RevisionChanged
        );
        assert_eq!(
            a.h.root_revision().await?,
            unchanged_root,
            "no CharacterRevision"
        );
        assert_eq!(
            a.h.count("game_character_proficiency_modification_terminals")
                .await?,
            1
        );
        assert_eq!(
            a.h.count("game_character_proficiency_modification_lines")
                .await?,
            0
        );
        // A replay stays refused even once the set matches again, and under any fence revision.
        let matching = Shaping {
            revision: "shaping-0",
            ..Shaping::default()
        };
        activate(a.h, "shaping-0", &["shaping-1"]).await?;
        assert_eq!(
            a.modify_with(3, stale.clone(), IN_ZONE, matching)
                .await
                .map_err(debug)?,
            Outcome::RevisionChanged
        );
        assert_eq!(
            a.h.root
                .reconcile_character_proficiency_modification(
                    a.authority,
                    CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
                    stale
                )
                .await
                .map_err(debug)?,
            Some(Outcome::RevisionChanged)
        );
        assert_eq!(
            a.h.count("game_character_proficiency_modification_terminals")
                .await?,
            1
        );
        // A bound member that no longer resolves (the axe has no shaping content) has moved
        // too: the occurrence is terminal, so reappearing content can never commit it.
        let axe = Command::new(
            occurrence(41),
            AXE,
            1,
            Kind::Clear,
            revision(3),
            Bound {
                definition_revision: "definition-1".into(),
                shaping_revision: "shaping-1".into(),
                simulation_revision: "simulation-1".into(),
            },
        )
        .map_err(debug)?;
        assert_eq!(
            a.modify(3, axe.clone()).await.map_err(debug)?,
            Outcome::RevisionChanged
        );
        assert_eq!(
            a.modify(3, axe).await.map_err(debug)?,
            Outcome::RevisionChanged
        );
        assert_eq!(
            a.h.count("game_character_proficiency_modification_terminals")
                .await?,
            2
        );
        a.verify().await?;
        // verify_character_integrity recomputes a standalone terminal's binding from its own
        // fields: one whose slot disagrees with its binding fails.
        let mut connection = a.h.pool.acquire().await?;
        for sql in [
            "SET session_replication_role = replica",
            "UPDATE game_character_proficiency_modification_terminals SET slot = 2 \
             WHERE operation = 'MODIFY'",
            "SET session_replication_role = DEFAULT",
        ] {
            sqlx::query(sql).execute(&mut *connection).await?;
        }
        drop(connection);
        assert!(
            a.verify().await.is_err(),
            "a terminal unlike its binding verifies"
        );
        Ok(())
    })
}

#[test]
fn retention_serializes_with_activation() -> TestResult {
    run("profshape_retention", async |a| {
        a.accept(3, command(50, 1, Kind::Modify { level: 0 }, 3, "shaping-1"))
            .await?;
        let refused = activate(a.h, "shaping-2", &[])
            .await
            .expect_err("drops a referenced revision");
        assert!(
            matches!(&refused, sqlx::Error::Database(error) if error.code().as_deref() == Some("OTC01")),
            "{refused:?}"
        );
        let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM game_proficiency_shaping_revisions WHERE shaping_revision='shaping-1' AND active")
            .fetch_one(&a.h.pool)
            .await?;
        assert_eq!(retained, 1, "a refused activation changes nothing");
        // The control-plane entry point answers the same refusal as Ok(false).
        let root = &a.h.root;
        assert!(
            !root
                .record_proficiency_shaping_activation(SHAPING, "shaping-2", &[])
                .await
                .map_err(debug)?
        );
        let keep = ["shaping-1".to_owned()];
        assert!(
            root.record_proficiency_shaping_activation(SHAPING, "shaping-2", &keep)
                .await
                .map_err(debug)?
        );
        let newer = Shaping {
            revision: "shaping-2",
            ..Shaping::default()
        };
        a.refuse(
            "rank on an older revision",
            4,
            command(51, 1, Kind::RankUp, 4, "shaping-2"),
            newer.clone(),
            IN_ZONE,
            R::ShapingOutdated,
        )
        .await?;
        a.refuse(
            "offer on an older revision",
            4,
            command(51, 1, Kind::ReshapeOffer, 4, "shaping-2"),
            newer.clone(),
            IN_ZONE,
            R::ShapingOutdated,
        )
        .await?;
        let clear = a
            .modify_with(
                4,
                command(52, 1, Kind::Clear, 4, "shaping-2"),
                IN_ZONE,
                newer,
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(clear, Outcome::Committed(_)),
            "CLEAR stays open: {clear:?}"
        );
        // The cleared row no longer holds shaping-1; dropping it leaves every check verifiable.
        activate(a.h, "shaping-2", &[]).await?;
        a.verify().await?;
        // A write that references an unretained revision is refused by the database.
        let mut tx = a.h.pool.begin().await?;
        let error = sqlx::query("UPDATE game_character_proficiency_modifications SET level=0, shaping_key=$1, shaping_revision='shaping-1', entry_index=1, rank=1")
            .bind(SHAPING)
            .execute(&mut *tx)
            .await
            .expect_err("unretained revision");
        assert!(
            format!("{error:?}").contains("unretained shaping revision"),
            "{error:?}"
        );
        Ok(())
    })
}

#[test]
fn modified_levels_refuse_selection_changes_and_migrations_clear_them() -> TestResult {
    run("profshape_levels", async |a| {
        a.accept(3, command(60, 1, Kind::Modify { level: 0 }, 3, "shaping-1"))
            .await?;
        let change = selection(61, [Some(0), None, None], [Some(1), None, None], 4)?;
        assert_eq!(
            a.proficiency(4, change, Definitions::default()).await?,
            ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::ModifiedLevel)
        );
        // Selecting at an unmodified level is unaffected.
        let other = selection(62, [Some(0), None, None], [Some(0), Some(1), None], 4)?;
        assert!(matches!(
            a.proficiency(4, other, Definitions::default()).await?,
            ProficiencyCommitOutcome::Committed(_)
        ));
        // An incompatible migration that clears level 0 clears the modification with no refund.
        let migration = Request::new(
            occurrence(63),
            Cause::Migration,
            vec![
                Line::new(
                    Cause::Migration,
                    state(LEVEL_THREE, [Some(0), Some(1), None], "definition-1"),
                    state(LEVEL_THREE, [None, Some(1), None], "definition-2"),
                )
                .map_err(debug)?,
            ],
            None,
            DIGEST,
        )
        .map_err(debug)?;
        let migrated = Definitions {
            revision: "definition-2",
        };
        assert!(matches!(
            a.proficiency(5, migration, migrated).await?,
            ProficiencyCommitOutcome::Committed(_)
        ));
        assert_eq!(a.row(1).await?.map(|row| row.0), Some(None));
        let (operation, dust): (String, i64) = sqlx::query_as(
            "SELECT operation, dust_spent FROM game_character_proficiency_modification_lines ORDER BY committed_character_revision DESC LIMIT 1",
        )
        .fetch_one(&a.h.pool)
        .await?;
        assert_eq!((operation.as_str(), dust), ("MIGRATION_CLEAR", 0));
        Ok(())
    })
}

#[test]
fn a_losing_writer_retries_under_the_same_occurrence_and_pays_once() -> TestResult {
    run("profshape_losing", async |a| {
        let modify = command(70, 1, Kind::Modify { level: 0 }, 3, "shaping-1");
        let before = snapshot(a.h).await?;
        assert!(matches!(
            a.modify(2, modify.clone()).await,
            Err(Error::CharacterRevisionMismatch)
        ));
        assert_eq!(snapshot(a.h).await?, before, "a stale commit wrote nothing");
        let Outcome::Committed(first) = a.accept(3, modify.clone()).await? else {
            unreachable!()
        };
        assert_eq!(
            a.modify(4, modify).await.map_err(debug)?,
            Outcome::AlreadyCommitted(first)
        );
        assert_eq!(
            a.h.count("game_character_proficiency_modification_lines")
                .await?,
            1
        );
        Ok(())
    })
}

#[test]
fn database_checks_refuse_forbidden_shapes() -> TestResult {
    run("profshape_checks", async |a| {
        a.accept(3, command(80, 1, Kind::Modify { level: 0 }, 3, "shaping-1"))
            .await?;
        let refused = async |sql: &str| -> TestResult<String> {
            let mut tx = a.h.pool.begin().await?;
            let statement = sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
                .execute(&mut *tx)
                .await;
            let error = match statement {
                Err(error) => error,
                Ok(_) => tx.commit().await.expect_err(sql),
            };
            Ok(format!("{error:?}"))
        };
        assert!(
            refused("DELETE FROM game_character_proficiency_modifications")
                .await?
                .contains("cannot be deleted")
        );
        assert!(
            refused("UPDATE game_character_proficiency_modifications SET rank=5")
                .await?
                .contains("does not equal its latest line")
        );
        // Each operation changing a forbidden field, and any cost before the ledger, fails its CHECK.
        let line = "INSERT INTO game_character_proficiency_modification_lines SELECT \
            proficiency_occurrence_id, character_id, committed_character_revision, cause, item_key, 2, ";
        for (tail, name) in [
            (
                "'RANK_UP', bound_shaping_revision, level_after, shaping_key_after, shaping_revision_after, entry_index_after, rank_after, NULL, level_after, shaping_key_after, shaping_revision_after, entry_index_after + 1, rank_after + 1, NULL, 0, 0, 0, 0",
                "direction",
            ),
            (
                "'CLEAR', bound_shaping_revision, level_after, shaping_key_after, shaping_revision_after, entry_index_after, rank_after, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 1000, 1, 0, 0",
                "cost",
            ),
            (
                "'CLEAR', bound_shaping_revision, level_after, shaping_key_after, shaping_revision_after, entry_index_after, rank_after, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 1000, 1000, 0, 0",
                "line_no_ledger",
            ),
            (
                "'RESHAPE_DECLINE', bound_shaping_revision, level_after, shaping_key_after, shaping_revision_after, entry_index_after, rank_after, NULL, level_after, shaping_key_after, shaping_revision_after, entry_index_after, rank_after, NULL, 0, 0, 0, 0",
                "direction",
            ),
        ] {
            let error = refused(&format!(
                "{line}{tail} FROM game_character_proficiency_modification_lines"
            ))
            .await?;
            assert!(error.contains(name), "{name}: {error}");
        }
        // Two slots in one perk_modification receipt are refused, even with a matching row.
        let error = refused(&format!(
            "WITH line AS ({line}'MODIFY', bound_shaping_revision, NULL, NULL, NULL, NULL, NULL, NULL, \
             1, shaping_key_after, shaping_revision_after, entry_index_after, 1, NULL, 0, 0, 0, 0 \
             FROM game_character_proficiency_modification_lines RETURNING *) \
             INSERT INTO game_character_proficiency_modifications SELECT character_id, item_key, slot, \
             level_after, shaping_key_after, shaping_revision_after, entry_index_after, rank_after, NULL, \
             committed_character_revision, proficiency_occurrence_id FROM line"
        ))
        .await?;
        assert!(error.contains("one modification line"), "{error}");
        a.verify().await?;
        // verify_character_integrity fails a row that differs from its latest line, even when
        // the triggers were bypassed.
        let mut connection = a.h.pool.acquire().await?;
        for sql in [
            "SET session_replication_role = replica",
            "UPDATE game_character_proficiency_modifications SET rank = 5",
            "SET session_replication_role = DEFAULT",
        ] {
            sqlx::query(sql).execute(&mut *connection).await?;
        }
        drop(connection);
        assert!(
            a.verify().await.is_err(),
            "a row unlike its latest line verifies"
        );
        Ok(())
    })
}
