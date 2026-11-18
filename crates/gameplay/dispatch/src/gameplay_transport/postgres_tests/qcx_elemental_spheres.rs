//! Arkulius dialogue and shop through the production talk dispatcher.
//! TibiaWiki Elemental Spheres Quest/Spoiler, curid 25641, oldid 1179410.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::{ChannelContentPin, WorldId};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcOffer, NpcTalkIntent, NpcTradeSide,
};

const ARKULIUS: &str = "oteryn:npc.arkulius";

#[test]
fn arkulius_explains_alverus_shrines_and_spheres_and_only_buys_neutral_matter() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-elemental-spheres", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "qcx_elemental_spheres", true, false)
                    .await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result
        })
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx_elemental_spheres arkulius dialogue and shop {:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("active content")?;
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
        pin.qualified_room().clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        2,
    )?;
    let admitted = deps.admitted;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let [arkulius] = install(&owner, &admitted, [ARKULIUS]).await?;
    talk(&mut owner, &admitted, arkulius, 700, "hi").await?;

    let mut replies = Vec::new();
    for (command, keyword) in [(701, "alverus"), (702, "shrines"), (703, "sphere")] {
        replies.push(talk(&mut owner, &admitted, arkulius, command, keyword).await?);
    }

    owner.clock_origin -= std::time::Duration::from_secs(2);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("current item fence")?;
    let trade = owner
        .npc_talk(
            deps.actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 704,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: arkulius,
                text: "trade".into(),
            },
        )
        .await;
    assert_eq!(trade.disposition, NpcIntentDisposition::Ok);
    let neutral_matter = active
        .native_gameplay()
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i954", "definition-r1"))
        .ok_or("Neutral Matter definition")?;
    assert_eq!(
        trade.window.ok_or("Arkulius shop window")?.offers,
        vec![NpcOffer {
            item_definition_ref: neutral_matter.get(),
            side: NpcTradeSide::Sell,
            count: 1,
            unit_price: 5000,
        }]
    );
    for (lines, expected) in replies.iter().zip([
        "He accidentally created an Ice Overlord",
        "matching elemental sphere is very close",
        "I found a way to visit them",
    ]) {
        assert!(
            lines.iter().any(|line| line.contains(expected)),
            "{lines:?}"
        );
    }
    Ok(())
}
