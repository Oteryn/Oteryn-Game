use super::*;
use oteryn_client_runtime::analyser::decode_analyser_facts;
use oteryn_client_runtime::combat::EntityRef;

fn hit(attacker: Option<EntityRef>) -> Result<AttributedFact, &'static str> {
    // One received hit of 20 Fire damage from a player, as decoded from the wire.
    let facts = decode_analyser_facts(&[10, 8, 42, 6, 8, 20, 16, 2, 24, 2])
        .map_err(|_| "damage input wire")?;
    let fact = facts.into_iter().next().ok_or("one fact")?;
    Ok(AttributedFact { fact, attacker })
}

fn attacker() -> EntityRef {
    EntityRef {
        identity: [9; 16],
        generation: 1,
    }
}

#[test]
fn c0486_an_attributed_hit_with_an_attack_target_flashes() -> Result<(), &'static str> {
    assert!(own_hit_flash(&[hit(Some(attacker()))?], true));
    Ok(())
}

#[test]
fn c0486_without_an_attack_target_nothing_flashes() -> Result<(), &'static str> {
    assert!(!own_hit_flash(&[hit(Some(attacker()))?], false));
    Ok(())
}

#[test]
fn c0486_a_hit_without_an_attacker_does_not_flash() -> Result<(), &'static str> {
    assert!(!own_hit_flash(&[hit(None)?], true));
    assert!(!own_hit_flash(&[], true));
    Ok(())
}

#[test]
fn c0486_one_attributed_hit_among_other_facts_flashes_and_facts_stay_plain()
-> Result<(), &'static str> {
    let kill = AttributedFact {
        fact: AnalyserFact::Kill { race: None },
        attacker: None,
    };
    let batch = [kill.clone(), hit(Some(attacker()))?];
    assert!(own_hit_flash(&batch, true));
    assert_eq!(plain_facts(&batch)[0], kill.fact);
    assert_eq!(plain_facts(&batch).len(), 2);
    Ok(())
}

#[test]
fn c0486_the_frame_blinks_twice_then_goes() {
    use std::time::Duration;
    let ms = Duration::from_millis;
    assert!(hit_flash_visible(ms(0)));
    assert!(hit_flash_visible(ms(99)));
    assert!(!hit_flash_visible(ms(100)));
    assert!(!hit_flash_visible(ms(199)));
    assert!(hit_flash_visible(ms(200)));
    assert!(hit_flash_visible(ms(299)));
    assert!(!hit_flash_visible(HIT_FLASH_LIFETIME));
    assert!(!hit_flash_visible(ms(5000)));
}
