//! W06 Echo Warden kill trigger, portal, raid and first-species charm reward, derived from the
//! Echo Warden domain owner (`domain/echo_warden.rs`, `death_effect`).
//!
//! A qualifying kill (difficulty stars 1-2, no forge stacks, not a summon or boss) schedules the
//! Echo portal on a 0.05 percent draw; a tagged Warden rewards the first-species charm.
//! 1. Walk to the nearest rat spawn and kill rats through the real attack path (`client_hunt::hunt`),
//!    scanning every entity frame after each kill for the portal item (54133).
//! 2. Log out requiring `Accepted`.
//! 3. The portal draw cannot be forced from a player route, so a miss reports FAIL naming the
//!    0.05 percent draw; a sighted portal still reports FAIL because the raid and the charm reward
//!    are not driven. The row never mints state.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};

const PORTAL_ITEM: u32 = 54_133;
const KILLS: usize = 3;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(420), journey(clients)).await;
    let detail = match result {
        Ok(Ok(seen)) => describe(KILLS, seen),
        Ok(Err(error)) => compact(&error.to_string()),
        Err(_) => "step=journey-observed=timeout".to_owned(),
    };
    evidence(&format!(
        "e2e_all scenario=W06.full status=FAIL seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let token: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(400)
        .collect();
    if token.is_empty() {
        "unknown".to_owned()
    } else {
        token
    }
}

/// The report token for a finished hunt.
fn describe(kills: usize, portal_seen: bool) -> String {
    if portal_seen {
        format!(
            "needed:echo-raid-and-first-species-charm-driver(portal-observed-after-{kills}-kills)"
        )
    } else {
        format!(
            "needed:echo-warden-portal(0.05pct-draw-not-hit-in-{kills}-qualifying-kills);no-deterministic-player-route"
        )
    }
}

fn portal_visible(client: &DevClientSession, portal: u32) -> bool {
    client.world_entities().is_some_and(|entities| {
        entities.iter().any(|entry| {
            entry.kind != EntityKind::Corpse
                && matches!(entry.detail, oteryn_dev_client::EntityDetail::Object { item_definition_ref, .. } if item_definition_ref == portal)
        })
    })
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<bool> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let index = provisioning::item_index()?;
    let portal = provisioning::item_reference(&index, PORTAL_ITEM)?.get();
    let mut client = row_connect_world(clients, 0xe3).await?;
    let mut seen = false;
    for _ in 0..KILLS {
        travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(&mut client).await?;
        client.service_liveness(Duration::from_millis(200)).await?;
        seen |= portal_visible(&client, portal);
    }
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(seen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_names_the_missing_draw_or_driver() {
        assert!(describe(3, false).starts_with("needed:echo-warden-portal("));
        assert!(describe(3, true).contains("portal-observed-after-3-kills"));
        assert!(!describe(3, false).contains(char::is_whitespace));
    }

    #[test]
    fn compact_is_one_token() {
        assert_eq!(compact("a  b"), "a-b");
    }
}
