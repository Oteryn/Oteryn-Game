//! Physical proof using the node's control handler and the live appearance stream.
#![cfg(feature = "qualification-spawn")]

use crate::bestiary_postgres_harness::TestResult;
use crate::qualification::client_world::{parse_spawned_entity, request_fixture_control};
use oteryn_dev_client::{DevClientSession, SessionEvent, native_session::AppearanceUpdate};
use oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use std::time::Duration;

#[test]
fn qualification_forge_spawn_first_appearance_carries_explicit_classification() -> TestResult {
    if std::env::var_os("NODE_BOOT_CONTROL_SOCKET").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            crate::qualification::node_boot_spell_client_case(Some(assert_first_appearances))
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "qualification Forge spawn proof thread failed")?
        .map_err(Into::into)
}

async fn assert_first_appearances(mut client: DevClientSession) -> TestResult {
    assert!(
        client.selected_capabilities().contains(&CAPABILITY_FORGE_CREATURE_V1),
        "the observer must negotiate Forge appearance"
    );
    // The Thalom source stairs lead from the temple protection zone to the hunt floor.
    for _ in 0..9 {
        let step = client.step_retrying(StepDirection::South).await?;
        assert_eq!(
            step.disposition,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
        );
    }
    let at = client.world_spatial().actor_position;
    let hunt = crate::qualification_wild_spawn::POSITION;
    assert_eq!((at.x, at.y, at.floor), (hunt.x - 1, hunt.y, hunt.floor));
    let session = crate::node::uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let request = async |arguments: String| {
        request_fixture_control(session.clone(), "--spawn-forge", arguments).await
    };
    let refused = request(format!(
        "canary:creature/druid_familiar {} {} {} fiendish", hunt.x, hunt.y, hunt.floor
    )).await;
    assert_eq!(
        refused.as_ref().err().map(String::as_str),
        Some("ineligible_forge_creature"),
        "{refused:?}"
    );

    for (classification, expected_stacks, x) in [
        ("influenced 3", 3, hunt.x),
        ("fiendish", 15, hunt.x - 2),
    ] {
        let reply = request(format!(
            "canary:creature/rat {x} {} {} {classification}", hunt.y, hunt.floor
        )).await?;
        let exact = parse_spawned_entity(&reply)?;
        // Check the first wire upsert for this incarnation. Polling the final client state
        // could hide an initial ordinary upsert followed by a classification update.
        let first_stacks = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                for event in client.wait_for_events(Duration::from_secs(15)).await? {
                    if let SessionEvent::Appearance(delta) = event
                        && let AppearanceUpdate::Mappings(mappings) = delta.value
                        && let Some(binding) = mappings.upserts.iter().find(|b| b.entity == exact)
                    {
                        return TestResult::Ok(binding.forge_stacks);
                    }
                }
            }
        })
        .await
        .map_err(|_| format!("first Forge appearance timed out for {reply}"))??;
        assert_eq!(first_stacks, Some(expected_stacks), "{reply} {classification}");
    }
    client.session_mut().logout().await?;
    Ok(())
}
