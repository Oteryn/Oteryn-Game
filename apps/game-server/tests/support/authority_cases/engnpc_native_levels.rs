//! Source level guards use progression locked by the Character transaction.
use crate::bestiary_postgres_harness::{self as fixture, TestResult, debug};
use crate::durability::{
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
fn engnpc_native_level_guards_recheck_progression_and_preserve_replay() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().name("engnpc-native-levels".into()).stack_size(256 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime().map_err(debug)?.block_on(async {
                let started = std::time::Instant::now();
                let h = fixture::Harness::create(admin, "engnpc_levels", true).await?;
                let result = exercise(&h).await;
                h.cleanup().await?;
                result?;
                eprintln!("POSTGRES_CASE_COMPLETED engnpc_native_level_guards_recheck_progression_and_preserve_replay duration={:?}", started.elapsed());
                Ok::<(), Box<dyn std::error::Error>>(())
            }).map_err(debug)
        })?.join().map_err(|_| "native level thread failed")??;
    Ok(())
}

async fn exercise(h: &fixture::Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let initial = fixture::fence(1)?;
    let level = h
        .root
        .read_character_progression(&authority, initial.character_id)
        .await
        .map_err(debug)?
        .ok_or("progression")?
        .level;
    assert!(level > 0, "the fixture must use a real positive level");
    let mut document: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/gameplay/quests/src/quest/npc_rules/test-fixtures/hero_greeting.json"
    )))?;
    document["program_key"] = "oteryn:quest-npc-program.engnpc_levels".into();
    let template = document["rules"][0].clone();
    let mut rules = Vec::new();
    for (word, guards) in [
        (
            "high",
            serde_json::json!([{"kind":"level","op":"ge","value":level + 1}]),
        ),
        (
            "low",
            serde_json::json!([{"kind":"level","op":"le","value":level - 1}]),
        ),
        (
            "band",
            serde_json::json!([
                {"kind":"level","op":"ge","value":level},
                {"kind":"level","op":"le","value":level},
            ]),
        ),
    ] {
        let mut rule = template.clone();
        rule["key"] = format!("oteryn:quest-npc/engnpc.level/{word}").into();
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
            transition_key: format!("oteryn:quest-npc/engnpc.level/{word}"),
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
    for (id, word) in [(1, "high"), (2, "low")] {
        let result = commit(initial, request(id, word)?).await.map_err(debug)?;
        assert_eq!(
            result.transition,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
        );
        assert_eq!(h.root_revision().await?, "1");
        assert_eq!(h.count("game_character_quest_receipts").await?, 0);
        assert_eq!(h.count("game_character_quest_tracks").await?, 0);
    }
    let band = request(3, "band")?;
    let accepted = commit(initial, band.clone()).await.map_err(debug)?;
    let QuestTransitionOutcome::Committed(receipt) = accepted.transition else {
        return Err("a native level band must use the locked real level".into());
    };
    assert_eq!(receipt.changes.len(), 1);
    let replay = commit(initial, band.clone()).await.map_err(debug)?;
    assert_eq!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(receipt)
    );
    assert_eq!(h.root_revision().await?, "2");
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);
    let mut stale = fixture::fence(2)?;
    stale.character_lease_generation += 1;
    let stale_request = request(4, "band")?;
    assert!(matches!(
        commit(stale, stale_request).await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    let mut conflicting = band;
    conflicting.message = "band changed".into();
    assert!(matches!(
        commit(fixture::fence(2)?, conflicting).await,
        Err(CharacterProgressionError::ConflictingOccurrence)
    ));
    assert_eq!(h.root_revision().await?, "2");
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);
    eprintln!(
        "ENGNPC_NATIVE_LEVEL observed_level={level} lower_and_upper_refused exact_band_committed_once"
    );
    Ok(())
}
