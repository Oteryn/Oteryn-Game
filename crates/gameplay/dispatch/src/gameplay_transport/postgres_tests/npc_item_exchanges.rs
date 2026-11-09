//! Atomic recipe transactions, driven through the existing fenced NPC writer.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::quest_state::npc_program::NpcProgramRequest;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition,
    npc_rules::{NpcRuleCatalog, NpcRuleTrigger},
    world_callbacks::{Offering, Position},
};
use std::sync::Arc;

const PREFIX: &str = "oteryn:quest-progress/quest/u8_5/hot_cuisine_quest/";

#[test]
fn npc_item_exchanges_consume_exact_recipes_with_one_receipt_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("npc-item-exchanges", || {
        super::quest_world_callbacks::scenario(
            "npc-item-exchanges",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "NPC_EXCHANGE POSTGRES_CASE_COMPLETED three_source_recipes duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("NPC_EXCHANGE POSTGRES_STARTED version={version}");
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    assert!(
        owner
            .premium_gate(super::fixture::id(40))
            .await
            .premium_current(),
        "resumed cook requires delivered live Premium"
    );
    let catalogue = resumed_catalogue(owner, h).await?;
    owner.quest_catalogue = Some(catalogue.clone());
    let book: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/hot-cuisine-batch3.json"
    ))?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
        .with_wheel(owner.premium_coordinator);
    let mut mint_ordinal = 700;
    for index in [6_u64, 8, 14] {
        let recipe = &book["recipes"][usize::try_from(index - 1)?];
        let mut fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let command = |id| -> TestResult<_> {
            Ok(CommandRef::new(
                admitted.game_session_id,
                CommandId::new(id).map_err(debug)?,
            ))
        };
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let seeded = slot
            .commit_quest_transition(
                owner.root,
                authority,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: format!("oteryn:quest-transition/fixture/resumed-cook-{index}"),
                    cause: QuestCause::Command(command(8000 + index)?),
                },
                catalogue.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(seeded, QuestTransitionOutcome::Committed(_)),
            "{seeded:?}"
        );
        drop(slot);
        fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let request = NpcProgramRequest {
            command: command(8100 + index)?,
            npc: "oteryn:npc.jean_pierre".into(),
            trigger: NpcRuleTrigger::Say,
            message: "yes".into(),
            topic: i32::try_from(recipe["topic"].as_i64().ok_or("topic")?)?,
            transition_key: format!(
                "oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-{index}"
            ),
        };
        let before_missing = owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?;
        let missing = owner
            .root
            .commit_character_npc_program(
                authority,
                owner.holder,
                &owner
                    .spell_lane_permit()
                    .await
                    .ok_or("NPC programme permit")?,
                fence,
                request.clone(),
                catalogue.clone(),
                &weights,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            missing.transition,
            QuestTransitionOutcome::Refused(_)
        ));
        assert_eq!(
            before_missing,
            owner
                .root
                .read_character_quest_state(authority, fence.character_id)
                .await
                .map_err(debug)?
        );
        for ingredient in recipe["ingredients"].as_array().ok_or("ingredients")? {
            assert!(
                ingredient
                    .get("subtype")
                    .is_none_or(|subtype| subtype.is_null())
            );
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &Offering {
                    position: Position {
                        x: 33293,
                        y: 31677,
                        floor: -15,
                    },
                    item: ingredient["item"].as_str().ok_or("item")?.into(),
                    count: u16::try_from(ingredient["count"].as_u64().ok_or("count")?)?,
                },
                mint_ordinal,
                true,
            )
            .await
            .map_err(|error| format!("ingredient {}: {error}", ingredient["item"]))?;
            mint_ordinal += 1;
        }
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let committed = slot
            .commit_npc_program(
                owner.root,
                authority,
                owner.holder,
                &owner
                    .spell_lane_permit()
                    .await
                    .ok_or("NPC programme permit")?,
                fence,
                request.clone(),
                catalogue.clone(),
                &weights,
            )
            .await
            .map_err(debug)?;
        drop(slot);
        assert!(
            matches!(committed.transition, QuestTransitionOutcome::Committed(_)),
            "{committed:?}"
        );
        let output = recipe["reward"]["item"].as_str().ok_or("output")?;
        let quantity: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
            .bind(output).fetch_one(&h.pool).await?;
        assert_eq!(quantity, 1, "exact cooked output");
        for ingredient in recipe["ingredients"].as_array().ok_or("ingredients")? {
            let remaining: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
                .bind(ingredient["item"].as_str().ok_or("item")?).fetch_one(&h.pool).await?;
            assert_eq!(remaining, 0, "every exact ingredient must be consumed");
        }
        let copy = owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(
            copy.tracks()
                .get(&format!("{PREFIX}kv/cookcount-{}", index + 1)),
            Some(&1)
        );
        let replay = owner
            .root
            .reconcile_npc_program(authority, fence.character_id, request.clone())
            .await
            .map_err(debug)?
            .ok_or("durable replay")?;
        let (
            QuestTransitionOutcome::AlreadyCommitted(replayed),
            QuestTransitionOutcome::Committed(original),
        ) = (&replay.transition, &committed.transition)
        else {
            return Err("receipt replay outcome".into());
        };
        assert_eq!(replayed, original);
        assert_eq!(replay.reply, committed.reply);
        assert_eq!(replay.next_topic, committed.next_topic);
        let mut conflict = request;
        conflict.message = "yes changed".into();
        assert!(
            owner
                .root
                .reconcile_npc_program(authority, fence.character_id, conflict)
                .await
                .is_err()
        );
        assert_eq!(
            copy,
            owner
                .root
                .read_character_quest_state(authority, fence.character_id)
                .await
                .map_err(debug)?
        );
    }
    // All nine recipe-six operands are stackable. Leave one of each behind so
    // consuming them cannot free the output's main-backpack slot.
    let mut fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let seeded = slot
        .commit_quest_transition(
            owner.root,
            authority,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: "oteryn:quest-transition/fixture/bonus-cook-6".into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(17000).map_err(debug)?,
                )),
            },
            catalogue.clone(),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    for ingredient in book["recipes"][5]["ingredients"]
        .as_array()
        .ok_or("ingredients")?
    {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: ingredient["item"].as_str().ok_or("item")?.into(),
                count: u16::try_from(ingredient["count"].as_u64().ok_or("count")? + 1)?,
            },
            mint_ordinal,
            true,
        )
        .await?;
        mint_ordinal += 1;
    }
    let bag = weights
        .loop_facts(&crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        })
        .ok_or("backpack definition")?;
    let capacity = i64::from(bag.container_capacity.ok_or("backpack capacity")?.min(20));
    let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    for _ in entries..capacity {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: "oteryn:item.tibia.i2853".into(),
                count: 1,
            },
            mint_ordinal,
            true,
        )
        .await?;
        mint_ordinal += 1;
    }
    let inventory: serde_json::Value = sqlx::query_scalar(
        "SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i",
    )
    .fetch_one(&h.pool)
    .await?;
    let quest = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    let receipts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_callback_consumptions),(SELECT count(*) FROM game_item_audit_outbox)").fetch_one(&h.pool).await?;
    let request = NpcProgramRequest {
        command: CommandRef::new(
            admitted.game_session_id,
            CommandId::new(18000).map_err(debug)?,
        ),
        npc: "oteryn:npc.jean_pierre".into(),
        trigger: NpcRuleTrigger::Say,
        message: "yes".into(),
        topic: 15,
        transition_key: "oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-6".into(),
    };
    let refused = owner
        .root
        .commit_character_npc_program(
            authority,
            owner.holder,
            &owner
                .spell_lane_permit()
                .await
                .ok_or("NPC programme permit")?,
            fence,
            request.clone(),
            catalogue.clone(),
            &weights,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            refused.transition,
            QuestTransitionOutcome::Refused(
                oteryn_gameplay_quests::quest::QuestRefusal::CapacityExceeded
            )
        ),
        "{refused:?}"
    );
    let after_inventory: serde_json::Value = sqlx::query_scalar(
        "SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        inventory, after_inventory,
        "output refusal must roll back all nine debits"
    );
    assert_eq!(
        quest,
        owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?
    );
    let after_receipts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_callback_consumptions),(SELECT count(*) FROM game_item_audit_outbox)").fetch_one(&h.pool).await?;
    assert_eq!(
        receipts, after_receipts,
        "refusal must leave no receipt or audit fragment"
    );
    assert!(
        owner
            .root
            .reconcile_npc_program(authority, fence.character_id, request.clone())
            .await
            .map_err(debug)?
            .is_none()
    );
    let mut stale = fence;
    stale.character_lease_generation += 1;
    let mut stale_request = request;
    stale_request.command = CommandRef::new(
        admitted.game_session_id,
        CommandId::new(18001).map_err(debug)?,
    );
    assert!(matches!(
        owner
            .root
            .commit_character_npc_program(
                authority,
                owner.holder,
                &owner
                    .spell_lane_permit()
                    .await
                    .ok_or("NPC programme permit")?,
                stale,
                stale_request,
                catalogue,
                &weights
            )
            .await,
        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)
    ));
    Ok(())
}

pub(super) async fn resumed_catalogue(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
) -> TestResult<Arc<QuestStateCatalogue>> {
    // A resumed level-35 cook is fixture history. Eligibility is subsequently
    // reloaded by the real fenced NPC owner, never supplied by the command.
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *fixture)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=35,total_experience=617100 WHERE character_id=encode($1,'hex')::uuid")
        .bind(super::fixture::id(41).as_slice()).execute(&mut *fixture).await?;
    fixture.commit().await?;
    let source = NpcRuleCatalog::runtime().map_err(debug)?;
    let program = source
        .program("oteryn:quest.hot_cuisine_quest")
        .ok_or("Hot Cuisine source programme")?;
    let tracks = program
        .document
        .tracks
        .iter()
        .map(|track| QuestTrack {
            key: track.key.clone(),
            quest: track.quest.clone(),
            initial: track.initial,
            min: track.min,
            max: track.max,
        })
        .collect();
    // A resumed apprenticeship is legal outside August. Establish only its
    // preconditions; the recipe's item/progress transaction is the owner under test.
    let mut transitions: Vec<_> = [6, 8, 12, 14, 15]
        .into_iter()
        .map(|index| QuestTransition {
            key: format!("oteryn:quest-transition/fixture/resumed-cook-{index}"),
            quest: "oteryn:quest.hot_cuisine_quest".into(),
            effects: [("quest_start", 1), ("current_dish", index)]
                .into_iter()
                .map(|(suffix, value)| QuestEffect {
                    track: format!("{PREFIX}{suffix}"),
                    from: QuestComparison::Between(-1, i64::MAX),
                    effect: QuestEffectKind::Set(value),
                })
                .chain(
                    (1..=14)
                        .filter(move |_| index == 15)
                        .map(|previous| QuestEffect {
                            track: format!("{PREFIX}kv/cookcount-{}", previous + 1),
                            from: QuestComparison::Any,
                            effect: QuestEffectKind::Set(1),
                        }),
                )
                .collect(),
            completes: false,
            experience: None,
            world_change: None,
        })
        .collect();
    let current_year: i64 =
        sqlx::query_scalar("SELECT extract(year FROM now() AT TIME ZONE 'UTC')::bigint")
            .fetch_one(&h.pool)
            .await?;
    transitions.push(QuestTransition {
        key: "oteryn:quest-transition/fixture/bonus-cook-6".into(),
        quest: "oteryn:quest.hot_cuisine_quest".into(),
        effects: [
            ("current_dish", 6),
            ("kv/bonusyear", current_year),
            ("kv/bonusused-7", 0),
        ]
        .into_iter()
        .map(|(suffix, value)| QuestEffect {
            track: format!("{PREFIX}{suffix}"),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Set(value),
        })
        .collect(),
        completes: false,
        experience: None,
        world_change: None,
    });
    // Legal resumed preconditions for the terminal award. The selected dish's
    // two cooks remain real item transactions; earlier courses are fixture history.
    for selected in [12, 15] {
        transitions.push(QuestTransition {
            key: format!("oteryn:quest-transition/fixture/m42-prior-{selected}"),
            quest: "oteryn:quest.hot_cuisine_quest".into(),
            effects: (1..=15)
                .filter(|index| *index != selected)
                .map(|index| QuestEffect {
                    track: format!("{PREFIX}kv/cookcount-{}", index + 1),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(2),
                })
                .collect(),
            completes: false,
            experience: None,
            world_change: None,
        });
        transitions.push(QuestTransition {
            key: format!("oteryn:quest-transition/fixture/m42-bonus-{selected}"),
            quest: "oteryn:quest.hot_cuisine_quest".into(),
            effects: [
                ("current_dish".to_string(), selected),
                ("kv/bonusyear".into(), current_year),
                (format!("kv/bonusused-{}", selected + 1), 0),
                ("quest_start".into(), 2),
            ]
            .into_iter()
            .map(|(suffix, value)| QuestEffect {
                track: format!("{PREFIX}{suffix}"),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(value),
            })
            .collect(),
            completes: false,
            experience: None,
            world_change: None,
        });
    }
    // Bind this resumed fixture to the real admitted programme, not merely shipped source.
    let digest = program
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let binding = format!("oteryn:quest-npc-pin/{digest}");
    transitions.push(
        owner
            .quest_catalogue
            .as_ref()
            .ok_or("native catalogue")?
            .transition(&binding)
            .ok_or("native Hot Cuisine binding")?
            .clone(),
    );
    let catalogue =
        Arc::new(QuestStateCatalogue::new("content-1", tracks, transitions).map_err(debug)?);
    Ok(catalogue)
}
