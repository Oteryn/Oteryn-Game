//! Native NPC requirements join the transaction and preserve bound replay.
use crate::bestiary_postgres_harness::{self as fixture, TestResult, debug};
use crate::durability::{
    DurabilityError,
    character_progression::CharacterProgressionError,
    item_loop_move::ItemLoopContent,
    item_mint::TypedDefinitionRef,
    item_transfer::{ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused},
    quest_state::{
        QuestTransitionOutcome, grizzly::GrizzlyPremiumGate, npc_program::NpcProgramRequest,
    },
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestRefusal, QuestStateCatalogue, QuestTrack,
    QuestTransition,
    npc_rules::{NpcRuleCatalog, NpcRuleTrigger},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

const FREE: &str = "oteryn:quest-npc/engnpc.premium/free";
const PAID: &str = "oteryn:quest-npc/engnpc.premium/paid";

#[test]
fn engnpc_native_premium_is_selected_fenced_atomic_and_replay_safe() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().name("engnpc-native-premium".into()).stack_size(256 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime().map_err(debug)?.block_on(async {
                let started = std::time::Instant::now();
                let h = fixture::Harness::create(admin, "engnpc_premium", true).await?;
                let result = exercise(&h).await;
                h.cleanup().await?;
                result?;
                eprintln!("POSTGRES_CASE_COMPLETED engnpc_native_premium_is_selected_fenced_atomic_and_replay_safe duration={:?}", started.elapsed());
                Ok::<(), Box<dyn std::error::Error>>(())
            }).map_err(debug)
        })?.join().map_err(|_| "native Premium thread failed")??;
    Ok(())
}

struct Gate {
    account: [u8; 16],
    allowed: AtomicBool,
    calls: AtomicUsize,
}

impl GrizzlyPremiumGate for Gate {
    async fn task_permitted(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        if account != self.account {
            return Err(DurabilityError::InvalidStoredState);
        }
        let xid: i64 = sqlx::query_scalar("SELECT txid_current()::bigint")
            .fetch_one(&mut **tx)
            .await?;
        if xid <= 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.allowed.load(Ordering::SeqCst))
    }
}

struct NoItems;
impl ItemWeightLookup for NoItems {
    fn generation_digest(&self) -> [u8; 32] {
        crate::durability::item_weight_lookup::TEST_WEIGHTS.generation_digest()
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        crate::durability::item_weight_lookup::TEST_WEIGHTS.weight_of(definition)
    }
}
impl ItemLoopContent for NoItems {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.generation_digest()
    }
    fn loop_facts(&self, _definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        None
    }
}

fn catalogues() -> TestResult<(NpcRuleCatalog, Arc<QuestStateCatalogue>)> {
    let original: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/gameplay/quests/src/quest/npc_rules/test-fixtures/hero_greeting.json"
    )))?;
    let mut documents = Vec::new();
    for (name, key, premium) in [("free", FREE, false), ("paid", PAID, true)] {
        let mut document = original.clone();
        let track = format!("oteryn:quest-progress/engnpc.premium/{name}");
        document["program_key"] = format!("oteryn:quest-npc-program.engnpc_{name}").into();
        document["premium_required"] = premium.into();
        document["tracks"][0]["key"] = track.clone().into();
        document["rules"][0]["key"] = key.into();
        document["rules"][0]["effects"][0]["track"] = track.into();
        if !premium {
            document["rules"][0]["trigger"] = "say".into();
            document["rules"][0]["keywords"] = serde_json::json!(["free"]);
        }
        documents.push(document.to_string());
    }
    let source =
        NpcRuleCatalog::from_json_documents(&[&documents[0], &documents[1]]).map_err(debug)?;
    let mut tracks = Vec::new();
    let mut bindings = Vec::new();
    for program in source.programs() {
        tracks.extend(program.document.tracks.iter().map(|track| QuestTrack {
            key: track.key.clone(),
            quest: track.quest.clone(),
            initial: track.initial,
            min: track.min,
            max: track.max,
        }));
        let digest = program
            .digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        bindings.push(QuestTransition {
            key: format!("oteryn:quest-npc-pin/{digest}"),
            quest: program.document.quest.clone(),
            effects: vec![QuestEffect {
                track: program
                    .document
                    .tracks
                    .first()
                    .ok_or("fixture track")?
                    .key
                    .clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Computed,
            }],
            completes: false,
            experience: None,
            world_change: None,
        });
    }
    let catalogue =
        Arc::new(QuestStateCatalogue::new("content-1", tracks, bindings).map_err(debug)?);
    Ok((source, catalogue))
}

async fn exercise(h: &fixture::Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let (source, catalogue) = catalogues()?;
    let account: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let gate = Gate {
        account: account.try_into().map_err(|_| "account UUID")?,
        allowed: AtomicBool::new(false),
        calls: AtomicUsize::new(0),
    };
    let items = NoItems;
    let permit = SpellLanePermit::of_fresh_scope(fixture::fence(1)?.runtime_scope).await?;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(debug)?;
    let request = |id, key: &str, message: &str, trigger| -> TestResult<_> {
        Ok(NpcProgramRequest {
            command: CommandRef::new(session, CommandId::new(id).map_err(debug)?),
            npc: "oteryn:npc.strange_pipe".into(),
            trigger,
            message: message.into(),
            topic: 0,
            transition_key: key.into(),
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
    // A Premium programme sharing the aggregate quest must not gate a free rule.
    let free = commit(
        fixture::fence(1)?,
        request(1, FREE, "free", NpcRuleTrigger::Say)?,
    )
    .await
    .map_err(debug)?;
    assert!(matches!(
        free.transition,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(gate.calls.load(Ordering::SeqCst), 0);
    let denied = commit(
        fixture::fence(2)?,
        request(2, PAID, "hi", NpcRuleTrigger::Greet)?,
    )
    .await
    .map_err(debug)?;
    assert_eq!(
        denied.transition,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch),
        "a fresh Premium-only source write must fail closed"
    );
    assert_eq!(gate.calls.load(Ordering::SeqCst), 1);
    assert_eq!(h.root_revision().await?, "2");
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);
    assert_eq!(h.count("game_character_quest_tracks").await?, 1);
    gate.allowed.store(true, Ordering::SeqCst);
    let mut stale = fixture::fence(2)?;
    stale.character_lease_generation += 1;
    assert!(matches!(
        commit(stale, request(3, PAID, "hi", NpcRuleTrigger::Greet)?).await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    assert_eq!(gate.calls.load(Ordering::SeqCst), 1);
    let paid = request(4, PAID, "hi", NpcRuleTrigger::Greet)?;
    let accepted = commit(fixture::fence(2)?, paid.clone())
        .await
        .map_err(debug)?;
    let QuestTransitionOutcome::Committed(receipt) = accepted.transition else {
        return Err("Premium-only source commit".into());
    };
    assert_eq!(gate.calls.load(Ordering::SeqCst), 2);
    gate.allowed.store(false, Ordering::SeqCst);
    let replay = commit(fixture::fence(2)?, paid.clone())
        .await
        .map_err(debug)?;
    assert_eq!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(receipt)
    );
    assert_eq!(replay.reply, accepted.reply);
    assert_eq!(
        gate.calls.load(Ordering::SeqCst),
        2,
        "replay must not rerun current eligibility"
    );
    let mut conflicting = paid;
    conflicting.message = "hi changed".into();
    assert!(matches!(
        commit(fixture::fence(2)?, conflicting).await,
        Err(CharacterProgressionError::ConflictingOccurrence)
    ));
    assert_eq!(h.root_revision().await?, "3");
    assert_eq!(h.count("game_character_quest_receipts").await?, 2);
    assert_eq!(h.count("game_character_quest_tracks").await?, 2);
    assert_eq!(gate.calls.load(Ordering::SeqCst), 2);
    Ok(())
}
