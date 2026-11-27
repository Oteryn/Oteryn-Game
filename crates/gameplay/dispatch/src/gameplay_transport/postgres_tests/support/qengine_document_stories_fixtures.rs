use super::fixture::Harness;

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) const BONE_MASTER: Position = Position {
    x: 32793,
    y: 32225,
    floor: -14,
};

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
    use oteryn_gameplay_quests::quest::reward_callbacks::participant_key;
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
        let keys = std::iter::once(callback.key.clone()).chain(
            callback
                .participants
                .iter()
                .enumerate()
                .map(|(index, _)| participant_key(&callback.key, index)),
        );
        for key in keys {
            let transition = canonical
                .transition(&key)
                .cloned()
                .ok_or("source transition")?;
            for effect in &transition.effects {
                tracks.insert(
                    effect.track.clone(),
                    canonical
                        .track(&effect.track)
                        .cloned()
                        .ok_or("source track")?,
                );
            }
            transitions.push(transition);
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
    let [npc] = if family == "brotherhood" {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let source = oteryn_world_bundle_compiler::source::project_regions(
            &root,
            &[(BONE_MASTER.x / 256, BONE_MASTER.y / 256)],
        )?;
        let position = source
            .families
            .npc_placements
            .iter()
            .find(|placement| placement.npc == npc_key)
            .ok_or("production Bone Master placement")?;
        let (x, y, floor) = position.cell;
        let position = Position {
            x,
            y,
            floor: -i8::try_from(floor)?,
        };
        assert_eq!(position, BONE_MASTER);
        super::classic_quests_fixtures::install_positions(owner, admitted, [npc_key], [position])
            .await?
    } else {
        super::classic_quests_fixtures::install(owner, admitted, [npc_key]).await?
    };
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
    let (refused, _, _) = super::qengine_rewards_fixtures::wire_reward_with_prefix_refusal(
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
    if family == "nightmare" {
        assert!(!h.root.read_character_quest_state(authority, super::fixture::fence(1)?.character_id)
            .await.map_err(debug)?.meets_door_storage(41014));
    }
    let session_id = admitted.game_session_id;
    let tower_open = |owner: &ComposedFreshAdmission<'_, '_, '_>| {
        owner.states.quest_sessions.lock().map_err(debug).map(|sessions| {
            sessions
                .get(&session_id)
                .and_then(|session| session.copy.as_ref())
                .is_some_and(|copy| copy.meets_door_storage(41014))
        })
    };
    assert!(
        !tower_open(owner)?,
        "the refused join cannot unlock Knightwatch Tower"
    );
    super::qengine_outfit_stories_fixtures::commit_fixture(owner, admitted, parent_key, 9700).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    // Dread Lord takes 1,500 essences at 1 oz each. The returning Brotherhood
    // character needs capacity for the full payment, bags and prior rank rewards.
    let carrying_level = match family { "brotherhood" => 60, "nightmare" => 150, _ => 50 };
    super::qengine_outfit_stories_fixtures::earn_level(owner, admitted, authority, carrying_level).await?;
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
                super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, callback, false).await?;
            assert_eq!(
                result,
                NpcIntentDisposition::Rejected,
                "missing source essences"
            );
            super::qengine_outfit_stories_fixtures::supply(
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
        let held_essences = super::qengine_voucher_stories_fixtures::quantity(h, 6499).await?;
        let (result, _, command) =
            super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, callback, false).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(
            super::qengine_voucher_stories_fixtures::quantity(h, item).await?,
            1,
            "actual NPC produces the document"
        );
        assert_eq!(
            super::qengine_voucher_stories_fixtures::quantity(h, 6499).await?,
            held_essences - i64::from(price),
            "exact source essences consumed, retain supplied surplus"
        );
        if stage == "join" {
            owner.refresh_quest_session(session_id, false).await;
            assert!(
                tower_open(owner)?,
                "joining opens both Knightwatch Tower entrances through durable storage"
            );
        }
        let replay = super::quest_world_callbacks_fixtures::wire(
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
            super::qengine_voucher_stories_fixtures::quantity(h, item).await?,
            1,
            "NPC replay cannot duplicate document"
        );
        if stage == "leader" {
            let [gift] = callback
                .give
                .as_ref()
                .ok_or("leader reward")?
                .additional
                .as_slice()
            else {
                return Err("leader exchange must grant the separate donor reward".into());
            };
            let gift_id = gift
                .item
                .strip_prefix("oteryn:item.tibia.i")
                .ok_or("leader gift id")?
                .parse()?;
            assert_eq!(
                super::qengine_voucher_stories_fixtures::quantity(h, gift_id).await?,
                i64::from(gift.count),
                "leader exchange and replay grant the extra reward exactly once"
            );
        }
        if family == "nightmare" {
            if stage == "join" {
                let fence = owner.current_quest_fence(admitted.game_session_id).await
                    .map_err(|()| "quest fence")?.ok_or("quest fence")?;
                assert!(h.root.read_character_quest_state(authority, fence.character_id)
                    .await.map_err(debug)?.meets_door_storage(41014));
            }
            super::qengine_voucher_stories_fixtures::use_native(
                owner,
                admitted,
                item,
                UseDisposition::Committed,
            )
            .await?;
        }
    }
    if family == "nightmare" {
        assert_eq!(super::qengine_voucher_stories_fixtures::quantity(h, 6390).await?, 1);
        let protector = find("lord-protector")?;
        let (refused, _, _) =
            super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, protector, false).await?;
        assert_eq!(refused, NpcIntentDisposition::Rejected);
        super::qengine_outfit_stories_fixtures::supply(
            owner,
            admitted,
            h,
            &oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount {
                fluid_subtype: None,
                item: "oteryn:item.tibia.i6499".into(),
                count: 1500,
            },
            &mut bags,
            &mut ordinal,
        ).await?;
        let before = super::qengine_voucher_stories_fixtures::quantity(h, 6499).await?;
        let (result, _, _) =
            super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, protector, false).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(super::qengine_voucher_stories_fixtures::quantity(h, 6499).await?, before - 1500);
        assert_eq!(
            super::classic_quests_fixtures::progress(
                owner, admitted, authority,
                "oteryn:quest-progress/quest/u7_9/nightmare_outfits/outfits",
            ).await?,
            4
        );
        assert_eq!(
            super::classic_quests_fixtures::progress(
                owner, admitted, authority,
                "oteryn:quest-progress/quest/u7_9/nightmare_outfits/door",
            ).await?,
            1
        );
        let completed: bool = sqlx::query_scalar(
            "SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key='oteryn:quest.nightmare_outfits_quest'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert!(completed);
        let awards: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/lord_protector'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(awards, 1);
    }
    let key = format!("oteryn:outfit.{family}");
    if family == "brotherhood" {
        let dread = find("dread-lord")?;
        let (refused, _, _) =
            super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, dread, false).await?;
        assert_eq!(refused, NpcIntentDisposition::Rejected);
        super::qengine_outfit_stories_fixtures::supply(
            owner,
            admitted,
            h,
            &dread.take[0],
            &mut bags,
            &mut ordinal,
        )
        .await?;
        let before = super::qengine_voucher_stories_fixtures::quantity(h, 6499).await?;
        let (result, _, _) =
            super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, dread, false).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        assert_eq!(
            super::qengine_voucher_stories_fixtures::quantity(h, 6499).await?,
            before - 1500
        );
        for state in &dread.storage {
            let value: i64 = sqlx::query_scalar(
                "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
            )
            .bind(&state.track)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(value, state.set);
        }
        let granted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_account_achievements WHERE achievement_key='oteryn:achievement/dread_lord')",
        )
        .fetch_one(&h.pool)
        .await?;
        assert!(granted);
        super::qengine_voucher_stories_fixtures::use_native(owner, admitted, 769, UseDisposition::Rejected)
            .await?;
        assert_eq!(
            super::qengine_voucher_stories_fixtures::quantity(h, 769).await?,
            1,
            "missing base retains officer document"
        );
        let premium = owner.premium_coordinator.take();
        super::qengine_voucher_stories_fixtures::use_native(owner, admitted, 768, UseDisposition::Rejected)
            .await?;
        owner.premium_coordinator = premium;
        for (item, mask) in [(768, 0), (769, 1), (770, 3)] {
            super::qengine_voucher_stories_fixtures::use_native(
                owner,
                admitted,
                item,
                UseDisposition::Committed,
            )
            .await?;
            assert_eq!(
                super::qengine_voucher_stories_fixtures::quantity(h, item).await?,
                0,
                "ordinary USE consumes the actual NPC document"
            );
            super::qengine_voucher_stories_fixtures::mask(owner, admitted, &key, mask).await?;
        }
    }
    super::qengine_voucher_stories_fixtures::mask(owner, admitted, &key, 3).await?;
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
        super::qengine_voucher_stories_fixtures::mint(owner, admitted, 768, 1, &mut ordinal).await?;
    }
    super::qengine_voucher_stories_fixtures::use_native(owner, admitted, 768, UseDisposition::Rejected)
        .await?;
    assert_eq!(super::qengine_voucher_stories_fixtures::quantity(h, 768).await?, 3);
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
