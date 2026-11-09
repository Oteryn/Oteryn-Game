//! Closed chosen-recipe profile, not a donor-equivalence claim.
//! content/quests/questpc7-world-callbacks.json:6127,6708 at 2caf66704f735bde02cb68370a5b5e00dd2d87a6,
//! entry/purification-entry; Feaster Wiki oldid421868, recipe s2/s6.
use super::{Callback, Position};

pub(super) fn admitted(callback: &Callback) -> bool {
    let (phase, receipt, destination, message) = match callback.key.as_str() {
        "oteryn:quest-callback/questpc7-feaster-entry" => (
            0,
            "entry",
            (32407, 31788, -7),
            "Ring the Death Knell at the three marked grave sites.",
        ),
        "oteryn:quest-callback/questpc7-feaster-purification-entry" => (
            5,
            "purification_entry",
            (31917, 32354, -8),
            "Collect souls at the fixed rift and purify them at the nearby apparatus.",
        ),
        _ => return false,
    };
    // Match the complete chosen contract. This exception cannot admit arbitrary
    // Wiki-only callbacks, rewards, phases, destinations or replacement objects.
    let expected = serde_json::from_value::<Callback>(serde_json::json!({
        "key":callback.key,"quest":"oteryn:quest.feaster_of_souls_quest","trigger":"step_in",
        "selector":{"position":{"x":32627,"y":32076,"floor":-7},"appearance":231,"unique":null,"action":null},
        "lever":null,"reversible":false,"passages":[],
        "teleport":Position{x:destination.0,y:destination.1,floor:destination.2},
        "storage":[
            {"track":"oteryn:quest-progress/questpc7/feaster/programme","minimum":phase,"maximum":phase,"set":phase,"preserve":true},
            {"track":format!("oteryn:quest-progress/questpc7/feaster/{receipt}"),"minimum":-1,"maximum":-1,"set":1}
        ],
        "minimum_level":250,"admission":if phase == 5 { Some(serde_json::json!({"premium":true})) } else { None },"requires_world_admission":true,
        "offerings":[],"item_on_target":null,"return_on_failure":false,"failure_message":null,"success_message":message,
        "sources":[
            "canonical:content/quests/definitions/quests-00100-00199.json:1#/records/41/definition:sha256:57af470e521006a832fbc046639ee4a5a5a455b2c6552335afff0a0131783f2c",
            "https://www.tibiawiki.com.br/index.php?title=Feaster_of_Souls_Quest&oldid=421868",
            "https://www.tibiawiki.com.br/index.php?title=Para_Norbert&oldid=419674"
        ]
    }));
    expected.is_ok_and(|expected| expected == *callback)
}
