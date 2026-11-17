//! P10 client capture of a live resting-area credit after an ordinary spell debit.
use super::*;
use oteryn_dev_client::client_runtime::daily_reward::{
    DailyRewardCommand, DailyRewardReply, decode_reply,
};
use oteryn_protocol_oteryn::daily_reward as wire;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "P10_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn resting_soul_observed(before: u32, after: u32) -> bool {
    before.checked_add(1) == Some(after)
}

#[test]
fn live_resting_soul_credit_qualifies_at_zero_streak() {
    assert!(!resting_soul_observed(5, 5));
    assert!(resting_soul_observed(5, 6));
}

fn resting_projection_observed(inside: bool, mana_multiplier: u8, soul: bool) -> bool {
    inside && (mana_multiplier > 0 || soul)
}

#[test]
fn resting_projection_requires_an_enabled_bonus_inside_the_area() {
    assert!(!resting_projection_observed(true, 0, false));
    assert!(!resting_projection_observed(false, 1, false));
    assert!(resting_projection_observed(true, 1, false));
    assert!(resting_projection_observed(true, 2, true));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_protocol_oteryn::{
        actor_spell as spell, actor_spell_catalog as catalogue, rest_status,
    };
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x2c).await?;
    let query = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_REWARD_V2,
            wire::COMMAND_TYPE_DAILY_REWARD_QUERY,
            &[],
        )
        .await?;
    if query.status != CommandStatus::Accepted {
        return Err(format!("wall query status={:?}", query.status).into());
    }
    capture("p10-wall.bin", &query.payload);
    let DailyRewardReply::Wall(wall) = decode_reply(DailyRewardCommand::Query, &query.payload)?
    else {
        return Err("wall query did not decode as a wall".into());
    };
    let path = route(
        world,
        tile(client.world_spatial().actor_position)?,
        |position| {
            world
                .base()
                .tile(position.x, position.y, position.floor)
                .is_some_and(|tile| tile.protection_zone())
        },
    )
    .ok_or("step=resting-area no reachable protection zone")?;
    walk(&mut client, world, &path).await?;
    let status_reply = client
        .session_mut()
        .gated_command(
            rest_status::CAPABILITY_REST_STATUS_V1,
            rest_status::COMMAND_TYPE_REST_STATUS_QUERY,
            &[],
        )
        .await?;
    let status = rest_status::RestStatus::decode(&status_reply.payload)
        .filter(|status| {
            status_reply.status == CommandStatus::Accepted
                && status.streak == wall.streak
                && resting_projection_observed(
                    status.inside_resting_area,
                    status.mana_multiplier,
                    status.soul,
                )
        })
        .ok_or_else(|| format!("step=resting enabled status absent: {status_reply:?}"))?;
    capture("p10-rest-status.bin", &status_reply.payload);
    let snapshot = client
        .session_mut()
        .gated_snapshot(catalogue::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("step=spell missing actor spell catalogue")?;
    let spells = catalogue::decode_catalog(&snapshot.payload)
        .map_err(|error| format!("step=spell catalogue decode {error:?}"))?;
    let before = *client.actor_vitals().ok_or("step=spell vitals absent")?;
    let soul_spell = spells.iter().find(|entry| {
        status.soul
            && entry.available
            && entry.soul > 0
            && entry.soul <= before.soul
            && entry.rune_item.is_none()
            && std::num::NonZeroU32::new(entry.cast_index)
                .and_then(|index| native_spell_book().ok()?.indexed(index))
                .is_some_and(|spell| !spell.aggressive && !spell.takes_target())
    });
    let use_soul = soul_spell.is_some();
    let learned = soul_spell
        .or_else(|| {
            spells.iter().find(|entry| {
                status.mana_multiplier > 0
                    && entry.available
                    && entry.words == "exura"
                    && entry.mana > 0
            })
        })
        .ok_or("step=spell no learned source for a resting-area deficit")?;
    let pool = |vitals: spell::ActorVitals| if use_soul { vitals.soul } else { vitals.mana };
    let stat = if use_soul { "soul" } else { "mana" };
    let cast = client
        .cast_spell(
            std::num::NonZeroU32::new(learned.cast_index).ok_or("step=spell invalid cast index")?,
            spell::SpellTarget::None,
            false,
        )
        .await?;
    let spent = cast
        .actor_vitals_delta
        .as_ref()
        .map(|delta| delta.value)
        .ok_or("step=spell missing vitals debit")?;
    if cast.disposition != spell::SpellCastDisposition::Cast || pool(spent) >= pool(before) {
        return Err(format!("step=spell {stat} debit not observed: {cast:?}").into());
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(135);
    verdict::Harness::global().bounded_wait(deadline, "resting-credit");
    let restored = tokio::time::timeout_at(deadline, async {
        loop {
            for event in client.take_events() {
                if let oteryn_dev_client::SessionEvent::ActorVitals(delta) = event
                    && delta.server_sequence > cast.result_server_sequence
                    && if use_soul {
                        resting_soul_observed(spent.soul, delta.value.soul)
                    } else {
                        delta.value.mana > spent.mana && delta.value.mana <= spent.max_mana
                    }
                {
                    return Ok::<_, oteryn_dev_client::DevClientError>(delta.value);
                }
            }
            client.service_liveness(Duration::from_millis(100)).await?;
        }
    })
    .await
    .map_err(|_| {
        format!(
            "step=resting no {stat} credit after debit; streak={}",
            wall.streak
        )
    })??;
    for (name, vitals) in [("p10-spent.bin", spent), ("p10-restored.bin", restored)] {
        capture(
            name,
            &spell::encode_actor_vitals(&vitals)
                .map_err(|error| format!("vitals encode: {error:?}"))?,
        );
    }
    Ok(format!(
        "streak={};lane_day={};{stat}={}..{}",
        wall.streak,
        wall.lane_day,
        pool(spent),
        pool(restored)
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(210), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=P10.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P10.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P10.client status=FAIL evidence=timeout"),
    }
}
