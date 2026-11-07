//! Deterministic property cases and byte-oriented decoder fuzzing.
//!
//! Set `OTERYN_PROPERTY_CASES` (default 256, maximum 1,000,000) and
//! `OTERYN_PROPERTY_SEED` (a decimal u64) to control a run. Invalid values use
//! the defaults. Failures report the individual case seed and a replay command.

mod mutation;
mod rng;

pub use mutation::mutate;
pub use rng::Rng;

use std::any::Any;
use std::fmt::Debug;
use std::panic::{AssertUnwindSafe, catch_unwind};

const DEFAULT_SEED: u64 = 0x4f54_4552_594e_0001;
const DEFAULT_CASES: u32 = 256;
const MAX_CASES: u32 = 1_000_000;

/// A bounded case budget and base seed, captured once for a property run.
#[derive(Debug, Clone, Copy)]
pub struct Cases {
    seed: u64,
    count: u32,
}

impl Cases {
    #[must_use]
    pub fn new(seed: u64, count: u32) -> Self {
        Self {
            seed,
            count: count.min(MAX_CASES),
        }
    }

    #[must_use]
    pub fn from_env() -> Self {
        Self::parse(
            std::env::var("OTERYN_PROPERTY_SEED").ok().as_deref(),
            std::env::var("OTERYN_PROPERTY_CASES").ok().as_deref(),
        )
    }

    fn parse(seed: Option<&str>, count: Option<&str>) -> Self {
        Self::new(
            seed.and_then(|value| value.parse().ok())
                .unwrap_or(DEFAULT_SEED),
            count
                .and_then(|value| value.parse().ok())
                .unwrap_or(DEFAULT_CASES),
        )
    }

    /// Runs zero-based cases, catching returned errors and unwinding panics.
    ///
    /// The name and index each contribute to a stable per-case seed. Replay
    /// uses the base seed and enough cases to reach the failing index; the
    /// reported case seed can also be passed directly to `Rng::new`.
    #[allow(clippy::panic)]
    pub fn for_each_case(
        self,
        name: &str,
        mut property: impl FnMut(&mut Rng, u32) -> Result<(), String>,
    ) {
        for case in 0..self.count {
            let seed = self.case_seed(name, case);
            let mut rng = Rng::new(seed);
            let failure = match catch_unwind(AssertUnwindSafe(|| property(&mut rng, case))) {
                Ok(Ok(())) => continue,
                Ok(Err(error)) => error,
                Err(payload) => format!("panic: {}", panic_message(payload.as_ref())),
            };
            panic!(
                "property {name:?} failed at case {case}; case seed {seed}: {failure}\n\
                 Replay the same test: OTERYN_PROPERTY_SEED={} OTERYN_PROPERTY_CASES={}\n\
                 Replay the case directly with Rng::new({seed})",
                self.seed,
                case + 1,
            );
        }
    }

    fn case_seed(self, name: &str, case: u32) -> u64 {
        // FNV-1a avoids std's unspecified, potentially randomized hashers.
        let name_hash = name.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
        });
        Rng::new((self.seed ^ name_hash).wrapping_add(u64::from(case))).u64()
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    if let Some(message) = payload.downcast_ref::<String>() {
        message
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        message
    } else {
        "non-string panic payload"
    }
}

/// Runs a property using the environment's case budget and seed.
pub fn for_each_case(name: &str, property: impl FnMut(&mut Rng, u32) -> Result<(), String>) {
    Cases::from_env().for_each_case(name, property);
}

/// Feeds random bytes or mutated samples to a decoder, rejecting any panic.
pub fn fuzz_decoder<R>(name: &str, samples: &[Vec<u8>], decode: impl Fn(&[u8]) -> R) {
    for_each_case(name, |rng, _| {
        let input = fuzz_input(rng, samples);
        let _ = decode(&input);
        Ok(())
    });
}

/// Checks that every accepted fuzz input survives encoding and decoding again.
pub fn fuzz_round_trip<T: PartialEq + Debug, E>(
    name: &str,
    samples: &[Vec<u8>],
    decode: impl Fn(&[u8]) -> Result<T, E>,
    encode: impl Fn(&T) -> Vec<u8>,
) {
    for_each_case(name, |rng, _| {
        let input = fuzz_input(rng, samples);
        if let Ok(value) = decode(&input) {
            check_round_trip(value, &decode, &encode)?;
        }
        Ok(())
    });
}

fn check_round_trip<T: PartialEq + Debug, E>(
    value: T,
    decode: impl Fn(&[u8]) -> Result<T, E>,
    encode: impl Fn(&T) -> Vec<u8>,
) -> Result<(), String> {
    let encoded = encode(&value);
    match decode(&encoded) {
        Ok(round_trip) if round_trip == value => Ok(()),
        Ok(round_trip) => Err(format!("round trip changed {value:?} to {round_trip:?}")),
        Err(_) => Err(format!("decoder rejected re-encoded value {value:?}")),
    }
}

fn fuzz_input(rng: &mut Rng, samples: &[Vec<u8>]) -> Vec<u8> {
    if samples.is_empty() || rng.bool() {
        let len = rng.below(513) as usize;
        rng.bytes(len)
    } else {
        let sample = rng.pick(samples);
        mutate(rng, sample)
    }
}

#[cfg(test)]
#[path = "property_tests.rs"]
mod tests;
