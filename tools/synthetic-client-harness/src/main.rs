use oteryn_client_domain::{
    ClientEntityRef, ClientWorldProjection, EntityProjection, Position, ProjectionEvent,
};
use oteryn_client_simulation::ClientSimulation;
use oteryn_foundation::{ClientSessionEpoch, ProcessGeneration};
use oteryn_input_actions::{
    ActionId, Binding, BindingMap, ButtonState, ContextDefinition, ContextId, ContextKind,
    InputAtom, InputChord, InputRouter, KeyCode, Modifiers, NormalizedInputEvent, RepeatPolicy,
};
use oteryn_renderer::{SurfaceDecision, SurfaceEvent, SurfaceSize, SurfaceState};
use oteryn_synthetic_assets::SyntheticImage;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut projection = ClientWorldProjection::empty(ClientSessionEpoch::new(1));
    projection.entities.insert(
        ClientEntityRef::new(1),
        EntityProjection {
            entity_ref: ClientEntityRef::new(1),
            position: Position {
                x: 100,
                y: 200,
                floor: 7,
            },
            display_name: "Synthetic Player".to_owned(),
        },
    );
    let mut simulation = ClientSimulation::new(projection);
    simulation.apply(ProjectionEvent::Upsert(EntityProjection {
        entity_ref: ClientEntityRef::new(2),
        position: Position {
            x: 101,
            y: 200,
            floor: 7,
        },
        display_name: "Synthetic Rat".to_owned(),
    }))?;

    let image = SyntheticImage::from_manifest_and_rgba(
        br#"{"schema":"synthetic-v1","asset_id":"checker","width":2,"height":2}"#,
        vec![
            0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255,
        ],
    )?;

    let gameplay = ContextId::new("gameplay".to_owned())?;
    let chord = InputChord::new(Modifiers::NONE, vec![InputAtom::Key(KeyCode::KEY_D)])?;
    let map = BindingMap::new(
        vec![ContextDefinition::new(
            gameplay.clone(),
            ContextKind::Gameplay,
            1,
        )],
        vec![Binding::new(
            gameplay.clone(),
            chord,
            ActionId::new("move.east".to_owned())?,
            RepeatPolicy::Ignore,
        )],
        &[],
    )?;
    let mut router = InputRouter::new(map);
    router.set_context_active(&gameplay, true)?;
    let actions = router.process(&NormalizedInputEvent::Key {
        code: KeyCode::KEY_D,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: false,
    });

    let generation = ProcessGeneration::new(1);
    let mut renderer = SurfaceState::new(generation);
    let decision = renderer.apply(SurfaceEvent::Resize {
        generation,
        width: 960,
        height: 540,
    })?;
    if decision != SurfaceDecision::Configure(SurfaceSize::new(960, 540)) {
        return Err("unexpected synthetic renderer decision".into());
    }

    println!(
        "synthetic-ok revision={} entities={} asset={} actions={}",
        simulation.snapshot().revision,
        simulation.snapshot().entities.len(),
        image.asset_id,
        actions.len()
    );
    Ok(())
}
