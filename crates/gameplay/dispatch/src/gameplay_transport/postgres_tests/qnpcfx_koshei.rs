//! Koshei's source payment uses the existing fenced fee/input/quest participants.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const PHASE: &str = "oteryn:quest-progress/qnpcfx/koshei/amulet";
const CLOCK: &str = "oteryn:quest-progress/qnpcfx/koshei/amulet_started_at";

#[test]
fn qnpcfx_koshei_paid_pieces_atomic_replay_and_clock() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-qnpcfx-koshei")
}

#[test]
fn qnpcfx_koshei_mixed_carried_bank_payment_atomic() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-qnpcfx-koshei-bank")
}

async fn snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_array((SELECT jsonb_agg(to_jsonb(t) ORDER BY character_id,track_key) FROM game_character_quest_tracks t),(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),(SELECT count(*) FROM game_item_fee_burns),(SELECT count(*) FROM game_quest_reward_sources),(SELECT jsonb_agg(to_jsonb(b) ORDER BY account_id,world_id) FROM game_account_bank_balances b),(SELECT count(*) FROM game_account_bank_entries))::text")
        .fetch_one(&h.pool).await?)
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    callback: &RewardCallback,
) -> TestResult<NpcIntentDisposition> {
    let (result, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
    Ok(result)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    from_bank: bool,
) -> TestResult {
    if from_bank {
        // BANK-FEE-0 admits Account bank debits only after the production Type2 V2 activation.
        sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
            .execute(&h.pool)
            .await?;
    }
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load().map_err(debug)?;
    let start = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/qnpcfx-koshei-start")
        .ok_or("start")?;
    let collect = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/qnpcfx-koshei-collect")
        .ok_or("collect")?;
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    };
    let fixture_now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut transitions = [start, collect]
        .into_iter()
        .map(|c| canonical.transition(&c.key).cloned().ok_or("transition"))
        .collect::<Result<Vec<_>, _>>()?;
    transitions.push(QuestTransition {
        world_change: None,
        key: "oteryn:quest-transition/qnpcfx-clock-fixture".into(),
        quest: start.quest.clone(),
        effects: vec![QuestEffect {
            track: CLOCK.into(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Set(fixture_now - 86401),
        }],
        completes: false,
        experience: None,
    });
    let tracks = [PHASE, CLOCK]
        .into_iter()
        .map(|k| canonical.track(k).cloned().ok_or("track"))
        .collect::<Result<Vec<_>, _>>()?;
    owner.quest_catalogue = Some(std::sync::Arc::new(
        QuestStateCatalogue::new(canonical.content_revision(), tracks, transitions)
            .map_err(debug)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = super::classic_quests::install(owner, admitted, [start.npc.as_str()]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let at = Position {
        x: 33293,
        y: 31677,
        floor: -15,
    };
    // Missing funds and each incomplete material set must leave every value untouched.
    let mut occurrence = 51000;
    for item in &start.take {
        let before = snapshot(h).await?;
        assert_eq!(
            talk(owner, admitted, npc, start).await?,
            NpcIntentDisposition::Rejected
        );
        assert_eq!(snapshot(h).await?, before);
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: at,
                item: item.item.clone(),
                count: 1,
            },
            occurrence,
            true,
        )
        .await?;
        occurrence += 1;
    }
    let before = snapshot(h).await?;
    assert_eq!(
        talk(owner, admitted, npc, start).await?,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(
        snapshot(h).await?,
        before,
        "all pieces without funds cannot be consumed"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    if from_bank {
        let account: sqlx::types::Uuid = sqlx::query_scalar(
            "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        super::guild_bank::bank_seed::balance(
            h,
            *account.as_bytes(),
            *fence.character_id.as_bytes(),
            4000,
            180,
        )
        .await?;
    }
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: at,
            item: "oteryn:item.tibia.i3035".into(),
            count: if from_bank { 10 } else { 60 },
        },
        occurrence,
        true,
    )
    .await?;
    let (start_result, _, start_command) =
        super::qengine_rewards::wire_reward(owner, admitted, npc, start, false).await?;
    assert_eq!(start_result, NpcIntentDisposition::Ok);
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, PHASE).await?,
        1
    );
    let stamp = super::classic_quests::progress(owner, admitted, authority, CLOCK).await?;
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    assert!(
        (fixture_now..=now).contains(&stamp),
        "stamp comes from transaction time"
    );
    let fee:(i64,i64,i64) = sqlx::query_as("SELECT count(*),coalesce(sum(fee_gold_units),0)::bigint,coalesce(sum(bank_debit_gold_units),0)::bigint FROM game_item_fee_burns WHERE cause_kind=18").fetch_one(&h.pool).await?;
    assert_eq!(fee, (1, 5000, if from_bank { 4000 } else { 0 }));
    let coins:i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3035'").fetch_one(&h.pool).await?;
    assert_eq!(
        coins,
        if from_bank { 0 } else { 10 },
        "spend carried coins first and retain exact change"
    );
    if from_bank {
        let remaining: i64 =
            sqlx::query_scalar("SELECT balance FROM game_account_bank_balances LIMIT 1")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(remaining, 0);
    }
    let pieces:i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=ANY($1)")
        .bind(start.take.iter().map(|i| i.item.as_str()).collect::<Vec<_>>()).fetch_one(&h.pool).await?;
    assert_eq!(pieces, 0);
    let before = snapshot(h).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
        .with_wheel(owner.premium_coordinator);
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let replay = slot
        .commit_quest_reward(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: start.key.clone(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(start_command).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("catalogue")?,
            crate::durability::reward_claim_mint::QuestRewardAdmission {
                native,
                weights: &weights,
                fee_payment: None,
                actor_conditions: None,
                carried_use: None,
                creature_use: None,
                peer_use: None,
            },
            &owner.premium_coordinator,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(replay, QuestTransitionOutcome::AlreadyCommitted(_)),
        "bound payment replay: {replay:?}"
    );
    drop(slot);
    assert_eq!(
        talk(owner, admitted, npc, collect).await?,
        NpcIntentDisposition::Rejected,
        "24h are not elapsed"
    );
    assert_eq!(
        snapshot(h).await?,
        before,
        "repeated start/early collection cannot charge again"
    );
    // Advance only the retained clock through the existing fenced Quest writer.
    let catalogue = owner.quest_catalogue.clone().ok_or("catalogue")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: "oteryn:quest-transition/qnpcfx-clock-fixture".into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(59000).map_err(debug)?,
                )),
            },
            std::sync::Arc::clone(&catalogue),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner.quest_catalogue = Some(catalogue);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        talk(owner, admitted, npc, collect).await?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, PHASE).await?,
        2
    );
    let amulets:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i7532'").fetch_one(&h.pool).await?;
    assert_eq!(amulets, 1);
    let before = snapshot(h).await?;
    assert_eq!(
        talk(owner, admitted, npc, collect).await?,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(snapshot(h).await?, before);
    Ok(())
}
