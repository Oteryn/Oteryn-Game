//! Exact bounded XML DATA. The physical Spawner remains the timer/actor owner.
use super::project::native_spell_world::PinnedActivitySourceRoom;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::content) struct AmbientXmlSpawn {
    pub position: [i32; 3],
    pub radius: u16,
    pub respawn_seconds: u64,
    pub creature_key: String,
    pub source_path: String,
    pub source_sha256: String,
    pub line: u64,
}

pub(in crate::content) fn expected(room: PinnedActivitySourceRoom) -> Option<serde_json::Value> {
    use PinnedActivitySourceRoom as R;
    let (keys, count): (&[&str], usize) = match room {
        R::GoshnarsSpite => (
            &[
                "oteryn:creature.weeping_soul",
                "oteryn:creature.spiteful_spitter",
            ],
            3,
        ),
        R::GoshnarsHatred => (&["oteryn:creature.symbol_of_hatred"], 2),
        R::GoshnarsCruelty => (&["oteryn:creature.poor_soul"], 1),
        R::GoshnarsMegalomania => (
            &[
                "oteryn:creature.necromantic_focus",
                "oteryn:creature.lesser_splinter_of_madness",
            ],
            3,
        ),
        R::CallbackRagiaz => (&["oteryn:creature.bone_capsule"], 1),
        _ => return None,
    };
    let capture: Vec<serde_json::Value> = serde_json::from_slice(include_bytes!(
        "../../../../../tools/content-schema/native-gameplay/srx3-spite-spawns.json"
    ))
    .ok()?;
    let rows: Vec<_> = capture
        .into_iter()
        .filter(|row| {
            row.get("creature_key")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|key| keys.contains(&key))
        })
        .collect();
    if rows.len() != count {
        return None;
    }
    for row in &rows {
        let typed: AmbientXmlSpawn = serde_json::from_value(row.clone()).ok()?;
        if typed.radius != 1
            || typed.line == 0
            || typed.source_path != "data-global/world/world-monster.xml"
            || typed.source_sha256
                != "f90e82203a112c9068d4ed123e3e5bb4c1b9743f2c2d3bcb12039ac8ebd9ad4e"
            || typed.respawn_seconds != if room == R::CallbackRagiaz { 60 } else { 10 }
        {
            return None;
        }
    }
    Some(serde_json::Value::Array(rows))
}
