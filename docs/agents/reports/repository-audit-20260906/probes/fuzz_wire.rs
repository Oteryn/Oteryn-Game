//! Temporary audit fuzz target; production parser bytes are unmodified.
#![no_main]
use libfuzzer_sys::fuzz_target;
use oteryn_game_server::foundation::{decode_wire_envelope,decode_framed_envelope,Direction};
fuzz_target!(|data: &[u8]| {
    for decoded in [decode_wire_envelope(data),decode_framed_envelope(data)] {
        if let Ok(envelope)=decoded {
            for direction in [Direction::ClientToServer,Direction::ServerToClient] {
                for admitted in [false,true] { let _=envelope.validate(direction,admitted); }
            }
        }
    }
});
