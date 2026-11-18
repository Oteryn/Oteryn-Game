{
    // Observe pristine starter custody before packet journeys can earn loot or move items.
    if groups.runs("f6") {
        full_e2ef6::run_starter(clients).await;
    }
    full_e2ef7::run_packet(clients, &groups).await;
    // Keep every remaining ordinary journey before profile fixture provisioning.
    if groups.runs("f6") {
        full_e2ef6::run(clients, world, &metadata.spawns, &metadata.npcs,
            groups.runs("rows") && e2e_row_selected(110),
            groups.runs("rows") && e2e_row_selected(111),
        ).await
    } else {
        (None, None)
    }
}
