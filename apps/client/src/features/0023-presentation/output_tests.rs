use super::*;

#[test]
fn live_rodio_sinks_keep_ui_attenuation_when_accepted_gains_change() {
    // Rodio's upstream in-memory mixer exercises live sinks without opening a hardware device.
    let (mixer, _source) = rodio::mixer::mixer(1, 48_000);
    let mut voices = Vec::new();
    for scale in [0.15, 1.0] {
        let sink = Sink::connect_new(&mixer);
        sink.append(rodio::source::SineWave::new(440.0).take_duration(Duration::from_millis(70)));
        voices.push(Voice {
            source: PresentationSource::Global,
            channel: SoundChannel::Ui,
            class: None,
            sink,
            scale,
        });
    }
    let mut settings = AudioSettings {
        master_volume: 40,
        ..Default::default()
    };
    update_voices(&mut voices, settings);
    assert_eq!(voices.len(), 2);
    assert!((voices[0].sink.volume() - 0.06).abs() < f32::EPSILON);
    assert!((voices[1].sink.volume() - 0.4).abs() < f32::EPSILON);
    settings.muted = true;
    update_voices(&mut voices, settings);
    assert!(voices.iter().all(|voice| voice.sink.volume() == 0.0));
    for voice in voices {
        voice.sink.stop();
    }
}

#[test]
fn device_names_stay_valid_after_the_first_asking_thread_exits() {
    // The Sound page lists devices from whichever thread builds it; a thread that has exited must
    // not take the shared host enumerator with it (an access violation on Windows before).
    let first = std::thread::spawn(audio_device_names).join();
    assert!(first.is_ok());
    let second = std::thread::spawn(audio_device_names).join();
    assert!(second.is_ok());
    let current = audio_device_names();
    assert_eq!(first.ok(), Some(current.clone()));
    assert_eq!(second.ok(), Some(current));
}

#[test]
fn a_failed_audio_host_pin_is_retried_and_a_successful_one_is_kept() {
    let pinned = std::sync::Mutex::new(false);
    let mut attempts = 0;
    pin_once(&pinned, || {
        attempts += 1;
        false
    });
    pin_once(&pinned, || {
        attempts += 1;
        true
    });
    pin_once(&pinned, || {
        attempts += 1;
        true
    });
    assert_eq!(
        attempts, 2,
        "a failed pin was not retried, or a kept pin ran again"
    );
}

#[test]
fn w24_ambient_capture_reaches_native_mixer() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{Pushed, feature_0023_presentation::protocol::presentation};
    use oteryn_session::{AppliedDelta, PresentationEvent, PresentationEventKind};
    use std::fs;

    // One constant sample per asset makes looping and replacement observable in the real mixer.
    let wave = |sample: i16| {
        let mut bytes = b"RIFF\x26\0\0\0WAVEfmt \x10\0\0\0\x01\0\x01\0\x80\xbb\0\0\0\x77\x01\0\x02\0\x10\0data\x02\0\0\0".to_vec();
        bytes.extend_from_slice(&sample.to_le_bytes());
        bytes
    };
    let fixture = tempfile::tempdir()?;
    let files = [
        ("sounds.dat", b"test metadata".to_vec()),
        ("day.wav", wave(8192)),
        ("night.wav", wave(-8192)),
    ];
    let entries: Vec<_> = files.iter().map(|(name, bytes)| {
        serde_json::json!({"name": name, "sha256": oteryn_client_assets::text_sha256(bytes)})
    }).collect();
    for (name, bytes) in &files {
        fs::write(fixture.path().join(name), bytes)?;
    }
    let manifest = serde_json::to_vec(&serde_json::json!({
        "schema": "OTERYN_CLIENT_ASSET_MANIFEST/v1", "sound_data": "sounds.dat",
        "sound_cues": {"25": ["day.wav"]},
        "ambient_cues": {"25": ["day.wav"], "28": ["night.wav"]}, "files": entries
    }))?;
    let manifest_path = fixture.path().join("manifest.json");
    fs::write(&manifest_path, &manifest)?;
    let catalog = SoundCatalog::open(
        fixture.path(),
        &manifest_path,
        &oteryn_client_assets::text_sha256(&manifest),
    )?;
    let mut cues = Vec::new();
    if let Some(root) = std::env::var_os("E2E_ALL_W24_PLAYBACK_DIR") {
        use oteryn_protocol_oteryn::presentation::{
            STATE_DOMAIN_AMBIENT_SOUND, decode_presentation_batch,
        };
        use oteryn_protocol_oteryn::{MessageType, decode_framed_envelope, decode_state_delta};
        let root = Path::new(&root).join("full");
        for name in ["w24-exit.bin", "w24-entry.bin", "w24-relog.bin"] {
            let bytes = fs::read(root.join(name))?;
            let mut frames = bytes.as_slice();
            while !frames.is_empty() {
                let length =
                    u32::from_be_bytes(frames.get(..4).ok_or("missing frame length")?.try_into()?)
                        as usize;
                let (frame, remaining) = frames
                    .split_at_checked(length + 4)
                    .ok_or("truncated frame")?;
                frames = remaining;
                let envelope = decode_framed_envelope(frame)?;
                if envelope.message_type() != MessageType::StateDelta {
                    continue;
                }
                let delta = decode_state_delta(envelope.payload())?;
                if delta.domain_id == STATE_DOMAIN_AMBIENT_SOUND {
                    cues.push(AppliedDelta {
                        server_sequence: envelope.server_sequence(),
                        base_revision: delta.base_revision,
                        new_revision: delta.new_revision,
                        value: decode_presentation_batch(delta.payload)
                            .map_err(|error| format!("{error:?}"))?,
                    });
                }
            }
        }
        assert_eq!(
            cues.len(),
            7,
            "full replay must exercise all captured ambience changes"
        );
    } else {
        for id in [25, 28, 0] {
            cues.push(AppliedDelta {
                server_sequence: 1,
                base_revision: 0,
                new_revision: 1,
                value: vec![PresentationEvent {
                    decision_ordinal: 1,
                    emission_ordinal: 1,
                    kind: PresentationEventKind::AmbientSound { cue_id: id },
                }],
            });
        }
    }
    let (mixer, mut samples) = rodio::mixer::mixer(1, 48_000);
    let mut output = AudioOutput {
        mixer,
        _stream: None,
        catalog,
        voices: Vec::new(),
        ambient: None,
        error: None,
    };
    for delta in cues {
        let [event] = delta.value.as_slice() else {
            return Err("expected one ambient playback event".into());
        };
        let PresentationEventKind::AmbientSound { cue_id: id } = event.kind else {
            return Err("expected ambient playback event".into());
        };
        let mut pushed = Pushed::default();
        presentation(&mut pushed, delta);
        output.update(AudioSettings::default(), pushed.sounds);
        assert_eq!(output.status(), None);
        assert_eq!(
            output.ambient.as_ref().map(|(id, _)| *id),
            (id != 0).then_some(id)
        );
        // Rodio applies Sink::stop on its 5 ms periodic-access boundary.
        for _ in 0..240 {
            let _ = samples.next();
        }
        for _ in 0..16 {
            let sample = samples.next().unwrap_or(0.0);
            match id {
                25 => assert!(sample > 0.0, "day loop did not play"),
                28 => assert!(sample < 0.0, "night loop did not play"),
                0 => assert_eq!(sample, 0.0, "stop left audible samples"),
                other => return Err(format!("unexpected Thais cue {other}").into()),
            }
        }
    }
    Ok(())
}
