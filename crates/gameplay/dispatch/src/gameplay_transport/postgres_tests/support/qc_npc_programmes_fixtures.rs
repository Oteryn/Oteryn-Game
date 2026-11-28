use super::classic_quests_fixtures::install;
use super::fixture::{Harness, debug};
use super::qengine_rewards_fixtures;
use super::questpc7_friends_fixtures::{mint, select_npc_capabilities};
use super::questpc7_outfit_items_fixtures::quantity;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
use crate::gameplay_transport::tcp_tls::write_frame;
use oteryn_protocol_oteryn::npc_service::{
    COMMAND_TYPE_NPC_TALK_INTENT, NpcIntentDisposition, NpcTalkIntent,
    STATE_DOMAIN_NPC_CONVERSATION, decode_npc_conversation, decode_npc_intent_result,
    encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, decode_state_delta,
    encode_client_command,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const STEEL_QUOTE: &str = "I can make draconian steel from a Dragon Shield. Do you want to trade?";
const AMULET_QUOTE: &str =
    "Me can do unbroken but Big Ben want gold 5000 and all four pieces. Yes?";
const FAVOR_QUOTE: &str = "Firy steel it is. Need green ones' breath to melt. Or red even better. Me can make from shield. Lil' one want to trade?";
const FAVOR_CONFIRM: &str = "Wait. Me work no cheap is. Do favour for me first, yes?";
const FAVOR_START: &str = "Me need gift for woman. We dance, so me want to give her bast skirt. But she big is. So I need many to make big one. Bring three okay? Me wait.";
const SKIRT_PROMPT: &str = "Lil' one bring three bast skirts.";
const SKIRT_SUCCESS: &str = "Good good! Woman happy will be. Now me happy too and help you.";
const TRACK: &str = "oteryn:quest-progress/questpc7/friends/program";

const SHIELD: u32 = 3416;
const STEEL: u32 = 5889;
const PLATINUM: u32 = 3035;
const FRAGMENTS: [u32; 4] = [7528, 7529, 7530, 7531];
const REWARD: &str = "oteryn:quest-reward/qc-obsidian-draconian-steel";
const DONOR_EXCHANGES: [(&str, u32, u32); 3] = [
    ("uth'kean", 3381, 5887),
    ("za'ralator", 3356, 5888),
    ("uth'prta", 3281, 5892),
];

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    case: &str,
) -> TestResult {
    select_npc_capabilities(owner, admitted)?;
    let [cyclops] = install(owner, admitted, ["oteryn:npc.a_sweaty_cyclops"]).await?;
    if case == "unlock" {
        return unlock(owner, admitted, h, cyclops).await;
    }
    if case != "locked-soul-orb" {
        for nonce in 7000..7003 {
            mint(owner, admitted, 3560, 1, nonce).await?;
        }
        with_wire(owner, admitted, cyclops, async |wire| {
            wire.greet().await?;
            wire.line(
                "forging friendship",
                "Bring three bast skirts for my partner.",
            )
            .await?;
            wire.line("deliver skirts", "Now I can forge royal steel for you.")
                .await
        })
        .await?;
    }
    // Seed custody while no connection is reading the Character authority.
    match case {
        "replaces-amulet" | "amulet-replaces-steel" | "locked-soul-orb" => {
            for item in FRAGMENTS {
                mint(owner, admitted, item, 1, u64::from(item)).await?;
            }
            mint(owner, admitted, PLATINUM, 50, 7003).await?;
            mint(owner, admitted, SHIELD, 1, 7004).await?;
        }
        "keyword" | "repeat" => mint(owner, admitted, SHIELD, 1, 7101).await?,
        "donor-exchanges" => {
            mint(owner, admitted, SHIELD, 1, 7101).await?;
            for (_, input, _) in DONOR_EXCHANGES {
                mint(owner, admitted, input, 1, u64::from(input)).await?;
            }
        }
        "other-services" => {
            mint(owner, admitted, SHIELD, 1, 7100).await?;
            mint(owner, admitted, 3381, 1, 7101).await?;
        }
        "mint-refusal" => fill_backpack_with_nested_shield(owner, admitted, h).await?,
        _ => {}
    }

    let session = *admitted;
    let retry = with_wire(owner, admitted, cyclops, async |wire| {
        exercise_conversation(owner, &session, h, case, wire).await
    })
    .await?;
    if let Some(command) = retry {
        assert_eq!(admitted.continuity.next_command_id, command + 1);
        let original = crate::foundation::CommandRef::new(
            admitted.game_session_id,
            crate::foundation::CommandId::new(command).map_err(debug)?,
        );
        assert!(owner.states.command_ingress.reconcilable(original));
        let committed = snapshot(h).await?;
        with_wire(owner, admitted, cyclops, async |wire| {
            wire.recovered_reply("Cling clang!").await?;
            // Replaying the command still returns its original acknowledgement.
            assert_eq!(
                wire.exchange(command, "yes", false).await?,
                Some((NpcIntentDisposition::Ok, vec![])),
                "the original acknowledgement is replayed",
            );
            Ok(())
        })
        .await?;
        assert!(owner.states.command_ingress.original_terminalized(original));
        assert_eq!(admitted.continuity.next_command_id, command + 1);
        assert_eq!(
            snapshot(h).await?,
            committed,
            "replay leaves the committed exchange intact"
        );

        mint(owner, admitted, SHIELD, 1, 7102).await?;
        with_wire(owner, admitted, cyclops, async |wire| {
            wire.greet().await?;
            wire.line("uth'lokr", STEEL_QUOTE).await?;
            assert_eq!(quantity(h, SHIELD).await?, 1, "the quote spends no shield");
            assert_eq!(quantity(h, STEEL).await?, 1);
            wire.line("yes", "Cling clang!").await
        })
        .await?;
        assert_eq!(quantity(h, SHIELD).await?, 0);
        assert_eq!(quantity(h, STEEL).await?, 2);
        assert_exchange_receipts(h).await?;
        mint(owner, admitted, SHIELD, 1, 7103).await?;
        crate::gameplay_transport::tcp_tls::FAIL_NPC_LINE.scope("Cling clang!", async {
            with_wire(owner, admitted, cyclops, async |wire| {
                wire.greet().await?;
                wire.line("uth'lokr", STEEL_QUOTE).await?;
                wire.disposition("yes", NpcIntentDisposition::Ok).await
            }).await
        }).await?;
        let committed = snapshot(h).await?;
        with_wire(owner, admitted, cyclops, async |wire| {
            wire.recovered_reply("Cling clang!").await
        }).await?;
        assert!(owner.states.npc_conversations.recovered_quest_lines(admitted.game_session_id).is_none());
        assert_eq!(snapshot(h).await?, committed);
        assert_eq!(quantity(h, STEEL).await?, 3);
    }
    Ok(())
}

async fn exercise_conversation(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    case: &str,
    wire: &mut Wire<'_>,
) -> TestResult<Option<u64>> {
    wire.greet().await?;
    match case {
        "replaces-amulet" => {
            wire.line("amulet", AMULET_QUOTE).await?;
            wire.line("uth'lokr", STEEL_QUOTE).await?;
            wire.line("yes", "Cling clang!").await?;
            assert_eq!(quantity(h, SHIELD).await?, 0, "yes must spend the shield");
            assert_eq!(quantity(h, STEEL).await?, 1);
            for item in FRAGMENTS {
                assert_eq!(quantity(h, item).await?, 1, "amulet fragment {item}");
            }
            assert_eq!(quantity(h, PLATINUM).await?, 50);
        }
        "amulet-replaces-steel" | "locked-soul-orb" => {
            if case == "amulet-replaces-steel" {
                wire.line("uth'lokr", STEEL_QUOTE).await?;
                wire.line("please make an amulet", AMULET_QUOTE).await?;
            } else {
                wire.line("amulet", AMULET_QUOTE).await?;
                assert!(wire.talk("soul orb", false).await?.is_some());
            }
            wire.line("yes", "Well, well. I do that! Big Ben makes li'l amulet unbroken with big hammer in big hands! No worry! Come back after sun hits the horizon 24 times and ask me for amulet.").await?;
            assert_eq!(quantity(h, SHIELD).await?, 1);
            assert_eq!(quantity(h, STEEL).await?, 0);
            for item in FRAGMENTS {
                assert_eq!(quantity(h, item).await?, 0, "amulet fragment {item}");
            }
            assert_eq!(quantity(h, PLATINUM).await?, 0);
            if case == "locked-soul-orb" {
                wire.line("uth'lokr", FAVOR_QUOTE).await?;
                assert!(wire.talk("soul orb", false).await?.is_some());
                wire.line("yes", FAVOR_CONFIRM).await?;
                assert!(wire.talk("soul orb", false).await?.is_some());
                wire.line("yes", FAVOR_START).await?;
                wire.line("uth'lokr", SKIRT_PROMPT).await?;
                assert!(wire.talk("soul orb", false).await?.is_some());
                assert_eq!(
                    wire.talk("yes", true).await?,
                    Some((NpcIntentDisposition::Rejected, vec![SKIRT_PROMPT.into()]))
                );
            }
        }
        "other-services" => {
            replaces_with_other_services(h, wire).await?;
        }
        "donor-exchanges" => {
            let source: serde_json::Value = serde_json::from_str(include_str!(
                "../../../../../../../content/quests/npc-offers/qc-programmes.json"
            ))?;
            let offers = source["offers"].as_array().ok_or("Cyclops offers")?;
            for (word, input, output) in DONOR_EXCHANGES {
                let quote = offers.iter().find(|row| row["keywords"][0] == word)
                    .and_then(|row| row["reply"][0].as_str()).ok_or(word)?;
                wire.line("uth'lokr", STEEL_QUOTE).await?;
                wire.line(word, quote).await?;
                let (result, _) = wire.talk("yes", true).await?.ok_or("exchange reply")?;
                assert_eq!(result, NpcIntentDisposition::Ok, "{word}");
                assert_eq!(quantity(h, input).await?, 0, "{word}");
                assert_eq!(quantity(h, output).await?, 1, "{word}");
                assert_eq!(quantity(h, SHIELD).await?, 1, "{word} replaced the steel quote");
                assert_eq!(quantity(h, STEEL).await?, 0);
            }
            // PARTIAL: these services supersede the quote but have no admitted exchange.
            for word in ["soul orb", "melt", "gear whell"] {
                wire.line("uth'lokr", STEEL_QUOTE).await?;
                assert!(wire.talk(word, false).await?.is_some());
                wire.disposition("yes", NpcIntentDisposition::Rejected).await?;
                assert_eq!(quantity(h, SHIELD).await?, 1, "{word}");
                assert_eq!(quantity(h, STEEL).await?, 0);
            }
        }
        "keyword" => {
            assert!(wire.talk("xuth'lokr, uth'lokr", false).await?.is_some());
            wire.disposition("yes", NpcIntentDisposition::Rejected)
                .await?;
            wire.line("please forge UTH'LOKR", STEEL_QUOTE).await?;
            assert!(wire.talk("yes please", false).await?.is_some());
            assert!(wire.talk(" yes ", false).await?.is_some());
            assert_eq!(quantity(h, SHIELD).await?, 1);
            assert_eq!(quantity(h, STEEL).await?, 0);
            wire.line("YES", "Cling clang!").await?;
            assert_eq!(quantity(h, SHIELD).await?, 0);
            assert_eq!(quantity(h, STEEL).await?, 1);
        }
        "repeat" => {
            wire.line("uth'lokr", STEEL_QUOTE).await?;
            assert_eq!(quantity(h, SHIELD).await?, 1, "the quote spends no shield");
            assert_eq!(quantity(h, STEEL).await?, 0);
            let command = wire.next;
            owner
                .states
                .npc_conversations
                .fixture_interrupt_quest_dialogue_commit(admitted.game_session_id, command)?;
            assert_eq!(
                wire.talk("yes", false).await?,
                None,
                "the committed command lost its acknowledgement"
            );
            assert_eq!(quantity(h, SHIELD).await?, 0);
            assert_eq!(quantity(h, STEEL).await?, 1);
            return Ok(Some(command));
        }
        "no-shield" => {
            wire.line("uth'lokr", STEEL_QUOTE).await?;
            let before = snapshot(h).await?;
            wire.disposition("yes", NpcIntentDisposition::Rejected)
                .await?;
            assert_eq!(snapshot(h).await?, before);
        }
        "ingredient-retry" => {
            let source: serde_json::Value = serde_json::from_str(include_str!(
                "../../../../../../../content/quests/npc-offers/qc-programmes.json"
            ))?;
            let offers = source["offers"].as_array().ok_or("Cyclops offers")?;
            for (word, _, output) in [("uth'lokr", SHIELD, STEEL)]
                .into_iter()
                .chain(DONOR_EXCHANGES)
            {
                let quote = offers
                    .iter()
                    .find(|row| row["keywords"][0] == word && row["storage"][0]["minimum"] == 1)
                    .ok_or(word)?;
                wire.line(word, quote["reply"][0].as_str().ok_or("metal quote")?)
                    .await?;
                wire.disposition("yes", NpcIntentDisposition::Rejected)
                    .await?;
                assert_eq!(quantity(h, output).await?, 0, "{word}");
                let topic = quote["next_topic"].as_u64().ok_or("metal quote topic")?;
                assert_eq!(
                    u64::from(
                        owner
                            .states
                            .npc_conversations
                            .fixture_quest_topic(admitted.game_session_id)?
                    ),
                    topic,
                    "missing ingredients preserve the live {word} quote"
                );
            }
        }
        "mint-refusal" => {
            // Sequence increments survive rollback, proving the refusal reached consumption.
            sqlx::raw_sql(
                "CREATE SEQUENCE cyclops_test_consumed;
                 GRANT USAGE ON SEQUENCE cyclops_test_consumed TO oteryn_game_runtime;
                 CREATE FUNCTION cyclops_test_mark_consumed() RETURNS trigger LANGUAGE plpgsql AS $$
                 BEGIN PERFORM nextval('cyclops_test_consumed'); RETURN NEW; END $$;
                 CREATE TRIGGER cyclops_test_mark_consumed AFTER UPDATE ON game_item_instances
                   FOR EACH ROW WHEN (OLD.definition_production_key='oteryn:item.tibia.i3416'
                                      AND OLD.lifecycle=1 AND NEW.lifecycle=2)
                   EXECUTE FUNCTION cyclops_test_mark_consumed()",
            )
            .execute(&h.pool)
            .await?;
            wire.line("uth'lokr", STEEL_QUOTE).await?;
            let before = snapshot(h).await?;
            wire.disposition("yes", NpcIntentDisposition::Rejected)
                .await?;
            assert!(
                sqlx::query_scalar::<_, bool>("SELECT is_called FROM cyclops_test_consumed")
                    .fetch_one(&h.pool)
                    .await?,
                "mint must refuse after shield consumption"
            );
            assert_eq!(snapshot(h).await?, before);
            assert_eq!(quantity(h, STEEL).await?, 0);
        }
        _ => return Err(format!("unknown Cyclops exchange case {case}").into()),
    }
    Ok(None)
}

async fn assert_exchange_receipts(h: &Harness) -> TestResult {
    let exchanges: Vec<(i32, i64, bool)> = sqlx::query_as(
                "SELECT c.quantity, m.quantity, c.created_xact_id=s.created_xact_id
                         AND m.created_xact_id=s.created_xact_id
                 FROM game_quest_reward_sources s
                 JOIN game_quest_callback_consumptions c USING(character_id,cause_id,cause_ordinal,transition_key)
                 JOIN game_reward_claim_mint_receipts m ON m.transaction_id=s.mint_transaction_id
                 JOIN game_item_instances i ON i.item_instance_id=m.item_instance_id
                 WHERE s.transition_key=$1
                   AND c.before_item->>'definition_production_key'='oteryn:item.tibia.i3416'
                   AND i.definition_production_key='oteryn:item.tibia.i5889'",
            )
            .bind(REWARD)
            .fetch_all(&h.pool)
            .await?;
    assert_eq!(exchanges, [(1, 1, true); 2]);
    Ok(())
}

async fn snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_array(
             (SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),
             (SELECT jsonb_agg(to_jsonb(t) ORDER BY character_id,track_key) FROM game_character_quest_tracks t),
             (SELECT count(*) FROM game_character_quest_receipts),
             (SELECT count(*) FROM game_quest_reward_sources),
             (SELECT count(*) FROM game_item_fee_burns),
             (SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_nested_entries e),
             (SELECT count(*) FROM game_quest_callback_consumptions),
             (SELECT count(*) FROM game_reward_claim_mint_receipts)
         )::text",
    )
    .fetch_one(&h.pool)
    .await?)
}

async fn unlock(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    cyclops: EntityRef,
) -> TestResult {
    let session = *admitted;
    with_wire(owner, admitted, cyclops, async |wire| {
        wire.greet().await?;
        wire.line("uth'lokr", FAVOR_QUOTE).await?;
        wire.line("yes", FAVOR_CONFIRM).await?;
        assert_eq!(program(h).await?, -1);
        wire.line("yes", FAVOR_START).await?;
        assert_eq!(program(h).await?, 0);
        wire.line("uth'lokr", SKIRT_PROMPT).await?;
        let before = snapshot(h).await?;
        assert_eq!(
            wire.talk("yes", true).await?,
            Some((NpcIntentDisposition::Rejected, vec![SKIRT_PROMPT.into()]))
        );
        assert_eq!(snapshot(h).await?, before);
        for nonce in 7000..7003 {
            mint(owner, &session, 3560, 1, nonce).await?;
        }
        assert_eq!(
            wire.talk("yes", false).await?,
            Some((NpcIntentDisposition::Rejected, vec![])),
            "a refused hand-in must require a new skirt prompt",
        );
        assert_eq!(quantity(h, 3560).await?, 3);
        wire.line("uth'lokr", SKIRT_PROMPT).await?;
        assert_eq!(
            quantity(h, 3560).await?,
            3,
            "only confirmation hands in skirts"
        );
        wire.line("yes", SKIRT_SUCCESS).await?;
        assert_eq!(quantity(h, 3560).await?, 0);
        assert_eq!(program(h).await?, 1);
        wire.line("uth'lokr", STEEL_QUOTE).await
    })
    .await
}

async fn program(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1)",
    )
    .bind(TRACK)
    .fetch_one(&h.pool)
    .await?)
}

async fn replaces_with_other_services(h: &Harness, wire: &mut Wire<'_>) -> TestResult {
    const PHASE: &str = "oteryn:quest-progress/chosen-trial/dragon_slayer/phase";
    // A report requested before the trial must still cancel the pending steel exchange.
    wire.line("uth'lokr", STEEL_QUOTE).await?;
    wire.disposition("dragon slayer report", NpcIntentDisposition::Rejected)
        .await?;
    wire.disposition("yes", NpcIntentDisposition::Rejected)
        .await?;
    assert_eq!(quantity(h, SHIELD).await?, 1);

    wire.line("uth'lokr", STEEL_QUOTE).await?;
    wire.line("forge royal steel", "Cling clang!").await?;
    wire.disposition("yes", NpcIntentDisposition::Rejected)
        .await?;
    assert_eq!(quantity(h, 3381).await?, 0);
    assert_eq!(quantity(h, 5887).await?, 1);

    wire.line("uth'lokr", STEEL_QUOTE).await?;
    wire.line("dragon slayer trial", "Thank you for your help.")
        .await?;
    let phase = sqlx::query_scalar::<_, i64>(
        "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
    )
    .bind(PHASE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(phase, 1);
    wire.disposition("yes", NpcIntentDisposition::Rejected)
        .await?;
    assert_eq!(quantity(h, SHIELD).await?, 1);
    assert_eq!(quantity(h, STEEL).await?, 0);
    Ok(())
}

async fn fill_backpack_with_nested_shield(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    const BAG: u32 = 2854;
    mint(owner, admitted, BAG, 1, 7100).await?;
    mint(owner, admitted, SHIELD, 1, 7101).await?;
    let item = |key: u32| async move {
        sqlx::query_scalar::<_, sqlx::types::Uuid>(
            "SELECT i.item_instance_id FROM game_item_container_entries e
             JOIN game_item_instances i USING(item_instance_id)
             WHERE i.definition_production_key=$1 AND i.lifecycle=1",
        )
        .bind(format!("oteryn:item.tibia.i{key}"))
        .fetch_one(&h.pool)
        .await
    };
    let (shield, bag) = (item(SHIELD).await?, item(BAG).await?);
    qengine_rewards_fixtures::into_bag(owner, admitted, *shield.as_bytes(), *bag.as_bytes(), 8000)
        .await?;
    let nested: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=$1 AND parent_item_instance_id=$2)",
    )
    .bind(shield)
    .bind(bag)
    .fetch_one(&h.pool)
    .await?;
    assert!(nested, "the shield must be in the inner backpack");
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(
        native,
        &crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: format!("oteryn:item.tibia.i{BAG}"),
            revision_ref: "definition-r1".into(),
        },
    )
    .map_err(debug)?;
    let capacity = facts.container_capacity.ok_or("capacity")?;
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
        .fetch_one(&h.pool)
        .await?;
    for index in u64::try_from(occupied)?..u64::from(capacity) {
        mint(owner, admitted, 3031, 100, 7200 + index).await?;
    }
    let slots: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        slots,
        i64::from(capacity),
        "consuming the nested shield frees no main slot"
    );
    Ok(())
}

pub(super) struct Wire<'a> {
    client: tokio::io::DuplexStream,
    npc: EntityRef,
    generation: u64,
    next: u64,
    clock: &'a AtomicU64,
}

impl Wire<'_> {
    pub(super) async fn greet(&mut self) -> TestResult {
        let (result, _) = self.talk("hi", true).await?.ok_or("greeting reply")?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        Ok(())
    }

    pub(super) async fn line(&mut self, text: &str, line: &str) -> TestResult {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), self.talk(text, true))
                .await
                .map_err(|_| format!("no wire reply for {text:?}"))??,
            Some((NpcIntentDisposition::Ok, vec![line.to_owned()])),
            "{text}"
        );
        Ok(())
    }

    async fn disposition(&mut self, text: &str, expected: NpcIntentDisposition) -> TestResult {
        assert_eq!(
            self.talk(text, false).await?,
            Some((expected, vec![])),
            "{text}"
        );
        Ok(())
    }

    async fn talk(
        &mut self,
        text: &str,
        expects_reply: bool,
    ) -> TestResult<Option<(NpcIntentDisposition, Vec<String>)>> {
        self.exchange(self.next, text, expects_reply).await
    }

    async fn exchange(
        &mut self,
        command_id: u64,
        text: &str,
        expects_reply: bool,
    ) -> TestResult<Option<(NpcIntentDisposition, Vec<String>)>> {
        let replay = command_id < self.next;
        // Advance the existing fixture clock past the NPC talk rate window.
        self.clock.fetch_add(1_000_001, Ordering::Relaxed);
        let payload = encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: self.npc,
            text: text.into(),
        })
        .map_err(debug)?;
        let command = encode_client_command(
            self.generation,
            &ClientCommandValue {
                command_id,
                command_type: COMMAND_TYPE_NPC_TALK_INTENT,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        write_frame(&mut self.client, &command).await?;
        let mut disposition = None;
        while let Some(bytes) = self.read().await? {
            let frame = decode_wire_envelope(&bytes).map_err(debug)?;
            match frame.message_type() {
                MessageType::CommandResult => {
                    let result = decode_command_result(frame.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, command_id);
                    let decoded = decode_npc_intent_result(result.payload).map_err(debug)?;
                    let status = if replay {
                        CommandStatus::DuplicateReplay
                    } else if decoded == NpcIntentDisposition::Rejected {
                        CommandStatus::Rejected
                    } else {
                        CommandStatus::Accepted
                    };
                    assert_eq!(result.status, status);
                    assert!(disposition.replace(decoded).is_none());
                    if !expects_reply {
                        self.next += u64::from(!replay);
                        return Ok(Some((decoded, vec![])));
                    }
                }
                MessageType::StateDelta => {
                    let delta = decode_state_delta(frame.payload()).map_err(debug)?;
                    if delta.domain_id == STATE_DOMAIN_NPC_CONVERSATION {
                        let conversation = decode_npc_conversation(delta.payload).map_err(debug)?;
                        if let Some(result) = disposition {
                            assert_eq!(
                                conversation.open,
                                Some((
                                    self.npc,
                                    oteryn_protocol_oteryn::npc_service::NpcConversationState::Open
                                ))
                            );
                            self.next += u64::from(!replay);
                            return Ok(Some((result, conversation.lines)));
                        }
                        assert!(
                            conversation.lines.is_empty(),
                            "unexpected dialogue before result for {text}"
                        );
                    }
                }
                MessageType::LivenessProbe => {
                    qengine_rewards_fixtures::acknowledge_probe(
                        &mut self.client,
                        self.generation,
                        frame.payload(),
                    )
                    .await?;
                }
                _ => {}
            }
        }
        assert!(
            disposition.is_none(),
            "NPC command {text:?} acknowledged without dialogue"
        );
        Ok(None)
    }

    async fn recovered_reply(&mut self, expected: &str) -> TestResult {
        let mut closed = false;
        let mut committed = false;
        loop {
            let bytes = self.read().await?.ok_or("missing recovered NPC reply")?;
            let frame = decode_wire_envelope(&bytes).map_err(debug)?;
            match frame.message_type() {
                MessageType::SnapshotChunk => {
                    let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(frame.payload())
                        .map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id == STATE_DOMAIN_NPC_CONVERSATION {
                            let value = decode_npc_conversation(domain.payload).map_err(debug)?;
                            assert_eq!(value, Default::default(), "resume closes NPC focus");
                            closed = true;
                        }
                    }
                }
                MessageType::SnapshotCommit => committed = true,
                MessageType::StateDelta => {
                    let delta = decode_state_delta(frame.payload()).map_err(debug)?;
                    if delta.domain_id == STATE_DOMAIN_NPC_CONVERSATION {
                        assert!(closed && committed, "reply follows the closed snapshot");
                        let reply = decode_npc_conversation(delta.payload).map_err(debug)?;
                        assert!(reply.open.is_none(), "the reply cannot reopen the old focus");
                        assert_eq!(reply.lines, [expected]);
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
    }

    async fn read(&mut self) -> TestResult<Option<Vec<u8>>> {
        let size = match self.client.read_u32().await {
            Ok(size) => size,
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = vec![0; usize::try_from(size)?];
        self.client.read_exact(&mut bytes).await?;
        Ok(Some(bytes))
    }

    async fn close(&mut self) -> TestResult {
        self.client.shutdown().await?;
        while let Some(bytes) = self.read().await? {
            let frame = decode_wire_envelope(&bytes).map_err(debug)?;
            if frame.message_type() == MessageType::StateDelta {
                let delta = decode_state_delta(frame.payload()).map_err(debug)?;
                if delta.domain_id == STATE_DOMAIN_NPC_CONVERSATION {
                    assert!(
                        decode_npc_conversation(delta.payload)
                            .map_err(debug)?
                            .lines
                            .is_empty()
                    );
                }
            }
        }
        Ok(())
    }
}

pub(super) async fn with_wire<T>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    conversation: impl AsyncFnOnce(&mut Wire<'_>) -> TestResult<T>,
) -> TestResult<T> {
    let before = *admitted;
    let clock = &owner.states.test_owner_time;
    clock.store(owner.owner_now().get() + 1, Ordering::Relaxed);
    let (mut server, client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let driving = async {
        let mut wire = Wire {
            client,
            npc,
            generation: before.continuity.connection_generation,
            next: before.continuity.next_command_id,
            clock,
        };
        let result = conversation(&mut wire).await?;
        wire.close().await?;
        Ok::<_, Box<dyn std::error::Error>>(result)
    };
    let (end, result) = tokio::time::timeout(
        Duration::from_secs(45),
        super::fixture::join_two(Box::pin(serving), driving),
    )
    .await?;
    let ConnectionEnd::AdmittedThenDisconnected(current) = end else {
        return Err(format!("NPC conversation ended unexpectedly: {end:?}").into());
    };
    *admitted = current;
    result
}
