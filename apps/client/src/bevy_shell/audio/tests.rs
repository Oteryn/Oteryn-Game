use super::driver::{Output, Player};
use crate::play::{AudioSettings, Outbox, PlayLink, PlayView, SoundCue, UiSound, play_channel};
use oteryn_session::{
    AppliedDelta, PresentationEvent, PresentationEventKind, PresentationSource, SessionEvent,
};
use std::{cell::RefCell, rc::Rc};

type TestResult = Result<(), Box<dyn std::error::Error>>;
#[derive(Debug, Clone, PartialEq)]
enum Event {
    Open(String),
    Render(AudioSettings, Vec<SoundCue>, Vec<UiSound>),
    Stop,
}
struct Recording(Rc<RefCell<Vec<Event>>>);
impl Output for Recording {
    fn update(&mut self, settings: AudioSettings, cues: Vec<SoundCue>, ui: Vec<UiSound>) {
        self.0.borrow_mut().push(Event::Render(settings, cues, ui));
    }
    fn stop(&mut self) {
        self.0.borrow_mut().push(Event::Stop);
    }
}
struct Play {
    view: PlayView,
    link: PlayLink,
    outbox: Outbox,
    _commands: tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
}
impl Play {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (link, commands, outbox) = play_channel();
        link.use_shared_frontend();
        let mut view = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
        view.use_shared_frontend();
        Ok(Self {
            view,
            link,
            outbox,
            _commands: commands,
        })
    }
    fn events(&mut self, kinds: Vec<PresentationEventKind>) -> TestResult {
        self.outbox
            .push(vec![SessionEvent::Presentation(AppliedDelta {
                server_sequence: 1,
                base_revision: 0,
                new_revision: 1,
                value: kinds
                    .into_iter()
                    .map(|kind| PresentationEvent {
                        decision_ordinal: 1,
                        emission_ordinal: 1,
                        kind,
                    })
                    .collect(),
            })]);
        self.view.tick(&self.link).map_err(|_| "tick")?;
        Ok(())
    }
    fn chat(&mut self) -> TestResult {
        self.outbox.push(vec![SessionEvent::ChatLine(AppliedDelta {
            server_sequence: 2,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::ChatLine::Private {
                speaker_name: "Fixture".into(),
                text: "Confirmed message".into(),
            },
        })]);
        self.view.tick(&self.link).map_err(|_| "tick")?;
        Ok(())
    }
}
fn sound(id: u32) -> PresentationEventKind {
    PresentationEventKind::Sound {
        position: oteryn_session::ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
        cue_id: id,
        source: PresentationSource::Own,
        secondary_cue_id: 0,
    }
}
fn open(log: &Rc<RefCell<Vec<Event>>>, device: &str) -> Result<Recording, String> {
    log.borrow_mut().push(Event::Open(device.to_owned()));
    Ok(Recording(Rc::clone(log)))
}
#[test]
fn confirmed_queues_reach_output_once_with_accepted_settings_and_ui_filters() -> TestResult {
    let mut play = Play::new()?;
    let mut preferences = play.view.preferences().clone();
    preferences.audio.master_volume = 40;
    preferences.audio.ui_sounds[UiSound::Chat as usize] = false;
    play.view.set_preferences(preferences.clone())?;
    play.events(vec![sound(41)])?;
    play.chat()?;
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut player = Player::default();
    player.poll(Some((1, &mut play.view)), true, |device| open(&log, device));
    let expected = SoundCue {
        id: 41,
        source: PresentationSource::Own,
        ambient: false,
    };
    assert!(
        log.borrow()
            .iter()
            .any(|event| matches!(event,Event::Render(settings,cues,ui)
        if *settings == preferences.audio && cues == &[expected] && ui.is_empty()))
    );
    assert!(play.view.take_sound_cues().is_empty());
    assert!(play.view.take_ui_sounds().is_empty());
    assert_eq!(play.view.preferences().audio, preferences.audio);
    Ok(())
}

fn rendered(
    log: &Rc<RefCell<Vec<Event>>>,
) -> Result<(AudioSettings, Vec<SoundCue>, Vec<UiSound>), Box<dyn std::error::Error>> {
    log.borrow()
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::Render(settings, cues, ui) => Some((*settings, cues.clone(), ui.clone())),
            _ => None,
        })
        .ok_or_else(|| "rendered frame".into())
}

#[test]
fn focus_drops_transients_and_resumes_only_the_latest_confirmed_ambient() -> TestResult {
    let mut play = Play::new()?;
    play.events(vec![
        PresentationEventKind::AmbientSound { cue_id: 7 },
        sound(41),
    ])?;
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut player = Player::default();
    player.poll(Some((1, &mut play.view)), true, |device| open(&log, device));
    play.events(vec![
        sound(42),
        PresentationEventKind::AmbientSound { cue_id: 8 },
    ])?;
    play.chat()?;
    player.poll(Some((1, &mut play.view)), false, |device| {
        open(&log, device)
    });
    assert_eq!(
        log.borrow()
            .iter()
            .filter(|event| matches!(event, Event::Stop))
            .count(),
        1
    );
    assert!(play.view.take_sound_cues().is_empty());
    assert!(play.view.take_ui_sounds().is_empty());
    player.poll(Some((1, &mut play.view)), true, |device| open(&log, device));
    let (_, cues, ui) = rendered(&log)?;
    assert_eq!(
        cues,
        [SoundCue {
            id: 8,
            source: PresentationSource::Global,
            ambient: true
        }]
    );
    assert!(ui.is_empty());
    // An authoritative stop received in the background must not resurrect the loop.
    play.events(vec![PresentationEventKind::AmbientSound { cue_id: 0 }])?;
    player.poll(Some((1, &mut play.view)), false, |device| {
        open(&log, device)
    });
    player.poll(Some((1, &mut play.view)), true, |device| open(&log, device));
    assert!(rendered(&log)?.1.is_empty());
    Ok(())
}

#[test]
fn device_and_generation_changes_stop_old_stream_and_never_replay_old_session_cues() -> TestResult {
    let mut play = Play::new()?;
    play.events(vec![PresentationEventKind::AmbientSound { cue_id: 7 }])?;
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut player = Player::default();
    player.poll(Some((1, &mut play.view)), true, |device| open(&log, device));
    let mut preferences = play.view.preferences().clone();
    preferences.audio_device = "Selected output".into();
    play.view.set_preferences(preferences)?;
    player.poll(Some((1, &mut play.view)), true, |device| open(&log, device));
    assert!(
        log.borrow()
            .contains(&Event::Open("Selected output".into()))
    );
    assert_eq!(
        rendered(&log)?.1,
        [SoundCue {
            id: 7,
            source: PresentationSource::Global,
            ambient: true
        }]
    );
    let mut next = Play::new()?;
    next.events(vec![sound(51)])?;
    player.poll(Some((2, &mut next.view)), true, |device| open(&log, device));
    assert_eq!(
        rendered(&log)?.1,
        [SoundCue {
            id: 51,
            source: PresentationSource::Own,
            ambient: false
        }]
    );
    player.poll(None, true, |device| open(&log, device));
    assert_eq!(
        log.borrow()
            .iter()
            .filter(|event| matches!(event, Event::Stop))
            .count(),
        3
    );
    drop(player);
    assert_eq!(
        log.borrow()
            .iter()
            .filter(|event| matches!(event, Event::Stop))
            .count(),
        3
    );
    Ok(())
}

#[test]
fn unavailable_output_drains_queues_without_retries_until_device_or_session_changes() -> TestResult
{
    let mut play = Play::new()?;
    play.events(vec![sound(41)])?;
    let mut player: Player<Recording> = Player::default();
    let mut attempts = 0;
    player.poll(Some((1, &mut play.view)), true, |_| {
        attempts += 1;
        Err("Device unavailable".into())
    });
    assert_eq!(player.status(), Some("Device unavailable"));
    assert!(play.view.take_sound_cues().is_empty());
    player.poll(Some((1, &mut play.view)), true, |_| {
        attempts += 1;
        Err("Device unavailable".into())
    });
    assert_eq!(attempts, 1);
    player.poll(Some((2, &mut play.view)), true, |_| {
        attempts += 1;
        Err("Device unavailable".into())
    });
    assert_eq!(attempts, 2);
    Ok(())
}
