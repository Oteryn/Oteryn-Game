use super::fixture::{Harness, debug, id};

use super::TestResult;

use crate::foundation::{
    MessageType,
    decode_wire_envelope,
};

use crate::gameplay_transport::ComposedFreshAdmission;

use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS,
    serve_admitted,
};

use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};

use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseTarget, encode_item_use_intent,
};

use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};

pub(super) const INNER: &str = "oteryn:item.tibia.i2854";

#[path = "house_balloon/wire.rs"]
pub(super) mod wire;
