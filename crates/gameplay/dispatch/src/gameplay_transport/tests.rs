use super::connection::{ConnectionEnd, ConnectionIdentifiers};
use super::*;
use crate::foundation::{CharacterId, MessageType, decode_wire_envelope};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Notify;
use tokio_rustls::TlsConnector;

const CHARACTER: [u8; 16] = [
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x71, 0x11, 0x91, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
];

// Textual includes keep the original gameplay_transport::tests paths and shared fixtures.
include!("tests/admission_results.rs");
include!("tests/objects.rs");
include!("tests/listener.rs");
include!("tests/session_authority.rs");
include!("tests/lifecycle.rs");
include!("tests/death.rs");
