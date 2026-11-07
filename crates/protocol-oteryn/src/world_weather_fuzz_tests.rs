#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_world_weather() {
    crate::decoder_fuzz_support::codec(
        "decode_world_weather",
        |rng| {
            if rng.bool() {
                WorldWeather::Unavailable
            } else {
                WorldWeather::Active {
                    binding_token: [0x31; 32],
                    absolute_slot: rng.u64(),
                    kind: (rng.below(u64::from(WEATHER_KIND_MAX)) + 1) as u8,
                    intensity: (rng.below(u64::from(WEATHER_INTENSITY_MAX)) + 1) as u8,
                }
            }
        },
        decode_world_weather,
        |v| encode_world_weather(*v).expect("weather"),
    );
}
