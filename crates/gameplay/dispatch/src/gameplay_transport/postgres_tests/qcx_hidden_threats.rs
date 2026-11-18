//! Global Hidden Threats dialogue and reward compatibility through NPC dispatch.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::{ChannelContentPin, WorldId};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use std::sync::Arc;

#[test]
fn servant_accepts_walkthrough_wording() -> TestResult {
    corym_case("decreased resources", false)
}

#[test]
fn southwestern_worker_grants_small_amethyst() -> TestResult {
    corym_case("decreasing resources", true)
}

fn corym_case(prompt: &'static str, check_reward: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run_async(async move |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "qcx_hidden", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            4,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(controller.active().ok_or("active content")?);
        owner.quest_catalogue = Some(Arc::new(
            load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        deps.initialize(&owner).await;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "quest fence")?
            .ok_or("quest fence")?;
        admitted.item_fence = Some(fence.item_fence());
        let generation = owner.active_generation.ok_or("active content")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let native = generation.native_gameplay().ok_or("native content")?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &template,
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        let admitted = &admitted;
        let [ratter, servant, worker] = install(
            &owner,
            admitted,
            [
                "oteryn:npc.corym_ratter",
                "oteryn:npc.corym_servant",
                "oteryn:npc.corym_worker_5",
            ],
        )
        .await?;

        talk(&mut owner, admitted, ratter, 100, "hi").await?;
        talk(&mut owner, admitted, ratter, 101, "help").await?;
        talk(&mut owner, admitted, ratter, 102, "yes").await?;
        talk(&mut owner, admitted, ratter, 103, "bye").await?;
        talk(&mut owner, admitted, servant, 104, "hi").await?;
        let lines = talk(&mut owner, admitted, servant, 105, prompt).await?;
        assert!(lines.iter().any(|line| line.contains("defy")), "{lines:?}");
        talk(&mut owner, admitted, servant, 106, "defy").await?;
        assert_eq!(
            progress(
                &owner,
                admitted,
                &authority,
                "oteryn:quest-progress/quest/u11_50/hidden_threats/servant_door"
            )
            .await?,
            1,
        );
        if check_reward {
            talk(&mut owner, admitted, servant, 107, "bye").await?;
            talk(&mut owner, admitted, worker, 108, "hi").await?;
            assert_eq!(super::questpc7_outfit_items::quantity(&h, 3033).await?, 1);
            assert_eq!(super::questpc7_outfit_items::quantity(&h, 3029).await?, 0);
            talk(&mut owner, admitted, worker, 109, "bye").await?;
            talk(&mut owner, admitted, worker, 110, "hi").await?;
            assert_eq!(super::questpc7_outfit_items::quantity(&h, 3033).await?, 1);
        }
        let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
        let base_pin = [
            0x8d, 0x75, 0x51, 0x7e, 0x41, 0x35, 0x9a, 0x25, 0xab, 0xea, 0x65, 0x3c, 0x8f, 0xd4,
            0x4f, 0xa5, 0xb4, 0xd2, 0x76, 0x8c, 0x14, 0x70, 0xd9, 0xac, 0xaa, 0x94, 0x42, 0xb7,
            0x3f, 0x54, 0x30, 0x65,
        ];
        assert_ne!(
            catalogue.definition_hash("oteryn:quest.tibia_tales"),
            Some(base_pin)
        );
        assert!(
            oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(
                catalogue,
                "oteryn:quest.tibia_tales",
                &base_pin,
            )
            .is_some()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {prompt} {:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
