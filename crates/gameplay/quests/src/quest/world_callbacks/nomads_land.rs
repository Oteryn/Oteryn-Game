//! Chosen Oteryn s4 USE binding, not an assertion of donor Lua behavior.
//! The three-crate/amphora rule is the pinned Nomads Land walkthrough. The exact
//! window, clear staging tiles and loose objects are checked against the current
//! B3 World Project; evidence lives in docs/reference/content-prep/qmech/nomads-land-window.json.
use super::Callback;

pub(super) fn callback() -> Result<Callback, String> {
    super::parse(PROFILE)?
        .pop()
        .ok_or_else(|| "Nomads Land window profile".into())
}

const PROFILE: &str = r#"{
  "schema": 1,
  "callbacks": [{
    "key": "oteryn:quest-callback/engenc2/nomads-land-window",
    "quest": "oteryn:quest.authored.nomads_land_quest",
    "trigger": "use",
    "selector": {"position":null,"unique":null,"action":null,"appearance":null,
      "native":{"position":{"x":33216,"y":32855,"floor":-9},"ordinal":1,"appearance":1744}},
    "lever":null,"reversible":false,"requires_world_admission":true,
    "passages":[],"teleport":null,
    "storage":[{"track":"oteryn:quest-progress/authored/nomads_land_quest/s4_window_obscured",
      "minimum":-1,"maximum":0,"set":1}],
    "world_requirements":[
      {"position":{"x":33215,"y":32854,"floor":-9},"ground":{"stacks":[
        {"item":"oteryn:item.tibia.i2471","revision":"definition-r1","quantity":1}]}},
      {"position":{"x":33216,"y":32854,"floor":-9},"ground":{"stacks":[
        {"item":"oteryn:item.tibia.i2471","revision":"definition-r1","quantity":1},
        {"item":"oteryn:item.tibia.i2904","revision":"definition-r1","quantity":1}]}},
      {"position":{"x":33217,"y":32854,"floor":-9},"ground":{"stacks":[
        {"item":"oteryn:item.tibia.i2471","revision":"definition-r1","quantity":1}]}}
    ],
    "offerings":[],"return_on_failure":false,"failure_message":null,
    "sources":[
      "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/items/items.xml:3817,5020,6245",
      "https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml#L3817",
      "https://tibia.fandom.com/index.php?curid=22842&oldid=1002600",
      "https://github.com/Lavovskii/grimorio/blob/dbeee0138a54b30a74dd690c4a7e6fab514c8db0/_questski/tools/briefs/nomads-land-quest.md"
    ]
  }]
}"#;
