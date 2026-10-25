use super::*;

fn failure_message(action: impl FnOnce()) -> String {
    match catch_unwind(AssertUnwindSafe(action)) {
        Ok(()) => String::new(),
        Err(payload) => panic_message(payload.as_ref()).to_owned(),
    }
}

#[test]
fn equal_seeds_produce_equal_streams() {
    let mut left = Rng::new(123);
    let mut right = Rng::new(123);
    for _ in 0..256 {
        assert_eq!(left.u64(), right.u64());
        assert_eq!(left.below(17), right.below(17));
        assert_eq!(left.range_i64(-42, 70), right.range_i64(-42, 70));
        assert_eq!(left.bool(), right.bool());
        assert_eq!(left.bytes(13), right.bytes(13));
        assert_eq!(left.pick(&[1, 2, 3]), right.pick(&[1, 2, 3]));
    }
    let mut known = Rng::new(0);
    assert_eq!(known.u64(), 0xe220_a839_7b1d_cdaf);
    assert_eq!(known.u64(), 0x6e78_9e6a_a1b9_65f4);
}

#[test]
fn bounded_sampling_handles_signed_extremes_and_invalid_bounds() {
    let mut rng = Rng::new(0);
    for bound in [1, 2, 17, (1 << 63) + 1, u64::MAX] {
        for _ in 0..256 {
            assert!(rng.below(bound) < bound);
        }
    }
    for (lo, hi) in [
        (i64::MIN, i64::MIN),
        (i64::MAX, i64::MAX),
        (i64::MIN, i64::MAX),
        (i64::MIN, 0),
        (-1, i64::MAX),
        (-7, -2),
        (-3, 3),
    ] {
        for _ in 0..256 {
            assert!((lo..=hi).contains(&rng.range_i64(lo, hi)));
        }
    }
    let mut seen = [false; 3];
    for _ in 0..256 {
        seen[(rng.range_i64(-1, 1) + 1) as usize] = true;
    }
    assert_eq!(seen, [true; 3]);
    assert!(
        failure_message(|| {
            rng.below(0);
        })
        .contains("nonzero")
    );
    assert!(
        failure_message(|| {
            rng.range_i64(2, 1);
        })
        .contains("lo <= hi")
    );
    assert!(
        failure_message(|| {
            rng.pick::<u8>(&[]);
        })
        .contains("nonzero")
    );
    assert!(rng.bytes(0).is_empty());
}

#[test]
fn environment_configuration_defaults_caps_and_accepts_zero() {
    let defaults = Cases::parse(None, None);
    assert_eq!((defaults.seed, defaults.count), (DEFAULT_SEED, 256));
    for invalid in ["", "invalid", "-1", "18446744073709551616"] {
        let cases = Cases::parse(Some(invalid), Some(invalid));
        assert_eq!((cases.seed, cases.count), (DEFAULT_SEED, 256));
    }
    let cases = Cases::parse(Some("18446744073709551615"), Some("4294967295"));
    assert_eq!((cases.seed, cases.count), (u64::MAX, MAX_CASES));
    assert_eq!(Cases::parse(Some("0"), Some("0")).count, 0);
}

#[test]
#[allow(clippy::panic)] // This test proves that a zero budget never invokes the closure.
fn case_streams_depend_on_name_seed_and_index_and_replay_exactly() {
    let cases = Cases::new(91, 8);
    let mut visited = Vec::new();
    cases.for_each_case("decoder", |rng, case| {
        let bytes = rng.bytes(20);
        assert_eq!(bytes, Rng::new(cases.case_seed("decoder", case)).bytes(20));
        visited.push(bytes);
        Ok(())
    });
    assert_eq!(visited.len(), 8);
    for case in 0..8 {
        let seed = cases.case_seed("decoder", case);
        assert_ne!(seed, cases.case_seed("other decoder", case));
        assert_ne!(seed, Cases::new(92, 8).case_seed("decoder", case));
        for other in 0..case {
            assert_ne!(seed, cases.case_seed("decoder", other));
        }
    }
    let mut replay = Vec::new();
    Cases::new(91, 4).for_each_case("decoder", |rng, _| {
        replay.push(rng.bytes(20));
        Ok(())
    });
    assert_eq!(replay, visited[..4]);
    Cases::new(91, 0).for_each_case("empty budget", |_, _| {
        std::panic::panic_any("zero cases must not run");
    });
}

#[test]
fn failure_reports_case_seed_and_replay_environment() {
    let cases = Cases::new(42, 12);
    let message = failure_message(|| {
        cases.for_each_case("broken codec", |_, case| {
            if case == 3 {
                Err("wrong value".to_owned())
            } else {
                Ok(())
            }
        });
    });
    assert!(message.contains("broken codec"));
    assert!(message.contains("case 3"));
    assert!(message.contains(&format!("case seed {}", cases.case_seed("broken codec", 3))));
    assert!(message.contains("wrong value"));
    assert!(message.contains("OTERYN_PROPERTY_SEED=42 OTERYN_PROPERTY_CASES=4"));
}

#[test]
#[allow(clippy::panic)] // Deliberate panics exercise the property runner's diagnostics.
fn panicking_decoder_is_reported_with_case_context() {
    let cases = Cases::new(7, 1);
    let decode = |_: &[u8]| std::panic::panic_any("decoder exploded");
    let message = failure_message(|| {
        cases.for_each_case("panicking decoder", |rng, _| {
            let input = fuzz_input(rng, &[]);
            decode(&input);
            Ok(())
        });
    });
    assert!(message.contains("panicking decoder"));
    assert!(message.contains("case 0"));
    assert!(message.contains(&format!(
        "case seed {}",
        cases.case_seed("panicking decoder", 0)
    )));
    assert!(message.contains("decoder exploded"));
    assert!(message.contains("OTERYN_PROPERTY_SEED=7 OTERYN_PROPERTY_CASES=1"));
    let string_payload = failure_message(|| {
        cases.for_each_case("string payload", |_, _| {
            std::panic::panic_any(String::from("owned panic"));
        });
    });
    assert!(string_payload.contains("owned panic"));
    let opaque_payload = failure_message(|| {
        cases.for_each_case("opaque payload", |_, _| std::panic::panic_any(123));
    });
    assert!(opaque_payload.contains("non-string panic payload"));
}

#[test]
fn mutations_are_deterministic_and_bounded_including_empty_inputs() {
    for len in [0, 1, 2, 4, 8, 64, 512, 1024] {
        let input = vec![0x55; len];
        let mut left = Rng::new(1);
        let mut right = Rng::new(1);
        let mut shrank = false;
        let mut grew = false;
        let mut changed = false;
        for _ in 0..2048 {
            let output = mutate(&mut left, &input);
            assert_eq!(output, mutate(&mut right, &input));
            assert!(output.len() <= input.len() + 64);
            shrank |= output.len() < input.len();
            grew |= output.len() > input.len();
            changed |= output != input;
        }
        assert!(grew && changed);
        assert!(len == 0 || shrank);
        assert_eq!(input, vec![0x55; len]);
    }
}

#[test]
fn fuzz_input_exercises_random_and_mutated_samples() {
    let samples = vec![vec![0x55; 1024]];
    let mut random = false;
    let mut mutated = false;
    Cases::new(55, 256).for_each_case("input branches", |rng, _| {
        let input = fuzz_input(rng, &samples);
        assert!(input.len() <= 1088);
        random |= input.len() <= 512;
        mutated |= input.len() > 512;
        assert!(fuzz_input(rng, &[]).len() <= 512);
        Ok(())
    });
    assert!(random && mutated);
}

#[test]
#[allow(clippy::panic)] // Rejected inputs must never reach the deliberately panicking encoder.
fn decoder_and_round_trip_entry_points_accept_valid_codecs() {
    fuzz_decoder("arbitrary bytes", &[], |bytes| bytes.len());
    fuzz_decoder("mutated bytes", &[vec![0, 1, 2]], |bytes| bytes.to_vec());
    fuzz_round_trip(
        "identity codec",
        &[vec![], vec![0xff; 32]],
        |bytes| Ok::<_, ()>(bytes.to_vec()),
        Vec::clone,
    );
    fuzz_round_trip(
        "rejecting codec",
        &[],
        |_| Err::<u8, _>(()),
        |_| std::panic::panic_any("rejected values must not be encoded"),
    );
}

#[test]
fn round_trip_failures_are_reported_through_cases() {
    for (expected, decode) in [
        ("round trip changed 1 to 2", Ok(2_u8)),
        ("decoder rejected re-encoded value 1", Err(())),
    ] {
        let message = failure_message(|| {
            Cases::new(17, 1).for_each_case("broken round trip", |_, _| {
                check_round_trip(1_u8, |_| decode, |value| vec![*value])
            });
        });
        assert!(message.contains(expected));
        assert!(message.contains("broken round trip"));
        assert!(message.contains("OTERYN_PROPERTY_SEED=17 OTERYN_PROPERTY_CASES=1"));
    }
}

#[test]
fn mutations_cover_bit_byte_length_slice_and_integer_boundaries() {
    let input = vec![0x55; 16];
    let mut rng = Rng::new(0);
    let outputs: Vec<_> = (0..8192).map(|_| mutate(&mut rng, &input)).collect();
    assert!(outputs.iter().any(Vec::is_empty));
    assert!(outputs.iter().any(|output| output.len() == 80));
    assert!(outputs.iter().any(|output| output.len() == 15));
    assert!(outputs.iter().any(|output| output.len() == 17));
    assert!(outputs.iter().any(|output| output == &vec![0x55; 32]));
    for bit in 0..8 {
        assert!(outputs.iter().any(|output| {
            output.len() == 16
                && output.iter().filter(|&&byte| byte != 0x55).count() == 1
                && output.contains(&(0x55 ^ (1 << bit)))
        }));
    }
    for byte in [0x00, 0xff, 0x7f, 0x80] {
        assert!(outputs.iter().any(|output| {
            output.len() == 16
                && output.iter().filter(|&&value| value != 0x55).count() == 1
                && output.contains(&byte)
        }));
    }
    for width in [2, 4, 8] {
        for byte in [0x00, 0xff] {
            assert!(outputs.iter().any(|output| {
                output.len() == 16
                    && output.iter().filter(|&&value| value != 0x55).count() == width
                    && output
                        .windows(width)
                        .any(|window| window == vec![byte; width])
            }));
        }
    }
}
