use super::*;
use oteryn_client_ui::personal_map_marks::{PersonalMapMark, PersonalMapMarks};
fn treasure() -> PersonalMapMarks {
    PersonalMapMarks(vec![PersonalMapMark {
        key: "sterling".into(),
        x: 31948,
        y: 31924,
        floor: 7,
        text: "Treasure".into(),
    }])
}
#[test]
fn personal_quest_marks_are_not_editable_or_saved_in_world_only_cache()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let directory = fixture.path().join("questpc5-personal-map-privacy");
    let mut map = Minimap::default();
    map.root = Some(directory.clone());
    map.bind_world("11111111-1111-4111-8111-111111111111");
    map.binding = Some(([1; 32], [2; 32], 1));
    map.load()?;
    assert!(map.replace_quest_marks(treasure()));
    assert!(
        map.replace_quest_markers(&[oteryn_session::quest_log::QuestMapMark {
            x: 31948,
            y: 31924,
            floor: 7,
            text: "Treasure".into(),
        }])
    );
    map.bind_world("11111111-1111-4111-8111-111111111111");
    assert!(
        map.quest_markers.is_empty(),
        "admission also clears legacy Character facts"
    );
    assert!(
        map.quest_marks.0.is_empty(),
        "a new admission to the same World clears Character facts"
    );
    assert!(map.replace_quest_marks(treasure()));
    map.edit_marker(31948, 31924, 7, "Local label".into(), 0)?;
    assert_eq!(map.quest_marks, treasure());
    map.remove_marker(31948, 31924, 7)?;
    assert_eq!(map.quest_marks, treasure());
    map.edit_marker(10, 20, 7, "Home".into(), 1)?;
    let path = map.path().ok_or("path")?;
    let encoded = std::fs::read_to_string(path)?;
    assert!(!encoded.contains("Treasure"));
    assert!(!encoded.contains("quest_marks"));
    assert!(encoded.contains("Home"));
    let mut other = Minimap::default();
    other.root = Some(directory.clone());
    other.bind_world("11111111-1111-4111-8111-111111111111");
    other.binding = Some(([1; 32], [2; 32], 1));
    other.load()?;
    assert_eq!(other.markers.len(), 1);
    assert!(
        other.quest_marks.0.is_empty(),
        "another Character never inherits server facts from the World cache"
    );
    map.bind_world("22222222-2222-4222-8222-222222222222");
    assert!(map.quest_marks.0.is_empty());
    Ok(())
}
#[test]
fn malformed_personal_quest_marks_cannot_replace_confirmed_facts() {
    let mut map = Minimap::default();
    assert!(map.replace_quest_marks(treasure()));
    let mut invalid = treasure();
    invalid.0[0].text = "bad\0label".into();
    assert!(!map.replace_quest_marks(invalid));
    assert_eq!(map.quest_marks, treasure());
}
