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
