//! Order documents must enter through their NPC source and ordinary carried USE.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

#[test]
fn qengine_l10_brotherhood_complete_document_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l10-documents-brotherhood-story")
}
#[test]
fn qengine_l10_nightmare_complete_document_story() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l10-documents-nightmare-story")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let family = if name.contains("brotherhood") {
        "brotherhood"
    } else {
        "nightmare"
    };
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    for stage in [
        "join",
        "officer",
        "leader",
        "use-follower",
        "use-officer",
        "use-leader",
    ] {
        let key = format!("oteryn:quest-reward/l10-{family}-{stage}");
        assert!(
            callbacks.iter().any(|c| c.key == key),
            "missing source document callback {key}"
        );
    }
    use super::fixture::debug;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    };
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    let callbacks = callbacks
        .into_iter()
        .filter(|c| {
            c.key
                .starts_with(&format!("oteryn:quest-reward/l10-{family}-"))
        })
        .collect::<Vec<_>>();
    let canonical = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut tracks = BTreeMap::new();
    let mut transitions = vec![];
    for callback in &callbacks {
        transitions.push(
            canonical
                .transition(&callback.key)
                .cloned()
                .ok_or("source transition")?,
        );
        for state in &callback.storage {
            tracks.insert(
                state.track.clone(),
                canonical
                    .track(&state.track)
                    .cloned()
                    .ok_or("source track")?,
            );
        }
        for required in &callback.requires {
            tracks.insert(
                required.track.clone(),
                canonical
                    .track(&required.track)
                    .cloned()
                    .ok_or("parent track")?,
            );
        }
    }
    let parent = "oteryn:quest-progress/chosen-source/dreamer_s_challenge_quest/s4";
    let parent_track = tracks
        .get(parent)
        .ok_or("independent Dreamer qualification")?;
    let parent_key = "oteryn:quest-transition/qengine2-story/document-parent";
    transitions.push(QuestTransition {
        world_change: None,
        key: parent_key.into(),
        quest: parent_track.quest.clone(),
        effects: vec![QuestEffect {
            track: parent.into(),
            from: QuestComparison::Eq(0),
            effect: QuestEffectKind::Set(1),
        }],
        completes: false,
        experience: None,
    });
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
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Order witness", owner.owner_now());
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    let npc_key = if family == "brotherhood" {
        "oteryn:npc.the_bone_master"
    } else {
        "oteryn:npc.the_dream_master"
    };
    let [npc] = super::classic_quests::install(owner, admitted, [npc_key]).await?;
    let find = |stage: &str| {
        callbacks
            .iter()
            .find(|c| c.key == format!("oteryn:quest-reward/l10-{family}-{stage}"))
            .ok_or("source callback")
    };
    let join = find("join")?;
    assert_eq!(
        join.give.as_ref().map(|r| (r.item.as_str(), r.count)),
        Some(("oteryn:item.tibia.i768", 1))
    );
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources")
        .fetch_one(&h.pool)
        .await?;
    // With the independent parent absent, the guarded source conversation
    // refuses its first "yes" as well as the final document confirmation.
    let (refused, _, _) = super::qengine_rewards::wire_reward_with_prefix_refusal(
        owner, admitted, npc, join, false, Some(2),
    ).await?;
    assert_eq!(refused, oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        after, before,
        "absent independent parent cannot create documents"
    );
    super::qengine_outfit_stories::commit_fixture(owner, admitted, parent_key, 9700).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    super::qengine_outfit_stories::earn_level(owner, admitted, authority, 50).await?;
    let mut bags = vec![];
    let mut ordinal = 700;
    for (stage, item, price) in [
        ("join", 768, 0),
        ("officer", 769, 500),
        ("leader", 770, 1000),
    ] {
        let callback = find(stage)?;
        assert_eq!(
            callback.give.as_ref().map(|r| (r.item.as_str(), r.count)),
            Some((format!("oteryn:item.tibia.i{item}").as_str(), 1))
        );
        assert_eq!(
            callback.take,
            if price == 0 {
                vec![]
            } else {
                vec![
                    oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount {
                        fluid_subtype: None,

                        item: "oteryn:item.tibia.i6499".into(),
                        count: price,
                    },
                ]
            }
        );
        if price > 0 {
            let (result, _, _) =
                super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
            assert_eq!(
                result,
                NpcIntentDisposition::Rejected,
                "missing source essences"
            );
            super::qengine_outfit_stories::supply(
                owner,
                admitted,
                h,
                &oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount {
                    fluid_subtype: None,

                    item: "oteryn:item.tibia.i6499".into(),
                    count: price,
                },
                &mut bags,
                &mut ordinal,
            )
            .await?;
        }
        let held_essences = super::qengine_voucher_stories::quantity(h, 6499).await?;
        let (result, _, command) =
            super::qengine_rewards::wire_reward(owner, admitted, npc, callback, false).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(
            super::qengine_voucher_stories::quantity(h, item).await?,
            1,
            "actual NPC produces the document"
        );
        assert_eq!(
            super::qengine_voucher_stories::quantity(h, 6499).await?,
            held_essences - i64::from(price),
            "exact source essences consumed, retain supplied surplus"
        );
        let replay = super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            7,
            oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
                &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                    npc_actor: npc,
                    text: "yes".into(),
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
        assert_eq!(
            super::qengine_voucher_stories::quantity(h, item).await?,
            1,
            "NPC replay cannot duplicate document"
        );
        if family == "nightmare" {
            super::qengine_voucher_stories::use_native(
                owner,
                admitted,
                item,
                UseDisposition::Committed,
            )
            .await?;
        }
    }
    let key = format!("oteryn:outfit.{family}");
    if family == "brotherhood" {
        super::qengine_voucher_stories::use_native(owner, admitted, 769, UseDisposition::Rejected)
            .await?;
        assert_eq!(
            super::qengine_voucher_stories::quantity(h, 769).await?,
            1,
            "missing base retains officer document"
        );
        let premium = owner.premium_coordinator.take();
        super::qengine_voucher_stories::use_native(owner, admitted, 768, UseDisposition::Rejected)
            .await?;
        owner.premium_coordinator = premium;
        for (item, mask) in [(768, 0), (769, 1), (770, 3)] {
            super::qengine_voucher_stories::use_native(
                owner,
                admitted,
                item,
                UseDisposition::Committed,
            )
            .await?;
            assert_eq!(
                super::qengine_voucher_stories::quantity(h, item).await?,
                0,
                "ordinary USE consumes the actual NPC document"
            );
            super::qengine_voucher_stories::mask(owner, admitted, &key, mask).await?;
        }
    }
    super::qengine_voucher_stories::mask(owner, admitted, &key, 3).await?;
    let grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE source_kind='quest'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        grants, 3,
        "base and each addon have their own immutable proof"
    );
    // A fresh, duplicate source cannot spend an already owned account grant.
    for _ in 0..3 {
        super::qengine_voucher_stories::mint(owner, admitted, 768, 1, &mut ordinal).await?;
    }
    super::qengine_voucher_stories::use_native(owner, admitted, 768, UseDisposition::Rejected)
        .await?;
    assert_eq!(super::qengine_voucher_stories::quantity(h, 768).await?, 3);
    let observed = owner
        .appearance_observation(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            vec![],
        )
        .await
        .ok_or("appearance")?;
    let other = if family == "brotherhood" {
        "nightmare"
    } else {
        "brotherhood"
    };
    assert!(
        !observed
            .catalogue
            .outfits
            .iter()
            .any(|o| o.key == format!("oteryn:outfit.{other}")),
        "one document cannot earn both orders"
    );
    let (ordinal_text,proof):(String,Vec<u8>)=sqlx::query_as("SELECT cause_ordinal::text,proof FROM game_quest_callback_consumptions WHERE transition_key LIKE 'oteryn:quest-reward/l10-%-use-follower' LIMIT 1").fetch_one(&h.pool).await?;
    let proof: serde_json::Value = serde_json::from_slice(&proof)?;
    let ingress: crate::durability::reward_claim_mint::QuestCarriedUse =
        serde_json::from_value(proof["carried_use"].clone())?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let command = crate::foundation::CommandRef::new(
        admitted.game_session_id,
        crate::foundation::CommandId::new(ordinal_text.parse()?).map_err(debug)?,
    );
    assert_eq!(
        owner
            .root
            .reconcile_quest_carried_use(
                owner.character,
                fence.character_id,
                command,
                ingress.intent.clone()
            )
            .await
            .map_err(debug)?,
        Some(true),
        "exact historical ingress survives source retirement"
    );
    assert_eq!(
        owner
            .root
            .reconcile_quest_carried_use(owner.character, fence.character_id, command, vec![255])
            .await
            .map_err(debug)?,
        Some(false),
        "a different ingress cannot borrow an existing success"
    );
    // Exercise the new deferred selected-instance proof before unrelated guards.
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_character_quest_receipts SELECT (jsonb_populate_record(NULL::game_character_quest_receipts,to_jsonb(r)||jsonb_build_object('cause_ordinal',999998,'original_character_revision',999997,'committed_character_revision',999998,'created_xact_id',pg_current_xact_id()::text))).* FROM game_character_quest_receipts r WHERE r.transition_key LIKE 'oteryn:quest-reward/l10-%-use-follower' LIMIT 1").execute(&mut *tx).await?;
    sqlx::query(r#"
        WITH template AS (
            SELECT c.* FROM game_quest_callback_consumptions c
            WHERE c.transition_key LIKE 'oteryn:quest-reward/l10-%-use-follower' LIMIT 1
        ), fresh AS (
            SELECT i.item_instance_id,to_jsonb(i) AS before_item,c.location,c.kind
            FROM template t JOIN game_item_instances i ON i.lifecycle=1
                AND i.definition_production_key='oteryn:item.tibia.i768'
            JOIN LATERAL game_quest_callback_carried(t.character_id,i.world_id) c
                ON c.item=i.item_instance_id LIMIT 1
        ), selected AS (
            SELECT to_jsonb(t)||jsonb_build_object(
                'transaction_id',game_character_uuid_v7()::text,'cause_ordinal',999998,
                'item_instance_id',f.item_instance_id,'before_item',f.before_item,
                'before_location',f.location,'location_kind',f.kind,
                'created_xact_id',pg_current_xact_id()::text) AS candidate_record,
                convert_to((convert_from(t.proof,'UTF8')::jsonb||jsonb_build_object(
                    'before',f.before_item,'location',f.location,
                    'carried_use',jsonb_build_object('source','[]'::jsonb,'intent',jsonb_build_array(1))))::text,'UTF8') AS bad
            FROM template t CROSS JOIN fresh f
        ) INSERT INTO game_quest_callback_consumptions
        SELECT (jsonb_populate_record(NULL::game_quest_callback_consumptions,
            candidate_record||jsonb_build_object('proof',bad,'proof_sha256',sha256(bad)))).*
        FROM selected
    "#).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_quest_reward_sources SELECT (jsonb_populate_record(NULL::game_quest_reward_sources,to_jsonb(s)||jsonb_build_object('cause_ordinal',999998,'created_xact_id',pg_current_xact_id()::text))).* FROM game_quest_reward_sources s WHERE s.plan ? 'carried_item' AND s.transition_key LIKE '%-use-follower' LIMIT 1").execute(&mut *tx).await?;
    let invalid = sqlx::query("SET CONSTRAINTS game_quest_carried_use_complete IMMEDIATE")
        .execute(&mut *tx)
        .await;
    assert!(
        matches!(invalid,Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("23514") && e.message().contains("ingress differs from selected instance")),
        "forged selected source must fail its own guard: {invalid:?}"
    );
    tx.rollback().await?;
    Ok(())
}
