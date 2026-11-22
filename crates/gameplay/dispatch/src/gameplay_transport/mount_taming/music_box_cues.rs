use crate::content::native_gameplay::mount_taming::TamingRule;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use crate::map::facts::{BundleFacts, MovementSpeech};

const MUSIC_BOX: &str = "oteryn:item.tibia.i16244";
const SOUND_RED: u16 = 20;
const MONSTER_SAY: u8 = 36;

pub(super) fn capture(
    runtime: &ChannelRuntimeV1,
    target: ExactActorRef,
    rule: &TamingRule,
) -> Option<MovementSpeech> {
    if rule.item_key != MUSIC_BOX {
        return None;
    }
    // Crystal 201189f8, data/scripts/actions/items/music_box.lua:2–10.
    let utterance = match rule.creature_key.as_str() {
        "oteryn:creature.dragonling" => "FI?",
        "oteryn:creature.draptor" => "Screeeeeeeeech",
        "oteryn:creature.enraged_white_deer" => "*bell*",
        "oteryn:creature.ironblight" => "Plinngggg",
        "oteryn:creature.magma_crawler" => "ZzzZzzZzzzZz",
        "oteryn:creature.midnight_panther" => "Purrrrrrr",
        "oteryn:creature.panda" => "Rrrrr... Grrrr",
        "oteryn:creature.wailing_widow" => "Sssssssss",
        "oteryn:creature.wild_horse" => "*snort*",
        _ => return None,
    };
    MovementSpeech::capture_monster(runtime, target, MONSTER_SAY, utterance)
}

pub(super) fn publish(facts: &BundleFacts, speech: MovementSpeech) {
    facts.emit_movement_effect(speech.position, SOUND_RED);
    facts.emit_movement_speech(speech);
}
