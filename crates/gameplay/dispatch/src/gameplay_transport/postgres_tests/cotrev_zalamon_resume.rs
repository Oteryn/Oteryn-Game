//! An exact pre-audit Children definition resumes through Zalamon's live Tome dialogue.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, loader,
    world_callbacks::Position,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.children_of_the_revolution_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/mission04";
const ZALAMON: &str = "oteryn:npc.zalamon";

#[test]
fn cotrev_zalamon_tome_resumes_exact_in_progress_predecessor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || scenario(admin).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Zalamon dialogue panicked")??;
    Ok(())
}

fn scenario(admin: String) -> TestResult {
    run(async move |_| {
        let h = Harness::create_with_admission_seed(admin, "cotrev_resume", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                super::fence(1)?.runtime_scope.world_id(),
                "content-1",
            )?;
        let entry = activated.entry_start();
        let frame_digest = activated.frame_binding().digest();
        let room = activated.qualified_room().clone();
        let mut deps = Dependencies::with_content(
            activated.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .get_mut()
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: entry.x,
                    y: entry.y,
                    floor: entry.floor,
                },
            )
            .map_err(debug)?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?
            .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(frame_digest.as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence,
                &NativeItemWeights(native),
                &crate::durability::starter_grant::StarterTemplate {
                    definition: crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i2854".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    ..crate::durability::starter_grant::StarterTemplate::main()
                },
            )
            .await
            .map_err(debug)?;

        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        let loaded = loader::load_embedded_quest_state("content-1").map_err(debug)?;
        // Snapshot from the task base before the new bindings.
        let before: [u8; 32] = [
            0x42, 0xf1, 0x9e, 0x44, 0x9f, 0x68, 0x10, 0x42, 0x97, 0x6a, 0x39, 0x81, 0x41, 0xa1,
            0x47, 0xd3, 0xe0, 0xa9, 0xf4, 0x8c, 0x31, 0x2c, 0x7a, 0x75, 0x6f, 0x52, 0x8b, 0x25,
            0xfb, 0x75, 0x20, 0xb4,
        ];
        // Model a later addition to the quest without importing the Content lane.
        // The unchanged Tome dialogue still dispatches and writes through its real owner.
        let current_catalogue = loaded
            .catalogue()
            .with_fixture_transition(QuestTransition {
                key: "oteryn:quest-transition/cotrev-fixture/survivor".into(),
                quest: QUEST.into(),
                effects: vec![QuestEffect {
                    track: QUESTLINE.into(),
                    from: QuestComparison::Eq(19),
                    effect: QuestEffectKind::Set(20),
                }],
                completes: false,
                experience: None,
                world_change: None,
            })
            .map_err(debug)?;
        assert_ne!(current_catalogue.definition_hash(QUEST), Some(before));
        owner.quest_catalogue = Some(Arc::new(current_catalogue));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;

        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for sql in [
            "UPDATE game_character_roots SET character_revision=greatest(character_revision,2) WHERE character_id=encode($1,'hex')::uuid",
            "UPDATE game_character_progression_state SET character_revision=greatest(character_revision,2) WHERE character_id=encode($1,'hex')::uuid",
        ] {
            sqlx::query(sql)
                .bind(id(41).as_slice())
                .execute(&mut *tx)
                .await?;
        }
        for (track, value) in [(QUESTLINE, 17_i64), (MISSION, 5_i64)] {
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,2)")
                .bind(id(41).as_slice()).bind(track).bind(QUEST).bind(value)
                .execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'content-1',$3,2,2)")
            .bind(id(41).as_slice()).bind(QUEST).bind(before.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        owner.admit_quest_session(&admitted).await;
        let [zalamon] = super::questl2_fixture::install_at(
            &owner,
            &admitted,
            [ZALAMON],
            [Position {
                x: u16::try_from(entry.x)?,
                y: u16::try_from(entry.y + 1)?,
                floor: -i8::try_from(entry.floor)?,
            }],
        )
        .await?;
        assert_eq!(
            talk(&mut owner, &admitted, zalamon, 1, "hi").await?.0,
            NpcIntentDisposition::Ok
        );

        talk(&mut owner, &admitted, zalamon, 2, "mission").await?;
        assert_eq!(
            talk(&mut owner, &admitted, zalamon, 3, "yes").await?.0,
            NpcIntentDisposition::Ok
        );
        assert_eq!(stage(&h, QUESTLINE).await?, Some(18));
        assert_eq!(stage(&h, MISSION).await?, Some(6));
        let after: Vec<u8> = sqlx::query_scalar(
            "SELECT definition_hash FROM game_character_quest_states WHERE quest_key=$1",
        )
        .bind(QUEST)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(after, before);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc_actor: EntityRef,
    command_id: u64,
    text: &str,
) -> TestResult<(NpcIntentDisposition, Vec<String>)> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor,
                text: text.into(),
            },
        )
        .await;
    assert!(!outcome.unknown);
    Ok((
        outcome.disposition,
        outcome.conversation.map(|c| c.lines).unwrap_or_default(),
    ))
}

async fn stage(h: &Harness, track: &str) -> TestResult<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(track)
            .fetch_optional(&h.pool)
            .await?,
    )
}
