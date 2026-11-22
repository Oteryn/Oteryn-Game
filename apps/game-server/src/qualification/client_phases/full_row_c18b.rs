//! C18b full-row journey, derived from the matrix/audit and the native-combat owner.
//!
//! Player scenario:
//! 1. Admit the ordinary node-boot character, walk from town to a non-protection-zone tile, and
//!    obtain the five crosshair spell indices from the catalogue served by this world.
//! 2. Send a native POSITION cast for Death Echo, Divine Grenade, Divine Barrage, Ethereal
//!    Barrage, and Thousand Fist Blows.  For every command assert an accepted command frame; a
//!    successful cast must also debit the exact mana/soul cost and advance server sequence.
//! 3. Send one more real POSITION cast beyond that spell's range.  Assert TargetIllegal and no
//!    actor-vitals delta (the refusal path must not charge the player).
//! 4. Logout, obtain a fresh admission, and assert a new game session retains position and the
//!    last server-acknowledged vitals.  Spell aiming itself creates no additional durable state.

use super::*;
use oteryn_dev_client::{CastOutcome, DevClientSession};
use oteryn_protocol_oteryn::actor_spell::{
    ActorVitals, SpellCastDisposition, SpellTarget, SpellTargetPosition,
};

const SPELLS: [&str; 5] = [
    "exevo mort ora",
    "exevo tempo mas san",
    "exori dir san",
    "exori dir moe",
    "exori mas amp pug",
];

struct Attempt {
    started: std::time::Instant,
    step: &'static str,
    observed: String,
}

impl Attempt {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            step: "admission",
            observed: "not-reached".into(),
        }
    }

    fn at(&mut self, step: &'static str) {
        self.step = step;
        self.observed = "pending".into();
    }

    fn observed(&mut self, value: impl std::fmt::Debug) {
        self.observed = format!("{value:?}")
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || "_-.,:=[]".contains(character) {
                    character
                } else {
                    '-'
                }
            })
            .collect();
    }

    fn finish(self, result: TestResult) {
        let (status, evidence_value) = match result {
            Ok(()) => ("PASS", "position-casts-refusal-fresh-admission".into()),
            Err(error) => {
                let error = error.to_string().replace(['\n', '\r'], " ");
                evidence(&format!(
                    "C18B_GAP step={} observed={} need={error}",
                    self.step, self.observed
                ));
                let error: String = error
                    .chars()
                    .map(|character| {
                        if character.is_ascii_alphanumeric() || "_-.,:=[]/".contains(character) {
                            character
                        } else {
                            '-'
                        }
                    })
                    .collect();
                (
                    "FAIL",
                    format!("step={};observed={};need={error}", self.step, self.observed),
                )
            }
        };
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C18b.full status={status} seconds={:.3} evidence={evidence_value}",
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut attempt = Attempt::new();
    let result = verdict::indexed_timeout(
        Duration::from_secs(180),
        journey(&mut attempt, clients),
    )
    .await
    .map_err(|_| "journey-timeout-needs-server-reply".into())
    .and_then(|result| result);
    attempt.finish(result);
}

fn spell_book() -> TestResult<crate::spell::SpellBook> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
        &root.join("content/spells.manifest.json"),
    )?;
    let catalog =
        crate::spell::executable_catalog::compile(&input.catalog.bytes, &input.catalog.sha256)?;
    let selections = crate::spell::executable_catalog::compile_source_selection(
        &input.source_selection.bytes,
        &input.source_selection.sha256,
        &catalog,
    )?;
    Ok(catalog.into_spell_book_with_selections(&selections)?)
}

fn spell_index(
    book: &crate::spell::SpellBook,
    words: &str,
) -> TestResult<std::num::NonZeroU32> {
    let key = &book
        .spoken(words)
        .ok_or_else(|| format!("served-catalogue-missing-{words}"))?
        .spell
        .key;
    (1..=book.source_len())
        .filter_map(|index| {
            u32::try_from(index)
                .ok()
                .and_then(std::num::NonZeroU32::new)
        })
        .find(|index| book.indexed(*index).is_some_and(|spell| &spell.key == key))
        .ok_or_else(|| format!("served-catalogue-has-no-index-for-{words}").into())
}

async fn ready(client: &mut DevClientSession, micros: u64) -> TestResult {
    let deadline = tokio::time::Instant::now()
        + Duration::from_micros(micros.saturating_add(200_000));
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

fn cast_payment(
    outcome: &CastOutcome,
    before: ActorVitals,
    mana: u32,
    soul: u32,
) -> bool {
    outcome.actor_vitals_delta.as_ref().is_some_and(|delta| {
        delta.server_sequence > outcome.result_server_sequence
            && before.mana.checked_sub(mana) == Some(delta.value.mana)
            && before.soul.checked_sub(soul) == Some(delta.value.soul)
    })
}

fn crosshair_plan(
    book: &crate::spell::SpellBook,
    vocation: crate::spell::Vocation,
) -> TestResult<Vec<&'static str>> {
    let mut plan = Vec::new();
    for words in SPELLS {
        let spell = book.spoken(words)
            .ok_or_else(|| format!("served-catalogue-missing-{words}"))?.spell;
        if spell.vocations.contains(&vocation) {
            plan.push(words);
        }
    }
    Ok(plan)
}

fn require_crosshair_casts(successful: usize, refusals: &[String]) -> TestResult {
    if !refusals.is_empty() {
        return Err(format!(
            "POSITION cast coverage {successful}/{}; missing={}; source:crates/gameplay/dispatch/src/gameplay_transport/ordinary_combat/prepare.rs:282",
            SPELLS.len(),
            refusals.join(", ")
        ).into());
    }
    Ok(())
}

fn outside_protection_zone(world: &BundleWorld, at: TilePos) -> bool {
    world
        .base()
        .tile(at.x, at.y, at.floor)
        .is_some_and(|tile| !tile.protection_zone())
}

fn crosshair_tile(world: &BundleWorld, from: TilePos) -> Option<TilePos> {
    [StepDirection::East, StepDirection::North, StepDirection::West, StepDirection::South]
        .into_iter()
        .filter_map(|direction| destination(world, from, direction))
        .find(|at| at.floor == from.floor && outside_protection_zone(world, *at))
}

async fn journey(attempt: &mut Attempt, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let book = spell_book()?;
    let mut client = connect_character(clients, 0xc8, 1).await?;
    attempt.observed(client.join_snapshot().game_session_id);

    attempt.at("crosshair-vocation-plan");
    let vocation = full_e2ef5::current_vocation(&mut client).await?;
    let plan = crosshair_plan(&book, vocation)?;
    let mut gaps = Vec::new();
    for words in SPELLS.into_iter().filter(|words| !plan.contains(words)) {
        let spell = book.spoken(words).ok_or("crosshair spell absent")?.spell;
        gaps.push(format!("{words} requires a separate admitted vocation {:?}; served={vocation:?}", spell.vocations));
    }
    if plan.is_empty() {
        attempt.observed(&gaps);
        full_e2ef5::player_logout(&mut client).await?;
        return require_crosshair_casts(0, &gaps);
    }

    attempt.at("walk-out-of-pz");
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        outside_protection_zone(world, at) && crosshair_tile(world, at).is_some()
    })
    .ok_or("needs-reachable-non-pz-tile-with-adjacent-position-target")?;
    walk(&mut client, world, &path).await?;
    let here = client.world_spatial().actor_position;
    attempt.observed(here);
    let target_tile = crosshair_tile(world, tile(here)?)
        .ok_or("adjacent-position-target-absent-at-acknowledged-caster-position")?;
    let target = SpellTarget::Position(SpellTargetPosition {
        x: i32::from(target_tile.x),
        y: i32::from(target_tile.y),
        floor: here.floor,
    });
    let mut successful = None;
    let mut successful_count = 0;
    for words in plan {
        attempt.at("crosshair-position-casts");
        let spell = book
            .spoken(words)
            .ok_or_else(|| format!("served-catalogue-missing-{words}"))?
            .spell;
        ready(
            &mut client,
            spell
                .groups
                .iter()
                .map(|group| group.cooldown_micros)
                .max()
                .unwrap_or(0)
                .max(spell.cooldown_micros),
        )
        .await?;
        let before = client.actor_vitals().copied().ok_or("actor-vitals-absent")?;
        let outcome = client
            .cast_spell(spell_index(&book, words)?, target, false)
            .await?;
        attempt.observed((words, outcome.status, outcome.disposition));
        if outcome.status != CommandStatus::Accepted {
            return Err(format!("needs-accepted-native-command-frame-for-{words}").into());
        }
        if outcome.disposition == SpellCastDisposition::Cast {
            let crate::spell::ManaCost::Fixed(mana) = spell.mana else {
                return Err("needs-fixed-cost-for-payment-assertion".into());
            };
            if !cast_payment(&outcome, before, mana, spell.soul) {
                return Err(format!("needs-exact-vitals-delta-for-{words}").into());
            }
            successful.get_or_insert((spell, spell_index(&book, words)?));
            successful_count += 1;
            evidence(&format!(
                "C18B_CHECK C18b POSITION PASS words={words:?} vocation={vocation:?} server_sequence={}",
                outcome.result_server_sequence
            ));
        } else {
            if outcome.actor_vitals_delta.is_some() {
                return Err(format!("refused-{words}-changed-vitals").into());
            }
            gaps.push(format!("{words}:{:?}", outcome.disposition));
        }
    }
    attempt.observed(&gaps);
    let Some((spell, index)) = successful else {
        full_e2ef5::player_logout(&mut client).await?;
        return require_crosshair_casts(0, &gaps);
    };

    attempt.at("out-of-range-refusal");
    ready(
        &mut client,
        spell
            .groups
            .iter()
            .map(|group| group.cooldown_micros)
            .max()
            .unwrap_or(0)
            .max(spell.cooldown_micros),
    )
    .await?;
    let refused = client
        .cast_spell(
            index,
            SpellTarget::Position(SpellTargetPosition {
                x: here.x.saturating_add(100),
                y: here.y,
                floor: here.floor,
            }),
            false,
        )
        .await?;
    attempt.observed((refused.status, refused.disposition));
    if refused.status != CommandStatus::Accepted
        || refused.disposition != SpellCastDisposition::TargetIllegal
        || refused.actor_vitals_delta.is_some()
    {
        return Err("needs-target-illegal-without-vitals-delta".into());
    }

    attempt.at("fresh-admission");
    let retained_position = client.world_spatial().actor_position;
    let retained_vitals = client.actor_vitals().copied().ok_or("actor-vitals-absent")?;
    let old_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout-refused".into());
    }
    drop(client);
    let client = connect_character(clients, 0xc9, 1).await?;
    attempt.observed(client.join_snapshot().game_session_id);
    if client.join_snapshot().game_session_id == old_session
        || client.world_spatial().actor_position != retained_position
        || client.actor_vitals().copied() != Some(retained_vitals)
    {
        return Err("needs-new-session-with-retained-position-and-vitals".into());
    }
    attempt.at("crosshair-position-casts");
    attempt.observed(&gaps);
    require_crosshair_casts(successful_count, &gaps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crosshair_plan_only_attempts_spells_for_the_admitted_vocation() -> TestResult {
        use crate::spell::Vocation;
        let book = spell_book()?;
        assert_eq!(crosshair_plan(&book, Vocation::Sorcerer)?, ["exevo mort ora"]);
        assert_eq!(
            crosshair_plan(&book, Vocation::Paladin)?,
            ["exevo tempo mas san", "exori dir san", "exori dir moe"]
        );
        assert_eq!(crosshair_plan(&book, Vocation::Monk)?, ["exori mas amp pug"]);
        assert!(crosshair_plan(&book, Vocation::Knight)?.is_empty());
        Ok(())
    }

    #[test]
    fn partial_crosshair_casts_leave_a_named_gap() {
        let refused = "exori mas amp pug:LevelTooLow".to_owned();
        let result = require_crosshair_casts(SPELLS.len() - 1, std::slice::from_ref(&refused));
        assert!(result.is_err_and(|error| error.to_string().contains(&refused)));
        assert!(require_crosshair_casts(SPELLS.len(), &[]).is_ok());
    }

    #[test]
    fn required_crosshair_spells_are_indexed() -> TestResult {
        let book = spell_book()?;
        for words in SPELLS {
            let spell = book.spoken(words).ok_or("crosshair spell absent")?.spell;
            spell_index(&book, words)?;
            assert!(spell.range_tiles.is_some());
        }
        Ok(())
    }
}
