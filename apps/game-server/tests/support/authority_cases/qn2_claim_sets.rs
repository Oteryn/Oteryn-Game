use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::completion_sets;
use std::sync::Arc;

#[test]
fn qn2_unordered_claims_durably_complete_on_last_chest() -> TestResult {
    claim_set_scenario(
        "qn2_claim_sets",
        "oteryn:quest.bear_room_quest",
        &["bear_chest3", "bear_chest1", "bear_chest2"],
    )
}

#[test]
fn qn2_behemoth_claims_complete_only_after_the_fourth_chest() -> TestResult {
    claim_set_scenario(
        "qn2_behemoth",
        "oteryn:quest.behemoth_quest",
        &[
            "behemothquest4",
            "behemothquest2",
            "behemothquest1",
            "behemothquest3",
        ],
    )
}

#[test]
fn qn2_circle_room_claims_complete_only_after_both_chests() -> TestResult {
    claim_set_scenario(
        "qn2_circle_room",
        "oteryn:quest.circle_room_quest",
        &["circleroom2", "circleroom1"],
    )
}

fn claim_set_scenario(
    tag: &'static str,
    quest: &'static str,
    markers: &'static [&'static str],
) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime().map_err(debug)?.block_on(async move {
                let harness = Harness::create(admin, tag, true).await.map_err(debug)?;
                eprintln!("QN2_POSTGRES_READY");
                let seal = harness.recovery.seal_current().map_err(debug)?;
                let authority = harness
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let catalogue = Arc::new(
                    oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                        .map_err(debug)?
                        .catalogue()
                        .clone(),
                );
                let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
                let world = crate::bestiary_postgres_harness::scope().map_err(debug)?.world_id();
                let native = (quest == "oteryn:quest.circle_room_quest").then(|| {
                    oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
                        world,
                        None,
                    )
                });
                if let Some((controller, _)) = &native {
                    let native = controller.active().and_then(|g| g.native_gameplay()).ok_or("native claim items")?;
                    super::engitem_reward_text::equip_backpack(
                        &harness, &authority, native, &super::engitem_reward_text::Weights(native),
                    ).await.map_err(debug)?;
                }
                for (index, marker) in markers.iter().enumerate() {
                    let mut request = QuestTransitionRequest {
                        transition_key: format!("oteryn:quest-transition/kv/{marker}/chest"),
                        cause: QuestCause::Command(CommandRef::new(
                            GameSessionId::decode(&id(50)).map_err(debug)?,
                            CommandId::new(100 + index as u64).map_err(debug)?,
                        )),
                    };
                    if let Some(claim) = completion_sets::required_mint_claim(&request.transition_key)? {
                        let native = native.as_ref().and_then(|(c, _)| c.active())
                            .and_then(|g| g.native_gameplay()).ok_or("native claim items")?;
                        mint_claim(&harness, &authority, native, claim, &request).await.map_err(debug)?;
                        let QuestCause::Command(command) = request.cause else {
                            return Err("claim command".into());
                        };
                        request.cause = QuestCause::ClaimObligation(command);
                    }
                    let outcome = harness
                        .root
                        .commit_character_quest_transition(
                            &authority,
                            &harness.node,
                            fence(index as u64 + 1).map_err(debug)?,
                            request.clone(),
                            Arc::clone(&catalogue),
                        )
                        .await
                        .map_err(debug)?;
                    let QuestTransitionOutcome::Committed(receipt) = outcome else {
                        return Err(format!("{outcome:?}"));
                    };
                    assert_eq!(receipt.completes, index + 1 == markers.len());
                    let replay = harness
                        .root
                        .commit_character_quest_transition(
                            &authority,
                            &harness.node,
                            fence(index as u64 + 1).map_err(debug)?,
                            request,
                            Arc::clone(&catalogue),
                        )
                        .await
                        .map_err(debug)?;
                    assert!(matches!(
                        replay,
                        QuestTransitionOutcome::AlreadyCommitted(_)
                    ));
                    let copy = harness
                        .root
                        .read_character_quest_state(&authority, character)
                        .await
                        .map_err(debug)?;
                    assert_eq!(copy.states()[quest].completed, index + 1 == markers.len());
                }
                drop(authority);
                drop(seal);
                harness.cleanup().await.map_err(debug)?;
                Ok(())
            })
        })?
        .join()
        .map_err(|_| "qn2 test panicked")?
        .map_err(Into::into)
}

async fn mint_claim(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    claim: &str,
    quest: &QuestTransitionRequest,
) -> TestResult {
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::reward_claim_mint::{RewardClaimMintOutcome, RewardClaimMintRequest};
    use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
    let item = match claim {
        "oteryn:reward-claim.kv.circleroom1" => 3279,
        "oteryn:reward-claim.kv.circleroom2" => 3323,
        _ => return Err("unexpected native claim fixture".into()),
    };
    assert_eq!(
        completion_sets::mint_transition_for_claim(claim)?,
        Some(quest.transition_key.as_str())
    );
    let QuestCause::Command(command) = quest.cause else {
        return Err("claim command".into());
    };
    let request = RewardClaimMintRequest {
        backpack_outputs: false,
        root_charges_remaining: None,
        root_fluid_subtype: None,
        command,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: claim.into(),
            revision_ref: "reward-claim-r1".into(),
        },
        source_placement: format!(
            "oteryn:placement/{}/0",
            claim.strip_prefix("oteryn:").ok_or("claim namespace")?
        ),
        item: generation_item_facts(native, &super::engitem_reward_text::definition(item))
            .map_err(debug)?,
        quantity: 1,
        contents: Vec::new(),
        additional_items: Vec::new(),
        root_text: None,
        backpack: generation_item_facts(native, &super::engitem_reward_text::definition(2854))
            .map_err(debug)?,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: Some(quest.transition_key.clone()),
        key_number: None,
    };
    let before = h.root_revision().await?;
    let fence = crate::bestiary_postgres_harness::item_fence()?;
    let weights = super::engitem_reward_text::Weights(native);
    let mut candidate = h
        .root
        .freeze_reward_claim_mint_weighted(authority, &h.node, fence, request, &weights)
        .await
        .map_err(debug)?;
    let (outcome, _) = h
        .root
        .commit_reward_claim_mint_weighted_noticed(
            authority,
            &h.node,
            fence,
            &mut candidate,
            None,
            &weights,
        )
        .await
        .map_err(debug)?;
    let RewardClaimMintOutcome::Committed(receipt) = outcome else {
        return Err(format!("native claim MINT: {outcome:?}").into());
    };
    let (replay, _) = h
        .root
        .commit_reward_claim_mint_weighted_noticed(
            authority,
            &h.node,
            fence,
            &mut candidate,
            None,
            &weights,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(replay, RewardClaimMintOutcome::AlreadyCommitted(ref old) if old == &receipt));
    assert_eq!(
        h.root_revision().await?,
        before,
        "MINT and its replay do not advance the quest revision"
    );
    Ok(())
}
