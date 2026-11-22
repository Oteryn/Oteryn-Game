use super::classic_quests_fixtures::{install, progress, talk};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::RewardClaimMintRequest;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};

const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/unnatural_selection/questline";
const MISSION06: &str = "oteryn:quest-progress/quest/u8_54/unnatural_selection/mission06";
const FRONTIER: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/questline";
const TRIBAL_ADDON: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/warmaster_addon_2";

async fn supply_pot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    subtype: u8,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native item generation")?;
    let facts = |key: &str| {
        generation_item_facts(
            native,
            &TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: "definition-r1".into(),
            },
        )
        .map_err(debug)
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "Firewater fence")?
        .ok_or("Firewater fence")?
        .item_fence();
    let request = RewardClaimMintRequest {
        command: CommandRef::new(
            admitted.game_session_id,
            CommandId::new(9000 + u64::from(subtype)).map_err(debug)?,
        ),
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: format!("fixture:reward.firewater-pot-{subtype}"),
            revision_ref: "definition-r1".into(),
        },
        source_placement: format!("fixture:placement.firewater-pot-{subtype}"),
        item: facts("oteryn:item.tibia.i3465")?,
        quantity: 1,
        contents: Vec::new(),
        backpack_outputs: false,
        additional_items: Vec::new(),
        root_text: None,
        root_charges_remaining: None,
        root_fluid_subtype: Some(subtype),
        backpack: facts("oteryn:item.tibia.i2854")?,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    let weights = NativeItemWeights(native);
    let mut claim = h
        .root
        .freeze_reward_claim_mint_weighted(authority, &h.node, fence, request, &weights)
        .await
        .map_err(debug)?;
    h.root
        .commit_reward_claim_mint_weighted_noticed(
            authority, &h.node, fence, &mut claim, None, &weights,
        )
        .await
        .map_err(debug)?;
    Ok(())
}

async fn custody(h: &Harness) -> TestResult<(i64, i64, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT
         (SELECT count(*) FROM game_item_instances i JOIN game_item_fluid_states f USING(item_instance_id)
          WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i3465' AND f.fluid_subtype=1),
         (SELECT count(*) FROM game_item_instances i JOIN game_item_fluid_states f USING(item_instance_id)
          WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i3465' AND f.fluid_subtype=3),
         (SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i10198'),
         (SELECT count(*) FROM game_character_quest_states WHERE quest_key='oteryn:quest.unnatural_selection_quest'
          AND completed_character_revision IS NOT NULL),
         (SELECT count(*) FROM game_quest_npc_item_sources
          WHERE transition_key='oteryn:quest-npc/unnatural_selection/lazaran/firewater-turn-in')"
    ).fetch_one(&h.pool).await?)
}

pub(super) async fn firewater(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("unnatural-firewater PostgreSQL {version}");
    let [ulala, lazaran, cael] = install(
        owner,
        admitted,
        ["oteryn:npc.ulala", "oteryn:npc.lazaran", "oteryn:npc.cael"],
    )
    .await?;
    super::qengine_outfit_stories_fixtures::select_outfit_capabilities(owner, admitted)?;
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[
            (QUESTLINE.into(), 12),
            (FRONTIER.into(), 27),
            (TRIBAL_ADDON.into(), -1),
        ],
        151,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let id = admitted.continuity.next_command_id;
    talk(owner, admitted, ulala, id, "hi").await?;
    talk(owner, admitted, ulala, id + 1, "mission").await?;
    assert_eq!(progress(owner, admitted, authority, QUESTLINE).await?, 13);
    talk(owner, admitted, ulala, id + 2, "bye").await?;
    talk(owner, admitted, lazaran, id + 3, "hi").await?;
    talk(owner, admitted, lazaran, id + 4, "mission").await?;
    assert_eq!(progress(owner, admitted, authority, MISSION06).await?, 2);

    supply_pot(owner, admitted, h, authority, 1).await?;
    talk(owner, admitted, lazaran, id + 5, "mission").await?;
    talk(owner, admitted, lazaran, id + 6, "yes").await?;
    assert_eq!(custody(h).await?, (1, 0, 0, 0, 0));
    assert_eq!(progress(owner, admitted, authority, QUESTLINE).await?, 14);

    supply_pot(owner, admitted, h, authority, 3).await?;
    talk(owner, admitted, lazaran, id + 7, "mission").await?;
    talk(owner, admitted, lazaran, id + 8, "yes").await?;
    assert_eq!(custody(h).await?, (1, 0, 1, 1, 1));
    assert_eq!(progress(owner, admitted, authority, QUESTLINE).await?, 15);
    assert_eq!(progress(owner, admitted, authority, MISSION06).await?, 3);
    talk(owner, admitted, lazaran, id + 9, "yes").await?;
    assert_eq!(custody(h).await?, (1, 0, 1, 1, 1));

    talk(owner, admitted, lazaran, id + 10, "bye").await?;
    talk(owner, admitted, cael, id + 11, "hi").await?;
    talk(owner, admitted, cael, id + 12, "tribal crest").await?;
    talk(owner, admitted, cael, id + 13, "yes").await?;
    assert_eq!(custody(h).await?, (1, 0, 0, 1, 1));
    assert_eq!(progress(owner, admitted, authority, TRIBAL_ADDON).await?, 1);
    Ok(())
}
