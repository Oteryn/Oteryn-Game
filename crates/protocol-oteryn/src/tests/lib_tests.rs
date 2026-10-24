// Included files share this module to preserve test paths and private helper access.
#[path = "capability_selection_tests.rs"]
mod capability_selection_tests;

use super::*;

include!("codec_seam_tests.rs");
include!("support.rs");
include!("handshake_tests.rs");
include!("client_message_tests.rs");
include!("envelope_tests.rs");
include!("error_registry_tests.rs");
include!("command_tests.rs");
include!("state_sync_tests.rs");
