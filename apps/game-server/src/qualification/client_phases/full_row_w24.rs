//! Full W24 journey uses the same capture and replay machinery as the client row.
use super::*;
use oteryn_dev_client::DevClientSession;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match qualification_world(clients) {
        Ok(world) => client_row_w24::run_journey(clients, world, true).await,
        Err(error) => evidence(&format!(
            "e2e_all scenario=W24.full status=FAIL evidence={error}"
        )),
    }
}

pub(super) fn clock_snapshot(client: &mut DevClientSession) -> TestResult<u16> {
    use oteryn_protocol_oteryn::world_clock::{
        SNAPSHOT_TYPE_WORLD_CLOCK_V1, STATE_DOMAIN_WORLD_CLOCK, decode_world_clock,
    };
    let snapshot = client
        .session_mut()
        .gated_snapshot(STATE_DOMAIN_WORLD_CLOCK)
        .ok_or("step=world-clock-admission observed=missing-snapshot need=tibian-minute")?;
    if snapshot.snapshot_type != SNAPSHOT_TYPE_WORLD_CLOCK_V1 {
        return Err("step=world-clock-admission observed=wrong-snapshot-type".into());
    }
    decode_world_clock(&snapshot.payload)
        .map(|clock| clock.tibian_minute)
        .map_err(|error| format!("step=world-clock-admission observed={error:?}").into())
}

type TimedCue = (u32, u16);

pub(super) fn zone(world: &BundleWorld, at: TilePos) -> Option<&str> {
    let table = &world.base().subareas;
    let (_, id) = table.resolve(at.x, at.y, at.floor)?;
    let key = table.areas.iter().find(|area| area.id == id)?.key.as_str();
    let key = key.strip_prefix("oteryn:content.area.")?;
    let city = key
        .strip_prefix("city.")
        .or_else(|| key.strip_prefix("subregion.")?.strip_suffix("_city"))?;
    matches!(
        city,
        "venore" | "thais" | "carlin" | "edron" | "kazordoon" | "port_hope"
    )
    .then_some(city)
}

pub(super) fn city_replacement(city: &str, cues: &[TimedCue]) -> bool {
    let (day, night) = match city {
        "venore" => (23, 28),
        "thais" | "carlin" => (25, 28),
        "edron" => (3, 36),
        "kazordoon" => (15, 41),
        "port_hope" => (11, 8),
        _ => return false,
    };
    let [(cue, minute)] = cues else {
        return false;
    };
    {
        let daylight = (crate::world_clock::SUNRISE_START..crate::world_clock::SUNSET_START)
            .contains(&u32::from(*minute));
        *cue == if daylight { day } else { night }
    }
}

fn qualification_world(clients: &SeamClients<'_>) -> TestResult<&'static BundleWorld> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journey_regression_requires_the_destination_city_cue() {
        assert!(city_replacement("edron", &[(3, 360)]));
        assert!(city_replacement("edron", &[(36, 1050)]));
        assert!(!city_replacement("edron", &[(25, 360)]));
        assert!(!city_replacement("edron", &[(36, 360)]));
        assert!(!city_replacement("edron", &[(3, 1050)]));
        assert!(!city_replacement("edron", &[(3, 360), (0, 360)]));
        assert!(!city_replacement("edron", &[(0, 360), (3, 360)]));
    }
}
