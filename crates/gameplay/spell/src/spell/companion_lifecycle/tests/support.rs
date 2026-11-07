fn writers() -> (FamiliarLifecycleWriter, FamiliarLifecycleWriter) {
    let old_writer = FamiliarLifecycleWriter {
        game_session_id: "0199aabb-1234-7000-8000-000000000001".into(),
        connection_generation: 1,
        world_id: "0199aabb-1234-7000-8000-000000000002".into(),
        channel_id: "0199aabb-1234-7000-8000-000000000003".into(),
        scope_ownership_generation: 4,
    };
    let current_writer = FamiliarLifecycleWriter {
        game_session_id: "0199aabb-1234-7000-8000-000000000004".into(),
        ..old_writer.clone()
    };
    (old_writer, current_writer)
}

fn lifecycle(
    writer: &FamiliarLifecycleWriter,
    remaining: u64,
    open: bool,
    marker: Option<&FamiliarLifecycleWriter>,
) -> FamiliarLifecycleV2 {
    FamiliarLifecycleV2 {
        version: 2,
        familiar_remaining_ms: remaining,
        open,
        writer: writer.clone(),
        clean_end_game_session_id: marker.map(|w| w.game_session_id.clone()),
    }
}
