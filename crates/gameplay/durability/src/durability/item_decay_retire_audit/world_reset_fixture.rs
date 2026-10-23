// Test-only wrappers reuse the existing private builders without copying them.
pub(super) fn world_reset_entry_fixture() -> OneItemDecayRetireV1 {
    entry_step()
}
pub(super) fn world_reset_root_fixture() -> OneItemDecayRetireV1 {
    corpse_step()
}
pub(super) fn world_reset_uuid_fixture(seed: u8) -> [u8; 16] {
    fixed(seed)
}
