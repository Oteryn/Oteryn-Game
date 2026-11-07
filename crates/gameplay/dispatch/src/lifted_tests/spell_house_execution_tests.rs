#![allow(clippy::items_after_test_module)]
#![allow(clippy::unwrap_used)]

use crate::spell::house_execution::*;
#[test]
fn actual_room_cannot_invent_house_from_catalogue_or_character() {
    let (mut runtime, actor, session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_player(90);
    let room = crate::content::qualify_native_entry_room(runtime.binding().world_id()).unwrap();
    runtime.initialize_first_entry_position(actor).unwrap();
    let expected = runtime.read_actor_position(actor).unwrap();
    // Fixture helper has a different active pin than the qualified room;
    // neither that mismatch nor missing placement can become a House proof.
    assert!(current_house_presence(&room, &runtime, session, actor, expected).is_err());
}
