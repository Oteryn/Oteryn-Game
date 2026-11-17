//! M15: the source cabinet initializes its parchment inside the same bag MINT.
use super::engitem_reward_text::{Weights, command, equip_backpack};
use crate::bestiary_postgres_harness as f;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::grizzly::GrizzlyPremiumGate;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, world_callbacks};
use std::sync::Arc;

const KEY: &str = "oteryn:quest-callback/nomads-land-sacred-casket-cabinet";

struct Premium<'a> {
    base: Weights<'a>,
    account: [u8; 16],
    current: bool,
}
impl ItemWeightLookup for Premium<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.base.generation_digest()
    }
    fn weight_of(&self, item: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        self.base.weight_of(item)
    }
    fn wheel_premium_current(&self, account: [u8; 16]) -> bool {
        self.current && account == self.account
    }
}
impl GrizzlyPremiumGate for Premium<'_> {
    async fn task_permitted(
        &self,
        _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(self.current && account == self.account)
    }
}

#[test]
fn questpc5_nomads_cabinet_mints_bound_child_text_atomically_and_replays() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "questpc5_nomads_child_text", true).await?;
                    let version: String = sqlx::query_scalar("SHOW server_version")
                        .fetch_one(&h.pool)
                        .await?;
                    eprintln!("M15_CABINET_POSTGRES_EXECUTED server={version}");
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    eprintln!(
                        "M15_CABINET_PG_COMPLETED duration={:?} success={}",
                        started.elapsed(),
                        result.is_ok() && cleanup.is_ok()
                    );
                    result?;
                    cleanup?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M15 PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let callback = world_callbacks::load()
        .map_err(f::debug)?
        .into_iter()
        .find(|row| row.key == KEY)
        .ok_or("canonical cabinet")?;
    let reward = callback.effects.item_reward.as_ref().ok_or("bag")?;
    let body = reward
        .text
        .as_ref()
        .and_then(|text| text.content(1))
        .ok_or("parchment body")?
        .to_owned();
    // Exercise the SQL source/MINT guard as well as the World callback path.
    let authored = serde_json::to_value(reward)?;
    let children = serde_json::json!([
        {"ordinal":1,"production_key":"oteryn:item.tibia.i7527","quantity":1,"text":null},
        {"ordinal":2,"production_key":"oteryn:item.tibia.i2833","quantity":1,"text":body}
    ]);
    for (candidate, expected) in [
        (children.clone(), true),
        (
            {
                let mut c = children.clone();
                c[1]["text"] = "forged text".into();
                c
            },
            false,
        ),
        (
            {
                let mut c = children.clone();
                c[0]["text"] = "unauthorized text".into();
                c
            },
            false,
        ),
        (
            {
                let mut c = children.clone();
                c[1]["quantity"] = 2.into();
                c
            },
            false,
        ),
    ] {
        let matched: bool = sqlx::query_scalar("SELECT game_quest_reward_text_matches(NULL,$1,$2)")
            .bind(candidate)
            .bind(&authored)
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(matched, expected);
    }
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            callback
                .storage
                .iter()
                .map(|s| QuestTrack {
                    key: s.track.clone(),
                    quest: callback.quest.clone(),
                    initial: s.minimum,
                    min: 0,
                    max: if s.preserve { 3 } else { 1 },
                })
                .collect(),
            vec![world_callbacks::primary_transition(&callback).map_err(f::debug)?],
        )
        .map_err(f::debug)?,
    );
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    assert!(
        h.root
            .record_content_activation(
                &crate::durability::content_activation::ContentActivationRequest {
                    world_id: f::scope()?.world_id(),
                    channel_id: crate::foundation::ChannelId::decode(&f::id(f::CHANNEL))
                        .map_err(f::debug)?,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest: native.source_digest(),
                    client_artifact_digest: pin.identity().client_artifact_digest(),
                    frame_binding_digest: pin.frame_binding().digest(),
                }
            )
            .await
            .map_err(f::debug)?
    );
    for id in [2853, 7527, 2833] {
        let facts = oteryn_gameplay_items::item_ref_admission::generation_item_facts(
            native,
            &super::engitem_reward_text::definition(id),
        )
        .map_err(|error| format!("cabinet item {id}: {error:?}"))?;
        eprintln!(
            "M15_CABINET_FACTS item={id} stack={:?} capacity={:?}",
            facts.stack, facts.container_capacity
        );
    }
    let account: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let premium = Premium {
        base: Weights(native),
        account: account.try_into().map_err(|_| "account UUID")?,
        current: true,
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let context = crate::durability::quest_world_callback::QuestWorldContext {
        map_revision: "map-1".into(),
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        placement_context: pin.frame_binding().digest().to_vec(),
        reset_epoch: 0,
        actor_placement: f::id(17),
        movement_revision: 1,
        party: Vec::new(),
    };
    let lane = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(f::scope()?)
        .await
        .map_err(f::debug)?;
    let apply = |revision, id| -> f::TestResult<_> {
        Ok(h.root
            .commit_character_world_callback_with_items_and_premium(
                &authority,
                &h.node,
                f::fence(revision)?,
                QuestTransitionRequest {
                    transition_key: KEY.into(),
                    cause: QuestCause::Use(command(id)?),
                },
                catalogue.clone(),
                &lane,
                context.clone(),
                Some(native),
                Some(QuestRewardAdmission {
                    native,
                    weights: &premium,
                    fee_payment: None,
                    actor_conditions: None,
                    carried_use: None,
                    creature_use: None,
                    peer_use: None,
                }),
                &premium,
            ))
    };
    assert!(matches!(
        apply(1, 9450)?
            .await
            .map_err(|error| format!("cabinet without backpack: {error:?}"))?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_texts")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    equip_backpack(h, &authority, native, &premium.base)
        .await
        .map_err(|error| format!("equip fixture backpack: {error}"))?;
    assert!(matches!(
        apply(1, 9451)?
            .await
            .map_err(|error| format!("cabinet grant or replay: {error:?}"))?,
        QuestTransitionOutcome::Committed(_)
    ));
    let (text, item, atomic): (String, String, bool) = sqlx::query_as("SELECT t.text,i.definition_production_key,m.root_text IS NULL AND (m.contents_plan->1->>'text')=t.text AND t.writer_character_id IS NULL AND t.text_revision=1 AND t.xmin=m.xmin AND m.xmin=q.xmin FROM game_item_texts t JOIN game_item_instances i USING(item_instance_id) JOIN game_item_nested_entries e USING(item_instance_id) JOIN game_reward_claim_mint_receipts m ON m.item_instance_id=e.parent_item_instance_id JOIN game_character_quest_receipts q ON q.character_id=m.character_id AND q.cause_id=m.game_session_id AND q.cause_ordinal=m.command_id AND q.transition_key=m.claim_production_key AND q.cause_kind='use'").fetch_one(&h.pool).await?;
    assert_eq!(text, body);
    assert_eq!(item, "oteryn:item.tibia.i2833");
    assert!(atomic);
    let recorded: Vec<u8> = sqlx::query_scalar(
        "SELECT definition_hash FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        recorded,
        catalogue
            .definition_hash(&callback.quest)
            .ok_or("bound source definition")?
            .to_vec()
    );
    let children: Vec<String> = sqlx::query_scalar("SELECT e.production_key FROM game_reward_claim_mint_receipts m CROSS JOIN LATERAL jsonb_to_recordset(m.contents_plan) AS e(production_key text,ordinal int) ORDER BY e.ordinal").fetch_all(&h.pool).await?;
    assert_eq!(
        children,
        vec!["oteryn:item.tibia.i7527", "oteryn:item.tibia.i2833"]
    );
    assert!(matches!(
        apply(1, 9451)?
            .await
            .map_err(|error| format!("cabinet grant or replay: {error:?}"))?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert!(matches!(
        apply(2, 9452)?.await.map_err(f::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_item_texts),(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1, 1));
    Ok(())
}
