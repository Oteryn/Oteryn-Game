use super::super::{PlayerMetadataState, metadata_request};
use oteryn_client_runtime::{
    combat::EntityRef,
    cui3::battle_metadata::{PlayerMetadata, decode_query, encode_reply},
    server_mechanic::{CommandStatus, ServerMechanicResponse},
};
#[test]
fn battle_metadata_rejects_unrequested_actors_and_rotates_bounded_batches()
-> Result<(), &'static str> {
    let now = std::time::Instant::now();
    let entity = |id| EntityRef {
        identity: [id; 16],
        generation: 1,
    };
    let refs = (1..=17).map(entity).collect::<Vec<_>>();
    let mut state = PlayerMetadataState::new(true);
    let first = metadata_request(&mut state, &refs, now).ok_or("query")?;
    let requested = decode_query(&first.payload).ok_or("request")?;
    assert_eq!(requested.len(), 16);
    assert!(metadata_request(&mut state, &refs, now).is_none());
    state.finish(&ServerMechanicResponse {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: encode_reply(&[PlayerMetadata {
            entity: entity(20),
            vocation: Some(4),
            same_guild: Some(true),
        }])
        .ok_or("reply")?,
    });
    assert!(state.values.is_empty());
    assert!(state.error.is_some());
    let next = metadata_request(&mut state, &refs, now + std::time::Duration::from_secs(1))
        .ok_or("next query")?;
    assert!(
        decode_query(&next.payload)
            .ok_or("next request")?
            .contains(&entity(17))
    );
    Ok(())
}
