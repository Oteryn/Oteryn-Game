//! Full level-qualified NPC and carried-tool programmes on the stock generation.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::reward_callbacks::{ItemAmount, RewardCallback};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::BTreeMap;
use std::sync::Arc;

#[test]
fn qengine_l11_dream_warrior_complete_level_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l11-dream-warrior-story")
}
#[test]
fn qengine_l11_issavi_complete_level_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l11-issavi-story")
}
#[test]
fn qengine_l11_makeshift_complete_apprenticeship_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l11-makeshift-story")
}

async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources")
            .fetch_one(&h.pool)
            .await?,
    )
}
async fn material(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    id: u16,
    count: u16,
    ordinal: &mut u64,
) -> TestResult {
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::ItemStackClass;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(
        native,
        &TypedDefinitionRef {
            family: "Item".into(),
            production_key: format!("oteryn:item.tibia.i{id}"),
            revision_ref: "definition-r1".into(),
        },
    )
    .map_err(debug)?;
    let maximum = match facts.stack {
        ItemStackClass::NonStackable => 1,
        ItemStackClass::Stackable { proven_maximum } => proven_maximum.unwrap_or(100),
        ItemStackClass::Unknown => return Err("unadmitted material".into()),
    };
    let mut left = u32::from(count);
    while left > 0 {
        let amount = left.min(maximum);
        super::qengine_voucher_stories::mint(owner, admitted, id, u16::try_from(amount)?, ordinal)
            .await?;
        left -= amount;
    }
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let (family, prefix, npc_key, level, items, stages): (
        &str,
        &str,
        &str,
        u32,
        [u16; 2],
        &[&str],
    ) = match name {
        "qengine-rewards-l11-dream-warrior-story" => (
            "dream_warrior",
            "dream-warrior",
            "oteryn:npc.vanys",
            250,
            [30169, 30168],
            &["base", "first", "second"],
        ),
        "qengine-rewards-l11-issavi-story" => (
            "citizen_of_issavi",
            "issavi",
            "oteryn:npc.eshaya",
            250,
            [37003, 37002],
            &["base", "first", "second"],
        ),
        "qengine-rewards-l11-makeshift-story" => (
            "makeshift_warrior",
            "makeshift",
            "oteryn:npc.gnomadness",
            100,
            [27657, 27656],
            &["start", "base", "first", "second"],
        ),
        _ => return Err("unknown level story".into()),
    };
    let all = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let mut callbacks = vec![];
    for stage in stages {
        let key = format!("oteryn:quest-reward/l11-{prefix}-{stage}");
        let callback = all
            .iter()
            .find(|c| c.key == key)
            .cloned()
            .ok_or_else(|| format!("missing level programme callback {key}"))?;
        assert_eq!(
            serde_json::to_value(&callback)?["minimum_level"],
            serde_json::json!(level),
            "chosen level threshold"
        );
        assert!(callback.requires_premium);
        callbacks.push(callback);
    }
    let find = |stage: &str| {
        callbacks
            .iter()
            .find(|c| c.key.ends_with(&format!("-{stage}")))
            .ok_or("programme stage")
    };
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut tracks = BTreeMap::new();
    let mut transitions = vec![];
    let mut parents = BTreeMap::new();
    for callback in &callbacks {
        transitions.push(
            canonical
                .transition(&callback.key)
                .cloned()
                .ok_or("native transition")?,
        );
        for state in &callback.storage {
            tracks.insert(
                state.track.clone(),
                canonical.track(&state.track).cloned().ok_or("own track")?,
            );
        }
        for parent in &callback.requires {
            tracks.insert(
                parent.track.clone(),
                canonical
                    .track(&parent.track)
                    .cloned()
                    .ok_or("parent track")?,
            );
            parents.insert(parent.track.clone(), parent.minimum);
        }
    }
    let expected_parents: BTreeMap<String, i64> = match prefix {
        "dream-warrior" => [
            (
                "oteryn:quest-progress/quest/u12_00/the_dream_courts/ward_stones/questline".into(),
                3,
            ),
            (
                "oteryn:quest-progress/quest/u12_00/the_dream_courts/main/the_summer_court".into(),
                1,
            ),
        ]
        .into(),
        "issavi" => [(
            "oteryn:quest-progress/chosen-source/kilmaresh_quest/s21".into(),
            1,
        )]
        .into(),
        _ => BTreeMap::new(),
    };
    assert_eq!(
        parents, expected_parents,
        "exact separate prerequisite owners"
    );
    let mut setup = vec![];
    for (index, (key, value)) in parents.iter().enumerate() {
        let track = tracks.get(key).ok_or("parent owner")?;
        let transition = format!("oteryn:quest-transition/qengine2-story/l11-parent-{index}");
        transitions.push(QuestTransition {
            world_change: None,
            key: transition.clone(),
            quest: track.quest.clone(),
            effects: vec![QuestEffect {
                track: key.clone(),
                from: QuestComparison::Eq(track.initial),
                effect: QuestEffectKind::Set(*value),
            }],
            completes: false,
            experience: None,
        });
        setup.push(transition);
    }
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            canonical.content_revision(),
            tracks.into_values().collect(),
            transitions,
        )
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
    owner.states.chat.admit(
        admitted.game_session_id,
        "Qualified witness",
        owner.owner_now(),
    );
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    let [npc] = super::classic_quests::install(owner, admitted, [npc_key]).await?;
    let first = if prefix == "makeshift" {
        find("start")?
    } else {
        find("base")?
    };
    let before = receipts(h).await?;
    let _ = super::qengine_rewards::wire_reward(owner, admitted, npc, first, false).await?;
    assert_eq!(
        receipts(h).await?,
        before,
        "no level or prerequisite shortcut"
    );
    for (index, key) in setup.iter().enumerate() {
        super::qengine_outfit_stories::commit_fixture(
            owner,
            admitted,
            key,
            9800 + u64::try_from(index)?,
        )
        .await?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let (result, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, first, false).await?;
    assert_eq!(
        result,
        NpcIntentDisposition::Rejected,
        "level2 cannot buy the programme with completed prerequisites"
    );
    assert_eq!(receipts(h).await?, before);
    super::qengine_outfit_stories::earn_level(owner, admitted, authority, level).await?;
    let actual_level: i64 =
        sqlx::query_scalar("SELECT level::bigint FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        actual_level,
        i64::from(level),
        "exact chosen level boundary earned through native progression"
    );
    let premium = owner.premium_coordinator.take();
    let (result, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, first, false).await?;
    assert_eq!(
        result,
        NpcIntentDisposition::Rejected,
        "minimum level is insufficient without Current Premium"
    );
    assert_eq!(receipts(h).await?, before);
    owner.premium_coordinator = premium;
    let npc_step = async |owner: &mut ComposedFreshAdmission<'_, '_, '_>,
                          admitted: &mut AdmittedSession,
                          callback: &RewardCallback|
           -> TestResult {
        let before = receipts(h).await?;
        let (result, _, command) =
            super::qengine_rewards::wire_reward(owner, admitted, npc, callback, true).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(receipts(h).await?, before + 1, "real NPC source transition");
        let replay = super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            7,
            oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
                &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                    npc_actor: npc,
                    text: callback.keywords[0].clone(),
                },
            )
            .map_err(debug)?,
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay)
                .map_err(debug)?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(receipts(h).await?, before + 1, "literal NPC replay");
        Ok(())
    };
    if prefix == "makeshift" {
        npc_step(owner, admitted, find("start")?).await?;
    }
    npc_step(owner, admitted, find("base")?).await?;
    let key = format!("oteryn:outfit.{family}");
    super::qengine_voucher_stories::mask(owner, admitted, &key, 0).await?;
    if prefix != "makeshift" {
        let base_receipts = receipts(h).await?;
        let (result, _, _) =
            super::qengine_rewards::wire_reward(owner, admitted, npc, find("base")?, false).await?;
        assert_eq!(
            result,
            NpcIntentDisposition::Rejected,
            "already-owned base cannot create a second value-only source"
        );
        assert_eq!(receipts(h).await?, base_receipts);
    }
    let mut ordinal = 800;
    if prefix != "dream-warrior" {
        material(owner, admitted, items[1], 1, &mut ordinal).await?;
        super::qengine_voucher_stories::use_native(
            owner,
            admitted,
            items[1],
            UseDisposition::Rejected,
        )
        .await?;
        assert_eq!(
            super::qengine_voucher_stories::quantity(h, items[1]).await?,
            1,
            "selected second addon requires the first and retains its token"
        );
    }
    for (index, (stage, id, needed, mask)) in [
        (
            "first",
            items[0],
            if prefix == "dream-warrior" { 5 } else { 1 },
            1,
        ),
        ("second", items[1], 1, 3),
    ]
    .into_iter()
    .enumerate()
    {
        let callback = find(stage)?;
        assert_eq!(
            callback.take,
            vec![ItemAmount {
                fluid_subtype: None,

                item: format!("oteryn:item.tibia.i{id}"),
                count: needed
            }]
        );
        if prefix == "dream-warrior" {
            if needed > 1 {
                material(owner, admitted, id, needed - 1, &mut ordinal).await?;
            }
            let held = super::qengine_voucher_stories::quantity(h, id).await?;
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
            assert_eq!(
                result,
                NpcIntentDisposition::Rejected,
                "incomplete source recipe"
            );
            assert_eq!(super::qengine_voucher_stories::quantity(h, id).await?, held);
            material(owner, admitted, id, 1, &mut ordinal).await?;
            npc_step(owner, admitted, callback).await?;
        } else {
            if super::qengine_voucher_stories::quantity(h, id).await? == 0 {
                material(owner, admitted, id, 1, &mut ordinal).await?;
            }
            super::qengine_voucher_stories::use_native(
                owner,
                admitted,
                id,
                UseDisposition::Committed,
            )
            .await?;
        }
        assert_eq!(
            super::qengine_voucher_stories::quantity(h, id).await?,
            0,
            "exact token cost {index}"
        );
        super::qengine_voucher_stories::mask(owner, admitted, &key, mask).await?;
    }
    let grants: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(grants, 3, "one base and two addons");
    let first = find("first")?;
    let count = if prefix == "dream-warrior" { 5 } else { 1 };
    material(owner, admitted, items[0], count, &mut ordinal).await?;
    if prefix == "dream-warrior" {
        let (result, _, _) =
            super::qengine_rewards::wire_reward(owner, admitted, npc, first, false).await?;
        assert_eq!(result, NpcIntentDisposition::Rejected);
    } else {
        super::qengine_voucher_stories::use_native(
            owner,
            admitted,
            items[0],
            UseDisposition::Rejected,
        )
        .await?;
    }
    assert_eq!(
        super::qengine_voucher_stories::quantity(h, items[0]).await?,
        i64::from(count),
        "owned addon retains its input"
    );
    if prefix != "makeshift" {
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_character_quest_receipts SELECT (jsonb_populate_record(NULL::game_character_quest_receipts,to_jsonb(r)||jsonb_build_object('cause_ordinal',999996,'original_character_revision',999995,'committed_character_revision',999996,'created_xact_id',pg_current_xact_id()::text))).* FROM game_character_quest_receipts r WHERE r.transition_key LIKE 'oteryn:quest-reward/l11-%-base' LIMIT 1").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_quest_reward_sources SELECT (jsonb_populate_record(NULL::game_quest_reward_sources,to_jsonb(s)||jsonb_build_object('cause_ordinal',999996,'plan',s.plan-'forbids_outfits','created_xact_id',pg_current_xact_id()::text))).* FROM game_quest_reward_sources s WHERE s.transition_key LIKE 'oteryn:quest-reward/l11-%-base' LIMIT 1").execute(&mut *tx).await?;
        let invalid =
            sqlx::query("SET CONSTRAINTS game_quest_reward_value_only_complete IMMEDIATE")
                .execute(&mut *tx)
                .await;
        assert!(
            matches!(invalid,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("23514") && e.message().contains("lacks one-shot earned predicate")),
            "unpredicated base grant cannot borrow the one-shot source arm: {invalid:?}"
        );
        tx.rollback().await?;
    }
    Ok(())
}
