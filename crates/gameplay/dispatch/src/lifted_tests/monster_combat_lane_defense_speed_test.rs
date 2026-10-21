#[cfg(test)]
#[allow(clippy::expect_used)]
mod defense_speed_test {
    use crate::content::ProjectReferenceRecord;
    use crate::content::ProjectV2AuthoringProfile;
    use crate::content::ProjectV2DefinitionRef as Ref;
    use crate::monster_combat_lane::native_invisibility::SelfSpeedSource;
    #[test]
    fn defense_speed_descriptor_census_uses_all268_native_phase_entries_once() {
        let packet:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/creature-speed/native-fixtures.json"))).expect("monster_combat_lane.rs:defense_speed_test:3954: qualified fixture operation must succeed");
        let records: Vec<ProjectReferenceRecord> =
            serde_json::from_value(packet["records"].clone()).expect("monster_combat_lane.rs:defense_speed_test:3956: qualified fixture operation must succeed");
        let profiles: Vec<ProjectV2AuthoringProfile> =
            serde_json::from_value(packet["authoring_profiles"].clone()).expect("monster_combat_lane.rs:defense_speed_test:3958: qualified fixture operation must succeed");
        let (r, _, _) = crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x59);
        for case in packet["cases"].as_array().expect("monster_combat_lane.rs:defense_speed_test:3960: qualified fixture operation must succeed") {
            let creature: Ref = serde_json::from_value(case["creature"].clone()).expect("monster_combat_lane.rs:defense_speed_test:3961: qualified fixture operation must succeed");
            let index = case["index"].as_u64().expect("monster_combat_lane.rs:defense_speed_test:3962: qualified fixture operation must succeed") as usize;
            let source =
                SelfSpeedSource::from_native(&r, &creature, index, &records, &profiles, [1; 32])
                    .expect("monster_combat_lane.rs:defense_speed_test:3965: qualified fixture operation must succeed");
            assert_eq!(
                source.base_speed(),
                case["base_speed"].as_u64().expect("monster_combat_lane.rs:defense_speed_test:3968: qualified fixture operation must succeed") as u16
            );
        }
    }
}
