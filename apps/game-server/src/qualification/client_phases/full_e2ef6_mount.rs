{
    #[path = "full_e2ef6.rs"]
    mod full_e2ef6;
    // Starter custody must be observed before profile_journey's fixture provisioning.
    full_e2ef6::run(clients, &world, &metadata.spawns, &metadata.npcs).await;
}
