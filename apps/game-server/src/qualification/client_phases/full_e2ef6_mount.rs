{
    // Observe pristine starter custody before packet journeys can earn loot or move items.
    if groups.runs("f6") {
        let before = journey_verdicts();
        full_e2ef6::run_starter(clients).await;
        journey_group_completed("f6_starter", before);
    }
    full_e2ef7::run_packet(clients, &groups).await;
    // Keep every remaining ordinary journey before profile fixture provisioning.
    if groups.runs("f6") {
        let before = journey_verdicts();
        full_e2ef6::run(clients, world, &metadata.spawns, &metadata.npcs).await;
        journey_group_completed("f6", before);
    }
}
