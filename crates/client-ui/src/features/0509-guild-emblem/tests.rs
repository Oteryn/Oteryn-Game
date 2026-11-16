use super::*;

#[test]
fn latest_emblems_replace_the_previous_list_and_resolve_per_entity() {
    let entity = |n: u8| EntityRef {
        identity: [n; 16],
        generation: 1,
    };
    let mut current = None;
    apply(
        &mut current,
        vec![GuildEmblem {
            entity: entity(1),
            emblem: Emblem::Ally,
        }],
    );
    let emblems = current.clone().unwrap_or_default();
    assert_eq!(emblem_of(&emblems, entity(1)), Some(Emblem::Ally));
    assert_eq!(emblem_of(&emblems, entity(2)), None);
    apply(&mut current, Vec::new());
    assert_eq!(current, Some(Vec::new()));
}
