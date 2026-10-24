//! Single integration-test binary for the Merge Queue `integration-rest` shard.
//!
//! Each module below was a separate `tests/<name>.rs` target. Merging them links the whole
//! `oteryn-game-server` crate once instead of once per file. Targets that need their own
//! process, path-include crate-private `src` modules, or are named by a workflow, validator or
//! tool stay as separate `tests/*.rs` files. `content_world_project_publication` is one: its
//! `flock` cases fail when a test in the same process spawns a child that briefly holds a copy of
//! the lock descriptor.

mod admission_evidence_wire;
mod authority_invariants;
mod content_first_production;
mod content_item_fx_audio_raw;
mod content_native_entry;
mod content_reference_playable;
mod content_world_cw2_b4_import;
mod content_world_project;
mod content_world_project_fs;
mod content_world_project_v2_encounter_admission;
mod content_world_project_v2_npc_admission;
mod diagnostic_line;
mod evidence_shell;
mod loop_progress_level_up;
mod map_cutover_boot;
mod map_cutover_view;
mod map_overlay_channel;
mod native_account_characters_transport;
mod native_runtime_status;
mod native_scope_assignment;
mod npc_placement_parity;
mod reference_character_progression_calc;
mod server_ci_qualification;
mod spell_lane_key33_pin;
mod world_reward_claims_binding;
