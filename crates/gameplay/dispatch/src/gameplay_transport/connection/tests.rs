// Included topic files share this module so test paths and private access stay stable.
#[path = "tests/buffered_step_recovery.rs"]
mod buffered_step_recovery;

#[path = "tests/capability_tests.rs"]
mod capability_tests;

#[path = "tests/hotkey_coalescing.rs"]
mod hotkey_coalescing;

#[path = "tests/idle_kick.rs"]
mod idle_kick;

#[path = "tests/integrity.rs"]
mod integrity;

mod bed_sleep_dispatch_tests {
    include!("../bed_sleep_dispatch_tests.rs");
}

mod item_transform_dispatch_tests {
    include!("../item_transform_dispatch_tests.rs");
}

#[path = "tests/write_text_depot_view.rs"]
mod write_text_depot_view;

use super::*;
use crate::gameplay_transport::world_spatial::encode_step_result;
use oteryn_protocol_oteryn::world_object::decode_use_result_with_gate;
use std::cell::{Cell, RefCell};
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

#[path = "tests/support.rs"]
mod support;

#[path = "tests/spell_catalog.rs"]
mod spell_catalog;
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

#[tokio::test]
async fn malformed_frame_yields_protocol_error_then_close() -> Result<(), Box<dyn Error>> {
    let (mut client, mut server) = tokio::io::duplex(256);
    let admitted = positioned()?;
    let task = tokio::spawn(async move { hold_admitted(&mut server, admitted).await });

    // A zero body length is forbidden by FND-02 framing.
    client.write_all(&0_u32.to_be_bytes()).await?;
    client.shutdown().await?;
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let frames = split_frames(&output)?;
    assert_eq!(frames.len(), 1);
    let envelope = decode_wire_envelope(&frames[0])?;
    assert_eq!(envelope.message_type(), MessageType::ProtocolError);
    assert_eq!(
        oteryn_protocol_oteryn::decode_protocol_error(envelope.payload())?.error_code,
        FoundationProtocolError::MalformedFrame.code()
    );
    assert!(matches!(
        task.await?,
        ConnectionEnd::AdmittedThenClosed(_, FoundationProtocolError::MalformedFrame)
    ));
    Ok(())
}
