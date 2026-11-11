//! Endgame accessories compose after the original parent earns its base.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) async fn accessories(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    family: &str,
) -> TestResult {
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let mut stages = Vec::new();
    for stage in ["first", "second"] {
        let key = format!("oteryn:quest-reward/l13-{family}-{stage}");
        stages.push(
            callbacks
                .iter()
                .find(|c| c.key == key)
                .ok_or_else(|| format!("missing endgame callback {key}"))?,
        );
    }
    let key = format!("oteryn:outfit.{family}");
    let native_base = if family == "revenant" {
        "oteryn:appearance-grant/quest/soul-war/revenant-base"
    } else {
        "oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base"
    };
    let original_base: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT grant_occurrence_id FROM game_account_appearance_unlocks WHERE unlock_key=$1",
    )
    .bind(native_base)
    .fetch_one(&h.pool)
    .await?;
    let (items, bits) = if family == "revenant" {
        ([34075, 34076], [2, 1])
    } else {
        ([43899, 43900], [1, 2])
    };
    for (index, callback) in stages.iter().enumerate() {
        assert_eq!(
            callback.carried_item.as_deref(),
            Some(format!("oteryn:item.tibia.i{}", items[index]).as_str())
        );
        assert_eq!(callback.minimum_level, Some(250));
        assert!(callback.requires_premium);
        assert_eq!(callback.requires_outfits, vec![key.clone()]);
        assert_eq!(
            callback.outfit.as_ref().ok_or("outfit")?.addons,
            bits[index]
        );
    }
    admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    assert!(
        owner.premium_coordinator.is_some(),
        "Current Premium authority fixture"
    );
    owner.admit_premium(id(40), admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    super::qengine_voucher_stories::mask(owner, admitted, &key, 0).await?;
    super::qengine_outfit_stories::earn_level(owner, admitted, authority, 250).await?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    // Parent-only fixtures have no carried-USE admission fence until this phase.
    admitted.item_fence = Some(fence.item_fence());
    let has_bag: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_slots")
        .fetch_one(&h.pool)
        .await?;
    if has_bag == 0 {
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                owner.character,
                owner.holder,
                &fence.item_fence(),
                owner
                    .active_generation
                    .ok_or("generation")?
                    .identity()
                    .server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        h.root
            .grant_starter_items(
                owner.character,
                owner.holder,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &crate::durability::starter_grant::StarterTemplate::main(),
            )
            .await
            .map_err(debug)?;
    }
    let mut ordinal = 99000;
    // A second accessory alone cannot bypass the selected order; no item is lost.
    super::qengine_voucher_stories::mint(owner, admitted, items[1], 1, &mut ordinal).await?;
    super::qengine_voucher_stories::use_native(owner, admitted, items[1], UseDisposition::Rejected)
        .await?;
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, items[1]).await?,
        1
    );
    for _ in 0..2 {
        super::qengine_voucher_stories::mint(owner, admitted, items[0], 1, &mut ordinal).await?;
    }
    let premium = owner.premium_coordinator.take();
    super::qengine_voucher_stories::use_native(owner, admitted, items[0], UseDisposition::Rejected)
        .await?;
    owner.premium_coordinator = premium;
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, items[0]).await?,
        2
    );
    for (index, item) in items.into_iter().enumerate() {
        super::qengine_voucher_stories::use_native(
            owner,
            admitted,
            item,
            UseDisposition::Committed,
        )
        .await?;
        assert_eq!(
            super::qengine_voucher_stories::quantity(h, item).await?,
            if index == 0 { 1 } else { 0 }
        );
        super::qengine_voucher_stories::mask(
            owner,
            admitted,
            &key,
            if index == 0 { bits[0] } else { 3 },
        )
        .await?;
    }
    super::qengine_voucher_stories::use_native(owner, admitted, items[0], UseDisposition::Rejected)
        .await?;
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, items[0]).await?,
        1
    );
    // The accepted generic callback contract includes its canonical base with
    // addons; retain the earlier native receipt as a separate compatible source.
    let canonical: Vec<String> = (0..=2)
        .map(|bit| format!("{key}/quest-grant/{bit}"))
        .collect();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key=ANY($1::text[])",
    )
    .bind(&canonical)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 3, "three exact canonical outfit facts");
    let mut expected = canonical;
    expected.push(native_base.into());
    expected.sort();
    let actual: Vec<String> = sqlx::query_scalar(
        "SELECT unlock_key FROM game_account_appearance_unlocks ORDER BY unlock_key LIMIT 5",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        actual, expected,
        "only the native base and canonical base/addons"
    );
    let retained_base: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT grant_occurrence_id FROM game_account_appearance_unlocks WHERE unlock_key=$1",
    )
    .bind(native_base)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        retained_base, original_base,
        "native base provenance remains immutable"
    );
    achievement(
        h,
        if family == "revenant" {
            "oteryn:achievement/unleash_the_beast"
        } else {
            "oteryn:achievement/fully_decayed"
        },
    )
    .await?;
    select_full(owner, admitted, &key).await?;
    Ok(())
}

#[test]
fn qengine_l13_rift_parent_and_tokens() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l13-rift-story")
}

pub(super) async fn rift(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    _authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTransition};
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let mut stages = Vec::new();
    for stage in ["base", "first", "second"] {
        let key = format!("oteryn:quest-reward/l13-rift-{stage}");
        let callback = callbacks
            .iter()
            .find(|c| c.key == key)
            .ok_or_else(|| format!("missing Rift callback {key}"))?;
        assert!(
            callback.minimum_level.is_none(),
            "chosen Rift minimum is zero"
        );
        if stage == "base" {
            assert_eq!(
                serde_json::to_value(callback)?["requires_completed_quests"],
                serde_json::json!(["oteryn:quest.ferumbras_ascension_quest"])
            );
        }
        stages.push(callback);
    }
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut transitions = Vec::new();
    for callback in &stages {
        transitions.push(
            canonical
                .transition(&callback.key)
                .cloned()
                .ok_or("transition")?,
        );
    }
    let parent = "oteryn:quest-transition/qengine2-story/l13-ferumbras-complete";
    transitions.push(QuestTransition {
        world_change: None,
        key: parent.into(),
        quest: "oteryn:quest.ferumbras_ascension_quest".into(),
        effects: vec![],
        completes: true,
        experience: None,
    });
    owner.quest_catalogue = Some(std::sync::Arc::new(
        QuestStateCatalogue::new(canonical.content_revision(), vec![], transitions)
            .map_err(debug)?,
    ));
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    assert!(
        owner.premium_coordinator.is_some(),
        "Current Premium authority fixture"
    );
    owner.admit_premium(id(40), admitted.game_session_id);
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.cledwyn"]).await?;
    let result =
        super::qengine_rewards::wire_reward(owner, admitted, npc, stages[0], false).await?;
    assert_eq!(
        result.0,
        NpcIntentDisposition::Rejected,
        "parent must actually be completed"
    );
    let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(grants, 0);
    super::qengine_outfit_stories::commit_fixture(owner, admitted, parent, 9800).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let result = super::qengine_rewards::wire_reward(owner, admitted, npc, stages[0], true).await?;
    assert_eq!(result.0, NpcIntentDisposition::Ok);
    super::qengine_voucher_stories::mask(owner, admitted, "oteryn:outfit.rift_warrior", 0).await?;
    let mut ordinal = 99000;
    super::qengine_voucher_stories::mint(owner, admitted, 22516, 99, &mut ordinal).await?;
    assert_eq!(
        super::qengine_rewards::wire_reward(owner, admitted, npc, stages[1], false)
            .await?
            .0,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, 22516).await?,
        99
    );
    super::qengine_voucher_stories::mint(owner, admitted, 22516, 2, &mut ordinal).await?;
    assert_eq!(
        super::qengine_rewards::wire_reward(owner, admitted, npc, stages[2], false)
            .await?
            .0,
        NpcIntentDisposition::Rejected,
        "selected order"
    );
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, 22516).await?,
        101
    );
    let result = super::qengine_rewards::wire_reward(owner, admitted, npc, stages[1], true).await?;
    assert_eq!(result.0, NpcIntentDisposition::Ok);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 22516).await?, 1);
    let replay = super::quest_world_callbacks::wire(
        owner,
        admitted,
        result.2,
        7,
        oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
            &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                npc_actor: npc,
                text: stages[1].keywords[0].clone(),
            },
        )
        .map_err(debug)?,
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    super::qengine_voucher_stories::mint(owner, admitted, 22516, 99, &mut ordinal).await?;
    assert_eq!(
        super::qengine_rewards::wire_reward(owner, admitted, npc, stages[2], true)
            .await?
            .0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(super::qengine_voucher_stories::quantity(h, 22516).await?, 0);
    super::qengine_voucher_stories::mask(owner, admitted, "oteryn:outfit.rift_warrior", 3).await?;
    let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(grants, 3);
    achievement(h, "oteryn:achievement/rift_warrior").await?;
    select_full(owner, admitted, "oteryn:outfit.rift_warrior").await?;
    Ok(())
}

async fn select_full(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    key: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::appearance::{Change, ChangeResult, Selection};
    let observation = owner
        .appearance_observation(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            vec![],
        )
        .await
        .ok_or("normal appearance")?;
    let change = Change {
        context: observation.catalogue.context,
        expected_root_revision: observation.catalogue.root_revision,
        prior: observation.catalogue.selected,
        selection: Selection {
            outfit_key: key.into(),
            colours: [0; 4],
            addons: 3,
            mount_key: None,
        },
        content_revision: owner
            .appearance_context()
            .ok_or("cosmetics")?
            .1
            .content_revision()
            .into(),
    };
    let next = admitted.continuity.next_command_id;
    let result = super::quest_world_callbacks::wire(
        owner,
        admitted,
        next,
        25,
        change.encode().map_err(debug)?,
    )
    .await?;
    assert!(matches!(
        ChangeResult::decode(&result).map_err(debug)?,
        ChangeResult::Committed(_)
    ));
    Ok(())
}

async fn achievement(h: &Harness, key: &str) -> TestResult {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind(key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 1, "source completion achievement {key}");
    Ok(())
}

pub(super) async fn current_premium(
    h: &Harness,
) -> TestResult<crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator> {
    use crate::gameplay_transport::spell_premium_coordinator::{
        SpellPremiumCoordinator, purchase_fixture,
    };
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    Ok(SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    ))
}
