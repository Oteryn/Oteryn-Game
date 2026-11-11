//! Automatic STEP input uses the existing atomic carried-input and quest receipt writer.
#![cfg(feature = "test-support")]
use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
use oteryn_gameplay_domain::domain;
use oteryn_gameplay_durability::durability;
use oteryn_gameplay_entities as foundation;
#[allow(dead_code, reason = "reuse canonical PostgreSQL harness")]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[allow(dead_code, reason = "reuse native backpack and reward mint fixtures")]
#[path = "../../../../apps/game-server/tests/support/authority_cases/engitem_reward_text.rs"]
mod reward_fixture;
use bestiary_postgres_harness as f;
use durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use std::sync::Arc;

struct Premium {
    account: [u8; 16],
    allowed: bool,
}
impl durability::quest_state::grizzly::GrizzlyPremiumGate for Premium {
    async fn task_permitted(
        &self,
        _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: [u8; 16],
    ) -> Result<bool, durability::DurabilityError> {
        Ok(self.allowed && self.account == account)
    }
}

#[test]
fn m01c_paid_step_debits_exact_carrot_once_with_premium_postgres() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "questpc3_m01c_paid_step", true).await?;
                    let result = paid_step(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M01C_PAID_STEP POSTGRES_CASE_COMPLETED duration_ms={} success=true",
                        started.elapsed().as_millis()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "paid STEP PostgreSQL thread")?
        .map_err(Into::into)
}

async fn paid_step(h: &f::Harness) -> f::TestResult {
    use durability::item_mint::TypedDefinitionRef;
    use durability::reward_claim_mint::{RewardClaimMintOutcome, RewardClaimMintRequest};
    use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        version,
        oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM
    );
    eprintln!("M01C_PAID_STEP POSTGRES_CONNECTED version={version}");
    reject_unqualified_step_proofs(h).await?;
    // The finite production union has not published carrot 3595 yet. The fixture
    // activates its existing qualified source record verbatim, without invented facts.
    use oteryn_gameplay_content::content::native_gameplay::{
        NativeGameplayInput, PinnedGameplayBytes,
    };
    use sha2::Digest;
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/spells.manifest.json");
    let mut input = NativeGameplayInput::from_manifest(&manifest).map_err(f::debug)?;
    let mut profiles: serde_json::Value = serde_json::from_slice(
        &input
            .item_profiles
            .as_ref()
            .ok_or("native item profiles")?
            .bytes,
    )?;
    let source: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../tools/content-schema/native-gameplay/quest-batch3-item-profiles.json"
    ))?;
    let carrot = source["records"]
        .as_array()
        .ok_or("source item records")?
        .iter()
        .find(|record| {
            record["production_definition"]["production_key"] == "oteryn:item.tibia.i3595"
        })
        .ok_or("qualified source carrot")?
        .clone();
    let records = profiles["records"]
        .as_array_mut()
        .ok_or("native item records")?;
    if !records.iter().any(|record| {
        record["production_definition"]["production_key"] == "oteryn:item.tibia.i3595"
    }) {
        records.push(carrot);
    }
    let bytes = serde_json::to_vec(&profiles)?;
    let sha256 = sha2::Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    input.item_profiles = Some(PinnedGameplayBytes { bytes, sha256 });
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::try_activate_input(
            f::scope()?.world_id(),
            &input,
        )
        .map_err(f::debug)?;
    let native = controller
        .active()
        .and_then(|a| a.native_gameplay())
        .ok_or("native content")?;
    let frame = pin.frame_binding().digest();
    let client = pin.identity().client_artifact_digest();
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).bind(native.source_digest().as_slice()).bind(client.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    reward_fixture::equip_backpack(h, &authority, native, &reward_fixture::Weights(native)).await?;
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../quests/src/quest/world_callbacks/paid_step_fixture.json"
    ))?;
    let callback = document["callbacks"][0].clone();
    let bindings = serde_json::json!({"schema":1,"quests":[{
        "quest":callback["quest"],"recipe_sha256":"11".repeat(32),
        "tracks":[{"key":callback["storage"][0]["track"],"initial":0,"minimum":0,"maximum":1}],
        "stages":[{"key":"entry","completes":false,"trigger":{"kind":"step_in","callback":callback}}],
        "sources":["fixture:registered-paid-step-writer"]
    }]});
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::authored_bindings::Bindings::parse(&serde_json::to_vec(
            &bindings,
        )?)
        .map_err(f::debug)?
        .attach(
            &oteryn_gameplay_quests::quest::QuestStateCatalogue::empty("content-1")
                .ok_or("catalogue")?,
        )
        .map_err(f::debug)?,
    );
    let account: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let mut premium = Premium {
        account: account.try_into().map_err(|_| "account UUID")?,
        allowed: true,
    };
    let sequencer = durability::character_revision_sequencer::CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(f::fence(1)?.character_id).await;
    let lane = durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(f::scope()?)
        .await
        .map_err(f::debug)?;
    let context = durability::quest_world_callback::QuestWorldContext {
        map_revision: "map-1".into(),
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        placement_context: frame.to_vec(),
        reset_epoch: 0,
        actor_placement: f::id(17),
        movement_revision: 1,
        party: Vec::new(),
    };
    let request = |number| -> f::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: "oteryn:quest-callback/m01c-paid-step-fixture".into(),
            cause: QuestCause::Step(reward_fixture::command(number)?),
        })
    };
    let empty = slot
        .commit_world_callback_with_items(
            &h.root,
            &authority,
            &h.node,
            f::fence(1)?,
            request(22000)?,
            Arc::clone(&catalogue),
            &lane,
            context.clone(),
            Some(native),
            None,
            &premium,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(empty, QuestTransitionOutcome::Refused(_)),
        "missing carrot: {empty:?}"
    );
    let mint = RewardClaimMintRequest {
        backpack_outputs: false,
        root_charges_remaining: None,
        root_fluid_subtype: None,
        command: reward_fixture::command(22001)?,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: "fixture:m01c-carrots".into(),
            revision_ref: "definition-r1".into(),
        },
        source_placement: "fixture:m01c-carrot-box".into(),
        item: generation_item_facts(native, &reward_fixture::definition(3595)).map_err(f::debug)?,
        quantity: 2,
        contents: Vec::new(),
        additional_items: Vec::new(),
        root_text: None,
        backpack: generation_item_facts(native, &reward_fixture::definition(2854))
            .map_err(f::debug)?,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    let mut candidate = h
        .root
        .freeze_reward_claim_mint(&authority, &h.node, f::item_fence()?, mint)
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        h.root
            .commit_reward_claim_mint(&authority, &h.node, f::item_fence()?, &mut candidate)
            .await
            .map_err(f::debug)?,
        RewardClaimMintOutcome::Committed(_)
    ));
    premium.allowed = false;
    let denied = slot
        .commit_world_callback_with_items(
            &h.root,
            &authority,
            &h.node,
            f::fence(1)?,
            request(22002)?,
            Arc::clone(&catalogue),
            &lane,
            context.clone(),
            Some(native),
            None,
            &premium,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(denied, QuestTransitionOutcome::Refused(_)),
        "free account: {denied:?}"
    );
    assert_eq!(carrots(h).await?, 2);
    premium.allowed = true;
    for replay in [false, true] {
        let outcome = slot
            .commit_world_callback_with_items(
                &h.root,
                &authority,
                &h.node,
                f::fence(1)?,
                request(22003)?,
                Arc::clone(&catalogue),
                &lane,
                context.clone(),
                Some(native),
                None,
                &premium,
            )
            .await
            .map_err(f::debug)?;
        assert!(
            if replay {
                matches!(outcome, QuestTransitionOutcome::AlreadyCommitted(_))
            } else {
                matches!(outcome, QuestTransitionOutcome::Committed(_))
            },
            "replay={replay}: {outcome:?}"
        );
        assert_eq!(carrots(h).await?, 1);
    }
    let progress:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/m01c-paid-step-fixture'").bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(progress, 1);
    Ok(())
}
async fn carrots(h: &f::Harness) -> f::TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i3595'")
        .bind(f::id(f::CHARACTER).as_slice()).bind(f::id(f::WORLD).as_slice()).fetch_one(&h.pool).await?)
}

// Probe the new guard separately: every existing mutation/custody/receipt arm
// remains mandatory, and STEP opt-in alone cannot authorize a different debit.
async fn reject_unqualified_step_proofs(h: &f::Harness) -> f::TestResult {
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../quests/src/quest/world_callbacks/paid_step_fixture.json"
    ))?;
    let original = document["callbacks"][0].clone();
    for mutation in 0..10 {
        let mut callback = original.clone();
        let mut quantity = 1;
        let mut receipt = serde_json::json!({"cause_kind":"step","quest_key":original["quest"]});
        match mutation {
            0 => {}
            1 => callback["item_on_target"]["on_step"] = false.into(),
            2 => callback["selector"]["native"] = serde_json::Value::Null,
            3 => quantity = 2,
            4 => callback["item_on_target"]["item"] = "oteryn:item.tibia.i3594".into(),
            5 => callback["item_on_target"]["reusable"] = true.into(),
            6 => callback["item_on_target"]["target_replacement"] = 1949.into(),
            7 => receipt["quest_key"] = "oteryn:quest.other".into(),
            8 => callback["requires_world_admission"] = false.into(),
            _ => callback["trigger"] = "use".into(),
        }
        let consumption = serde_json::json!({"transition_key":original["key"],"quantity":quantity,
            "before_item":{"definition_production_key":"oteryn:item.tibia.i3595"}});
        let proof = serde_json::to_vec(&serde_json::json!({"callback":callback}))?;
        let accepted: bool = sqlx::query_scalar("SELECT coalesce(game_quest_callback_paid_step_matches(jsonb_populate_record(NULL::game_quest_callback_consumptions,$1::jsonb || jsonb_build_object('proof',$3::bytea)),jsonb_populate_record(NULL::game_character_quest_receipts,$2::jsonb)),false)")
            .bind(consumption).bind(receipt).bind(proof).fetch_one(&h.pool).await?;
        assert_eq!(
            accepted,
            mutation == 0,
            "paid STEP proof mutation={mutation}"
        );
    }
    Ok(())
}
