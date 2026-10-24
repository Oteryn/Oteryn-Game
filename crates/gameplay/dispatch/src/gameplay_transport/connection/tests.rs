// Included topic files share this module so test paths and private access stay stable.
#[path = "tests/capability_tests.rs"]
mod capability_tests;

mod bed_sleep_dispatch_tests {
    include!("../bed_sleep_dispatch_tests.rs");
}

use super::*;
use crate::gameplay_transport::world_spatial::encode_step_result;
use oteryn_protocol_oteryn::world_object::decode_use_result_with_gate;
use std::cell::{Cell, RefCell};
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

#[path = "tests/support.rs"]
mod support;
use support::*;

include!("../step_dispatch_fixture.rs");
include!("../tutorial_session_tests.rs");
include!("../newhaven_travel_transport_tests.rs");
include!("../spectate_session_ingress_tests.rs");

include!("tests/use_capabilities.rs");
include!("tests/pending_spell.rs");
include!("tests/bank_resume.rs");
include!("tests/liveness.rs");
include!("tests/steps_pacing.rs");
include!("tests/admission.rs");
include!("tests/overlay.rs");
include!("tests/use_commit.rs");
include!("tests/spell_vitals.rs");
include!("tests/spell_resume.rs");
include!("tests/achievements_highscores.rs");
include!("tests/notices.rs");
