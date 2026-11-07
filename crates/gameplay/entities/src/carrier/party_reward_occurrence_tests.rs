#![allow(clippy::expect_used)]
use super::*;
fn id(seed: u8) -> [u8; 16] {
    let mut id = [seed; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
#[test]
fn party_reward_occurrence_is_per_character_idempotent_and_bounded_to_fifty() {
    let mut f = CombatDeathFixture::new(
        WorldId::decode(&id(1)).expect("world"),
        ChannelId::decode(&id(2)).expect("channel"),
        ScopeOwnershipGeneration::new(1).expect("generation"),
    )
    .expect("fixture");
    f.strike("lethal", 20).expect("lethal");
    let actor = f.actor();
    let mut owner = f.borrow_combat_death();
    let lethal = owner.committed_lethal_receipt(actor).expect("receipt");
    owner.project_committed_lethal(lethal).expect("projection");
    let mut occurrences = std::collections::BTreeSet::new();
    for seed in 1..=50 {
        let character = CharacterId::decode(&id(seed)).expect("character");
        let (first, created) = owner
            .party_reward_occurrence(actor, character)
            .expect("new");
        assert!(created);
        assert!(occurrences.insert(first));
        assert_eq!(
            owner
                .party_reward_occurrence(actor, character)
                .expect("replay"),
            (first, false)
        );
    }
    assert_eq!(
        owner.party_reward_occurrence(actor, CharacterId::decode(&id(51)).expect("character")),
        Err(CarrierError::CapacityExceeded)
    );
}
