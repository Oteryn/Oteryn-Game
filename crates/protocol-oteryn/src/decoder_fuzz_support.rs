//! Shared test mechanics; each wire module owns its generators and seed values.
#![allow(clippy::expect_used)]

use oteryn_test_support::property::{Rng, for_each_case, fuzz_decoder, fuzz_round_trip};
use std::fmt::Debug;

/// Exercise every seed and truncation even when the randomized case budget is zero.
/// Huge advertised lengths have tiny backing buffers: success would indicate a bad bound.
pub(super) fn decoder<E>(
    name: &str,
    mut samples: Vec<Vec<u8>>,
    decode: impl Fn(&[u8]) -> Result<(), E>,
) {
    for sample in &samples {
        assert!(
            decode(sample).is_ok(),
            "{name}: invalid valid seed {sample:02x?}"
        );
        for end in 0..sample.len() {
            let _ = decode(&sample[..end]);
        }
    }
    let garbage = [
        vec![0xff; 11],
        vec![0x0a, 0x80], // truncated length varint
        vec![
            0x0a, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01,
        ],
        vec![0x0a, 0xff, 0xff, 0xff, 0xff, 0x0f], // u32::MAX length
    ];
    for bad in garbage {
        assert!(decode(&bad).is_err(), "{name}: accepted garbage {bad:02x?}");
        samples.push(bad);
    }
    samples.push(Vec::new());
    fuzz_decoder(name, &samples, |bytes| decode(bytes).is_ok());
}

pub(super) fn codec<T: PartialEq + Debug, E: Debug>(
    name: &str,
    generate: impl Fn(&mut Rng) -> T,
    decode: impl Fn(&[u8]) -> Result<T, E>,
    encode: impl Fn(&T) -> Vec<u8>,
) {
    let mut samples = vec![encode(&generate(&mut Rng::new(1)))];
    for_each_case(name, |rng, _| {
        let value = generate(rng);
        let bytes = encode(&value);
        assert_eq!(decode(&bytes).expect("generated encoding decodes"), value);
        if samples.len() < 16 && !samples.contains(&bytes) {
            samples.push(bytes);
        }
        Ok(())
    });
    fuzz_round_trip(name, &samples, &decode, encode);
    decoder(name, samples, |bytes| decode(bytes).map(|_| ()));
}

pub(super) fn uuid(rng: &mut Rng) -> [u8; 16] {
    let mut value = rng.u64().to_be_bytes().repeat(2);
    value[6] = (value[6] & 15) | 0x70;
    value[8] = (value[8] & 63) | 0x80;
    value.as_slice().try_into().expect("16-byte UUID")
}
