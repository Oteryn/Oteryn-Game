//! Hunt journey over the real node; reference: `scenarios/s1_hunt.rs`.
use super::*;
pub(super) async fn hunt(_client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    // The entry room DOES spawn two rats, but default node boot has no native gameplay
    // generation. refresh_attack_weapon cannot load even fists without that generation,
    // and the entry rat has no loot table. Movement alone would not qualify a hunt.
    evidence("mechanic=hunt skipped=entry_room_has_no_native_combat_and_loot_content");
    Ok(())
}
