//! G36: Soulpit fight entry. The entry owner is `use_soulpit_obelisk`
//! (`crates/gameplay/dispatch/src/gameplay_transport/soulpit/entry.rs`): an Item USE of a
//! carried Soul Core on the obelisk map object at (32375,31157,-8) from entry slot 0
//! (32375,31158,-8); `TooFar` away from the slot, `StaleState` with no Soul Core carried,
//! `Rejected` for a carried item that is not a Soul Core.
//! 1. Admit the node-boot character and walk to the entry slot (when a walk route exists).
//! 2. Send the real USE (a Soul Core by definition, targeting the obelisk placement) and require
//!    the exact refusal: `StaleState` on the slot, `TooFar` anywhere else.
//! 3. A fresh character carries no Soul Core and no player route supplies one in the run, so the
//!    row ends FAIL naming the precondition. It never provisions state.
//! 4. Log out and require Accepted.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::world_object::{
    self, MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};

const OBELISK: (u16, u16, i8) = (32375, 31157, -8);
const ENTRY_SLOT: (u16, u16, i8) = (32375, 31158, -8);
const OBELISK_KEY: &str = "oteryn:item.tibia.i47367";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients))).await;
    let detail = match result {
        Ok(Ok(observed)) => {
            format!("needed:soul-core-in-custody-for-fight-entry;observed={observed}")
        }
        Ok(Err(error)) => token(&error.to_string()),
        Err(_) => "step=journey;observed=timeout".to_owned(),
    };
    evidence(&format!(
        "e2e_all scenario=G36.full status=FAIL seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

/// The refusal the owner must answer for a core-less character.
fn expected_refusal(at_entry_slot: bool) -> UseDisposition {
    if at_entry_slot {
        UseDisposition::StaleState
    } else {
        UseDisposition::TooFar
    }
}

/// Bundle placement key of the obelisk's base entry: tile x, y, depth and item ordinal.
fn placement_key(x: u16, y: u16, floor: i8, ordinal: u8) -> Option<u64> {
    let depth = u8::try_from(-i16::from(floor)).ok()?;
    Some(u64::from(x) << 32 | u64::from(y) << 16 | u64::from(depth) << 8 | u64::from(ordinal))
}

fn obelisk_ordinal(bytes: &[u8]) -> TestResult<Option<u8>> {
    let mut found = None;
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        if sector.floor != OBELISK.2 {
            return Ok(());
        }
        for tile in sector.tiles {
            if tile.x != OBELISK.0 || tile.y != OBELISK.1 {
                continue;
            }
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if manifest.palette[item.palette as usize].key == OBELISK_KEY {
                    found = u8::try_from(ordinal).ok();
                }
            }
        }
        Ok(())
    })?;
    Ok(found)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout;observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let ordinal = obelisk_ordinal(&bytes)?.ok_or("step=bundle;needed=obelisk-i47367-on-map")?;
    let key = placement_key(OBELISK.0, OBELISK.1, OBELISK.2, ordinal).ok_or("step=bundle;key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let core = *oteryn_gameplay_domain::domain::soulpit::SOUL_CORE_SOURCE_IDS
        .first()
        .ok_or("step=content;needed=soul-core-source-id")?;
    let index = provisioning::item_index()?;
    let definition = provisioning::item_reference(&index, core)?;
    let mut client = connect_world(clients, 0xe7).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let route_to_slot = route(world, here, |at| (at.x, at.y, at.floor) == ENTRY_SLOT);
    let walked = if let Some(path) = &route_to_slot {
        walk(&mut client, world, path).await?;
        true
    } else {
        false
    };
    let now = tile(client.world_spatial().actor_position)?;
    let at_slot = (now.x, now.y, now.floor) == ENTRY_SLOT;
    let payload = world_object::encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(|error| format!("step=encode;observed={error:?}"))?;
    let response = client
        .session_mut()
        .world_context_use(&payload)
        .await?;
    let disposition = world_object::decode_use_result(&response.payload)
        .map_err(|error| format!("step=use;observed=decode:{error:?}"))?;
    logout(&mut client).await?;
    if response.status != CommandStatus::Accepted || disposition != expected_refusal(at_slot) {
        return Err(format!(
            "step=use;observed=status:{:?},disposition:{disposition:?},at_slot:{at_slot},walked:{walked};needed=refusal:{:?}",
            response.status,
            expected_refusal(at_slot)
        )
        .into());
    }
    Ok(format!("use={disposition:?};at_entry_slot={at_slot}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_depends_on_the_entry_slot() {
        assert_eq!(expected_refusal(true), UseDisposition::StaleState);
        assert_eq!(expected_refusal(false), UseDisposition::TooFar);
    }

    #[test]
    fn placement_key_packs_tile_depth_and_ordinal() {
        assert_eq!(
            placement_key(0x0102, 0x0304, -8, 5),
            Some(0x0102_0000_0000 | 0x0304_0000 | 0x0800 | 5)
        );
        assert_eq!(placement_key(1, 1, 3, 0), None);
        assert_eq!(token("a  b"), "a-b");
    }
}
