//! Bounded deterministic mutation smoke, not a coverage-guided fuzzing campaign.
use oteryn_game_server::foundation::{decode_framed_envelope, decode_wire_envelope, Direction};
use oteryn_game_server::content::{compile, synthetic_vsl_fixture, CompileTarget, EvidenceLimits, StagedArtifact};

fn next(state: &mut u64) -> u8 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state as u8
}

#[test]
fn bounded_wire_malformed_inputs_do_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let seed = [8_u8, 14, 34, 0];
    let envelope = decode_wire_envelope(&seed)?;
    envelope.validate(Direction::ServerToClient, false)?;
    let mut state = 0x6f746572796e_u64;
    for case in 0..20_000_usize {
        let length = case % 513;
        let bytes: Vec<u8> = (0..length).map(|_| next(&mut state)).collect();
        let _ = decode_wire_envelope(&bytes);
        let _ = decode_framed_envelope(&bytes);
    }
    for value in 0..=255_u8 {
        for index in 0..seed.len() {
            let mut altered = seed;
            altered[index] = value;
            let _ = decode_wire_envelope(&altered);
        }
    }
    println!("AUDIT_WIRE_MUTATION_SMOKE: 20000 deterministic buffers through both decoders plus 1024 valid-seed byte substitutions; no panic; not exhaustive fuzzing");
    Ok(())
}

#[test]
fn compiled_content_rejects_all_single_byte_corruptions_and_truncations() -> Result<(), Box<dyn std::error::Error>> {
    let limits = EvidenceLimits::new("evidence:audit-only", 1_048_576, 16, 524_288, 4096, 4096, 256, 512, 4096, 4096, 8192)?;
    let fixture = synthetic_vsl_fixture(&limits)?;
    let compiled = compile(&fixture, &limits, CompileTarget::Evidence)?;
    let mut count = 0_usize;
    for bytes in [&compiled.server_artifact, &compiled.client_artifact] {
        StagedArtifact::stage(bytes, &limits)?;
        for length in 0..bytes.len() {
            assert!(StagedArtifact::stage(&bytes[..length], &limits).is_err());
            count += 1;
        }
        for index in 0..bytes.len() {
            let mut corrupt = bytes.clone();
            corrupt[index] ^= 1;
            assert!(StagedArtifact::stage(&corrupt, &limits).is_err());
            count += 1;
        }
    }
    println!("AUDIT_CONTENT_MUTATION_SMOKE: {count} deterministic truncation/bit-flip rejects; integrity-path evidence, not full semantic fuzz coverage");
    Ok(())
}
