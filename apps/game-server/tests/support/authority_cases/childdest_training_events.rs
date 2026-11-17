//! CHILDDEST: Child of Destiny training events against the production binding.
//! The Party Skeleton kill (s2) credits once and advances the phase; a kill of
//! another creature, or a spear/rune hit on another creature or from another
//! source, writes no receipt and leaves the phase unchanged.
use super::engenc_combat_credit::native_hit_runtime;
use crate::bestiary_postgres_harness as fixture;
use crate::{durability, foundation};

use durability::character_progression::ExperienceRewardOccurrence;
use durability::character_revision_sequencer::CharacterRevisionSequencer;
use durability::quest_state::native_hits::NativeQuestHit;
use durability::quest_state::native_kills::{NativeDeathSource, NativeQuestDeath};
use durability::quest_state::{QuestCause, QuestTransitionOutcome};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use std::sync::Arc;

const CHILD: &str = "oteryn:quest.authored.child_of_destiny_quest";
const PHASE: &str = "oteryn:quest-progress/engenc/child/phase";
const MELEE: &str = "oteryn:quest-progress/engenc/child/melee";
const SKELETON: &str = "oteryn:creature.party_skeleton";
const SPEAR: &str = "oteryn:item.tibia.i17110";
const RUNE: &str = "oteryn:item.tibia.i17512";
const MISSILE: &str = "candidate:spell/rune/lightest_magic_missile_rune";

/// The production binding, with the phase track starting at `initial` so the
/// case begins at the lesson it proves.
fn child_catalogue(initial: i64) -> fixture::TestResult<Arc<QuestStateCatalogue>> {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/quests/authored-bindings/engenc-child-of-destiny.json"),
    )?;
    let mut bindings = Bindings::parse(&bytes)?;
    bindings.verify_embedded_recipe_evidence()?;
    bindings.quests[0]
        .tracks
        .iter_mut()
        .find(|track| track.key == PHASE)
        .ok_or("child phase")?
        .initial = initial;
    let base = QuestStateCatalogue::empty("content-1").ok_or("base")?;
    Ok(Arc::new(bindings.attach(&base)?))
}

fn run_case(
    name: &'static str,
    case: for<'a> fn(
        &'a fixture::Harness,
    )
        -> std::pin::Pin<Box<dyn std::future::Future<Output = fixture::TestResult> + 'a>>,
) -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let h = fixture::Harness::create(admin, name, true).await?;
                    let result = case(&h).await;
                    h.cleanup().await?;
                    result?;
                    eprintln!("CHILDDEST_PG_EXECUTED case={name}");
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "CHILDDEST worker panicked")?
        .map_err(Into::into)
}

async fn track(h: &fixture::Harness, key: &str) -> fixture::TestResult<Option<i64>> {
    Ok(sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
    )
    .bind(CHILD)
    .bind(key)
    .fetch_optional(&h.pool)
    .await?)
}

async fn receipts(h: &fixture::Harness, kind: &str) -> fixture::TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE cause_kind=$1",
    )
    .bind(kind)
    .fetch_one(&h.pool)
    .await?)
}

fn credited<E>(outcome: &Result<QuestTransitionOutcome, E>) -> bool {
    matches!(
        outcome,
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_))
    )
}

#[test]
fn childdest_party_skeleton_kill_credits_once_and_advances_postgres() -> fixture::TestResult {
    run_case("childdest_skeleton_kill", |h| Box::pin(skeleton_kill(h)))
}

async fn skeleton_kill(h: &fixture::Harness) -> fixture::TestResult {
    // Neill's lesson (s1) has set the phase to 1; s2 waits for the skeleton.
    let catalogue = child_catalogue(1)?;
    let hash = catalogue.definition_hash(CHILD).ok_or("child definition")?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let fence = fixture::fence(1)?;
    let observed_database_micros: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    let party = Arc::new(foundation::PartySnapshot {
        world: fixture::id(fixture::WORLD),
        character: fixture::id(fixture::CHARACTER),
        party_id: None,
        revision: None,
        leader: None,
        shared_xp_enabled: false,
        members: Vec::new(),
        observed_database_micros,
    });
    let death = |creature: &str, observed_revision: u64| NativeQuestDeath {
        transition: "oteryn:quest-transition/native-kills/engenc-child-s2".into(),
        creature: creature.into(),
        observed_revision,
        definition_hash: Some(hash),
        source: NativeDeathSource::Party(Arc::clone(&party)),
    };
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(fence.character_id).await;

    let wrong = slot
        .commit_native_quest_kill(
            &h.root,
            &authority,
            &h.node,
            fence,
            QuestCause::CreatureDeath(
                ExperienceRewardOccurrence::from_bytes(fixture::id(120))
                    .map_err(fixture::debug)?,
            ),
            &death("oteryn:creature.rat", 1),
            Arc::clone(&catalogue),
        )
        .await;
    assert!(!credited(&wrong), "another creature credited s2: {wrong:?}");
    assert_eq!(receipts(h, "creature_death").await?, 0);
    assert_eq!(track(h, MELEE).await?, None);

    let cause = QuestCause::CreatureDeath(
        ExperienceRewardOccurrence::from_bytes(fixture::id(121)).map_err(fixture::debug)?,
    );
    let skeleton = death(SKELETON, 1);
    let first = slot
        .commit_native_quest_kill(
            &h.root,
            &authority,
            &h.node,
            fence,
            cause,
            &skeleton,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(first, QuestTransitionOutcome::Committed(_)));
    let replay = slot
        .commit_native_quest_kill(
            &h.root,
            &authority,
            &h.node,
            fence,
            cause,
            &skeleton,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(
        replay,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(track(h, MELEE).await?, Some(1));
    assert_eq!(track(h, PHASE).await?, Some(2));
    assert_eq!(receipts(h, "creature_death").await?, 1);

    // A second skeleton death is a new occurrence, but the lesson is spent.
    let again = slot
        .commit_native_quest_kill(
            &h.root,
            &authority,
            &h.node,
            fence,
            QuestCause::CreatureDeath(
                ExperienceRewardOccurrence::from_bytes(fixture::id(122))
                    .map_err(fixture::debug)?,
            ),
            &death(SKELETON, 2),
            Arc::clone(&catalogue),
        )
        .await;
    assert!(!credited(&again), "spent s2 credited twice: {again:?}");
    assert_eq!(track(h, MELEE).await?, Some(1));
    assert_eq!(track(h, PHASE).await?, Some(2));
    assert_eq!(receipts(h, "creature_death").await?, 1);
    Ok(())
}

#[test]
fn childdest_hits_refuse_wrong_target_and_source_postgres() -> fixture::TestResult {
    run_case("childdest_hit_targets", |h| Box::pin(hit_targets(h)))
}

async fn hit_targets(h: &fixture::Harness) -> fixture::TestResult {
    // The skeleton lesson (s2) is complete; s3 waits for the spear.
    let catalogue = child_catalogue(2)?;
    let hash = catalogue.definition_hash(CHILD).ok_or("child definition")?;
    let fence = fixture::fence(1)?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(fence.character_id).await;
    let (mut runtime, player, target, _) = native_hit_runtime(1)?;
    let mut sequence = 0_u64;
    let mut witness = || -> fixture::TestResult<_> {
        sequence += 1;
        let command = foundation::CommandRef::new(
            fence.game_session_id,
            foundation::CommandId::new(sequence).map_err(fixture::debug)?,
        );
        let mut owner = runtime.borrow_exact_actor_commit();
        owner
            .commit_damage_for_bound_attacker(
                target,
                player,
                command,
                0,
                foundation::OwnerDamageCommand {
                    target: b"engenc:training",
                    occurrence: &[],
                    binding: b"childdest:training",
                    damage: 5,
                },
            )
            .map_err(fixture::debug)?;
        Ok(owner
            .committed_hit_for_bound_attacker(target, player, command, 0)
            .map_err(fixture::debug)?
            .ok_or("successful native hit")?)
    };
    let hit = |stage: u8, creature: &str, item: &str, spell: Option<&str>, w| {
        NativeQuestHit::from_committed(
            format!("oteryn:quest-transition/native-hits/engenc-child-s{stage}"),
            creature.into(),
            item.into(),
            spell.map(str::to_owned),
            u64::from(stage) - 2,
            hash,
            w,
        )
    };

    // Spear lesson: a hit on another creature, or a rune hit, gives no credit.
    for refused in [
        hit(3, "oteryn:creature.rat", SPEAR, None, witness()?),
        hit(3, SKELETON, RUNE, Some(MISSILE), witness()?),
    ] {
        let outcome = slot
            .commit_native_quest_hit(
                &h.root,
                &authority,
                &h.node,
                fence,
                &refused,
                Arc::clone(&catalogue),
            )
            .await;
        assert!(!credited(&outcome), "s3 credited {refused:?}: {outcome:?}");
    }
    assert_eq!(receipts(h, "native_hit").await?, 0);
    assert_eq!(track(h, PHASE).await?, None, "phase stays at its initial 2");

    let spear = hit(3, SKELETON, SPEAR, None, witness()?);
    let committed = slot
        .commit_native_quest_hit(
            &h.root,
            &authority,
            &h.node,
            fence,
            &spear,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(fixture::debug)?;
    assert!(matches!(committed, QuestTransitionOutcome::Committed(_)));
    assert_eq!(track(h, PHASE).await?, Some(3));

    // Rune lesson: another creature, the spear, or another spell gives no credit.
    for refused in [
        hit(4, "oteryn:creature.rat", RUNE, Some(MISSILE), witness()?),
        hit(4, SKELETON, SPEAR, None, witness()?),
        hit(
            4,
            SKELETON,
            RUNE,
            Some("candidate:spell/rune/light_magic_missile_rune"),
            witness()?,
        ),
    ] {
        let outcome = slot
            .commit_native_quest_hit(
                &h.root,
                &authority,
                &h.node,
                fence,
                &refused,
                Arc::clone(&catalogue),
            )
            .await;
        assert!(!credited(&outcome), "s4 credited {refused:?}: {outcome:?}");
    }
    assert_eq!(receipts(h, "native_hit").await?, 1);
    assert_eq!(track(h, PHASE).await?, Some(3));
    assert_eq!(
        track(h, "oteryn:quest-progress/engenc/child/rune").await?,
        None
    );
    Ok(())
}
