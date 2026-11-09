//! Source vocation guards use build state locked by the Character transaction.
use crate::bestiary_postgres_harness::{self as fixture, TestResult, debug};
use crate::durability::{
    character_build::{
        BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildFormula, BuildOccurrence,
        convert_vocation,
    },
    character_progression::CharacterProgressionError,
    quest_state::{
        QuestTransitionOutcome, grizzly::NoGrizzlyPremium, npc_program::NpcProgramRequest,
    },
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{QuestRefusal, npc_rules::NpcRuleTrigger};
use std::sync::Arc;

#[test]
fn engnpc_native_vocation_guards_recheck_build_and_preserve_replay() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().name("engnpc-native-vocations".into()).stack_size(256 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime().map_err(debug)?.block_on(async {
                let started = std::time::Instant::now();
                let h = fixture::Harness::create(admin, "engnpc_vocations", true).await?;
                let result = exercise(&h).await;
                h.cleanup().await?;
                result?;
                eprintln!("POSTGRES_CASE_COMPLETED engnpc_native_vocation_guards_recheck_build_and_preserve_replay duration={:?}", started.elapsed());
                Ok::<(), Box<dyn std::error::Error>>(())
            }).map_err(debug)
        })?.join().map_err(|_| "native vocation thread failed")??;
    Ok(())
}

// A fixture formula for a genuine Character build-owner vocation choice.
struct Formula;
impl BuildFormula for Formula {
    fn required(&self, _vocation: &str, family: usize, level: u16) -> Option<u64> {
        (family < 8 && level > if family == 0 { 0 } else { 10 }).then_some(100)
    }
    fn digest(&self) -> [u8; 32] {
        [7; 32]
    }
    fn content_revision(&self) -> &str {
        "content-1"
    }
}

async fn exercise(h: &fixture::Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let initial = fixture::fence(1)?;
    let build = h
        .root
        .read_character_build_state(&authority, initial.character_id)
        .await
        .map_err(debug)?;
    let vocation = format!("oteryn:vocation.{}", build.vocation());
    assert_eq!(vocation, "oteryn:vocation.none");
    let mut document: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/gameplay/quests/src/quest/npc_rules/test-fixtures/hero_greeting.json"
    )))?;
    document["program_key"] = "oteryn:quest-npc-program.engnpc_vocations".into();
    let template = document["rules"][0].clone();
    let mut rules = Vec::new();
    for (word, guards) in [
        (
            "other",
            serde_json::json!([{"kind":"vocation","value":"oteryn:vocation.knight"}]),
        ),
        (
            "current",
            serde_json::json!([{"kind":"vocation","value":vocation}]),
        ),
    ] {
        let mut rule = template.clone();
        rule["key"] = format!("oteryn:quest-npc/engnpc.vocation/{word}").into();
        rule["trigger"] = "say".into();
        rule["keywords"] = serde_json::json!([word]);
        rule["guards"] = guards;
        rules.push(rule);
    }
    document["rules"] = rules.into();
    let (source, catalogue) =
        super::engnpc_native_premium::bound_catalogue(&[&document.to_string()])?;
    let permit = SpellLanePermit::of_fresh_scope(initial.runtime_scope).await?;
    let items = super::engnpc_native_premium::NoItems;
    let gate = NoGrizzlyPremium;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(debug)?;
    let request = |id, word: &str| -> TestResult<_> {
        Ok(NpcProgramRequest {
            command: CommandRef::new(session, CommandId::new(id).map_err(debug)?),
            npc: "oteryn:npc.strange_pipe".into(),
            trigger: NpcRuleTrigger::Say,
            message: word.into(),
            topic: 0,
            transition_key: format!("oteryn:quest-npc/engnpc.vocation/{word}"),
        })
    };
    let commit = |fence, request| {
        h.root.fixture_commit_npc_program_from_source(
            &authority,
            &h.node,
            &permit,
            fence,
            request,
            Arc::clone(&catalogue),
            &items,
            &source,
            &gate,
        )
    };
    let result = commit(initial, request(1, "other")?).await.map_err(debug)?;
    assert_eq!(
        result.transition,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(h.root_revision().await?, "1");
    assert_eq!(h.count("game_character_quest_receipts").await?, 0);
    assert_eq!(h.count("game_character_quest_tracks").await?, 0);
    let band = request(3, "current")?;
    let accepted = commit(initial, band.clone()).await.map_err(debug)?;
    let QuestTransitionOutcome::Committed(receipt) = accepted.transition else {
        return Err("a native vocation guard must use the locked Character build".into());
    };
    assert_eq!(receipt.changes.len(), 1);
    let replay = commit(initial, band.clone()).await.map_err(debug)?;
    assert_eq!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(receipt.clone())
    );
    assert_eq!(h.root_revision().await?, "2");
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);
    let mut stale = fixture::fence(2)?;
    stale.character_lease_generation += 1;
    let stale_request = request(4, "current")?;
    assert!(matches!(
        commit(stale, stale_request).await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    let mut conflicting = band.clone();
    conflicting.message = "current changed".into();
    assert!(matches!(
        commit(fixture::fence(2)?, conflicting).await,
        Err(CharacterProgressionError::ConflictingOccurrence)
    ));
    assert_eq!(h.root_revision().await?, "2");
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);
    let after = convert_vocation(&Formula, &build, "knight").map_err(debug)?;
    let changed = h
        .root
        .commit_character_build(
            &authority,
            &h.node,
            fixture::fence(2)?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(fixture::id(83)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: build,
                after,
                pruned_stance: None,
            },
            &Formula,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(changed, BuildCommitOutcome::Committed(_)));
    assert_eq!(
        h.root
            .read_character_build_state(&authority, initial.character_id)
            .await
            .map_err(debug)?
            .vocation(),
        "knight"
    );
    let refused = commit(fixture::fence(3)?, request(5, "current")?)
        .await
        .map_err(debug)?;
    assert_eq!(
        refused.transition,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    // Exact-command recovery remains valid after the vocation owner changes.
    let recovered = commit(initial, band).await.map_err(debug)?;
    assert_eq!(
        recovered.transition,
        QuestTransitionOutcome::AlreadyCommitted(receipt)
    );
    assert_eq!(h.root_revision().await?, "3");
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);
    eprintln!(
        "ENGNPC_NATIVE_VOCATION observed_vocation={vocation} wrong_vocation_refused current_vocation_committed_once"
    );
    Ok(())
}
