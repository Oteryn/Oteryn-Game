//! Selected carried quest items must enter through native USE, not NPC text.
use super::*;

#[test]
fn questl2_selected_carried_use_native_journey() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_quest_reward(admin, "l2_carried_use", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED l2-carried-use-binding server_version={version}");
        let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
            .into_iter()
            .next()
            .ok_or("callback")?;
        let mut value = serde_json::to_value(callback)?;
        value["carried_item"] = serde_json::json!("oteryn:item.tibia.i768");
        let supported = serde_json::from_value::<RewardCallback>(value).is_ok();
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_BINDING_PROBE l2-carried-use duration={:?}",
            started.elapsed()
        );
        assert!(supported, "native carried quest item binding is absent");
        Ok(())
    })?;
    scenario("qengine-rewards-l2-engine-carried")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::durability::reward_claim_mint::{QuestCarriedUse, QuestRewardAdmission};
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
    use oteryn_protocol_oteryn::world_object::{
        UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
    };
    use std::collections::BTreeMap;
    let all = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let join = all
        .iter()
        .find(|c| c.key.ends_with("/l2-carried-join"))
        .ok_or("source admission")?;
    let document = all
        .iter()
        .find(|c| c.key.ends_with("/l2-carried-use"))
        .ok_or("source document")?;
    let canonical = owner.quest_catalogue.clone().ok_or("catalogue")?;
    let mut tracks = BTreeMap::new();
    let mut transitions = Vec::new();
    for callback in [join, document] {
        let mut keys = vec![callback.key.clone()];
        keys.extend(callback.participants.iter().enumerate().map(|(i, _)| {
            oteryn_gameplay_quests::quest::reward_callbacks::participant_key(&callback.key, i)
        }));
        for key in keys {
            let transition = canonical
                .transition(&key)
                .cloned()
                .ok_or("source transition")?;
            for key in transition
                .effects
                .iter()
                .map(|e| e.track.as_str())
                .chain(callback.requires.iter().map(|r| r.track.as_str()))
            {
                tracks.insert(
                    key.to_owned(),
                    canonical.track(key).cloned().ok_or("source track")?,
                );
            }
            transitions.push(transition);
        }
    }
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            canonical.content_revision(),
            tracks.values().cloned().collect(),
            transitions,
        )
        .map_err(debug)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_tracks")
            .fetch_one(&h.pool)
            .await?,
        0
    );
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
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Order witness", owner.owner_now());
    owner.admit_premium(super::super::fixture::id(40), admitted.game_session_id);
    let [npc] = super::super::classic_quests::install_nearby(
        owner,
        admitted,
        ["oteryn:npc.the_bone_master"],
    )
    .await?;
    assert_eq!(
        wire_grant(owner, admitted, npc, join).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(super::questl2_journeys::quantity(h, 768).await?, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_account_appearance_unlock_receipts"
        )
        .fetch_one(&h.pool)
        .await?,
        0,
        "NPC issues document; USE earns outfit"
    );
    assert_eq!(
        wire_reward(owner, admitted, npc, document, false).await?.0,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(
        super::questl2_journeys::quantity(h, 768).await?,
        1,
        "text cannot consume a carried USE binding"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            item: "oteryn:item.tibia.i768".into(),
            count: 1,
        },
        13000,
        true,
    )
    .await?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let reference = native
        .item_index()
        .ok_or("index")?
        .definition_ref("Item", "oteryn:item.tibia.i768", "definition-r1")
        .ok_or("document reference")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let weights = NativeItemWeights(native).with_wheel(owner.premium_coordinator);
    let invalid = QuestCarriedUse {
        source: super::super::fixture::id(199),
        intent: payload.clone(),
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_reward(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: document.key.clone(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(13100).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("catalogue")?,
            QuestRewardAdmission {
                native,
                weights: &weights,
                fee_payment: None,
                carried_use: Some(&invalid),
            },
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Refused(_)));
    assert_eq!(
        super::questl2_journeys::quantity(h, 768).await?,
        2,
        "wrong selected UUID leaves both documents intact"
    );
    let command = admitted.continuity.next_command_id;
    let (end, disposition, actual) = super::super::rust_lottery::exchange(
        owner,
        *admitted,
        command,
        &payload,
        Some(reference),
        None,
    )
    .await?;
    *admitted = end;
    assert_eq!(disposition, UseDisposition::Committed);
    let (end, replayed, _) =
        super::super::rust_lottery::exchange(owner, *admitted, command, &actual, None, None)
            .await?;
    *admitted = end;
    assert_eq!(replayed, UseDisposition::Committed);
    assert_eq!(super::questl2_journeys::quantity(h, 768).await?, 1);
    let (selected,proof):(sqlx::types::Uuid,Vec<u8>)=sqlx::query_as("SELECT item_instance_id,proof FROM game_quest_callback_consumptions WHERE transition_key=$1").bind(&document.key).fetch_one(&h.pool).await?;
    let proof: serde_json::Value = serde_json::from_slice(&proof)?;
    let ingress: QuestCarriedUse = serde_json::from_value(proof["carried_use"].clone())?;
    assert_eq!(ingress.source, *selected.as_bytes());
    assert_eq!(ingress.intent, actual);
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let original = CommandRef::new(
        admitted.game_session_id,
        CommandId::new(command).map_err(debug)?,
    );
    assert_eq!(
        owner
            .root
            .reconcile_quest_carried_use(
                owner.character,
                current.character_id,
                original,
                actual.clone()
            )
            .await
            .map_err(debug)?,
        Some(true)
    );
    let mut altered = actual;
    altered.push(0);
    assert_eq!(
        owner
            .root
            .reconcile_quest_carried_use(owner.character, current.character_id, original, altered)
            .await
            .map_err(debug)?,
        Some(false)
    );
    let command = admitted.continuity.next_command_id;
    let (end, result, _) = super::super::rust_lottery::exchange(
        owner,
        *admitted,
        command,
        &payload,
        Some(reference),
        None,
    )
    .await?;
    *admitted = end;
    assert_eq!(result, UseDisposition::Rejected);
    assert_eq!(
        super::questl2_journeys::quantity(h, 768).await?,
        1,
        "owned outfit cannot consume the duplicate document"
    );
    let keys: Vec<String> = sqlx::query_scalar(
        "SELECT unlock_key FROM game_account_appearance_unlock_receipts ORDER BY unlock_key",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(keys, ["oteryn:outfit.brotherhood/quest-grant/0"]);
    Ok(())
}
