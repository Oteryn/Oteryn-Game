use crate::{
    ActionId, Binding, BindingMap, ButtonState, ContextId, ContextKind, InputAtom, InputChord,
    InputError, MAX_CHORD_INPUTS, Modifiers, NormalizedInputEvent, RepeatPolicy,
};
use std::collections::{BTreeMap, BTreeSet};

/// Semantic action lifecycle emitted by the deterministic router.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionPhase {
    /// A physical chord became active.
    Started,
    /// An allowed platform repeat was received while the chord remained held.
    Repeated,
    /// A physical chord was released normally.
    Ended,
    /// Focus, capture, device or context lifecycle invalidated the chord.
    Cancelled,
}

/// Context-qualified semantic action output suitable for later consumers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionEvent {
    action: ActionId,
    context: ContextId,
    phase: ActionPhase,
}

impl ActionEvent {
    fn new(action: ActionId, context: ContextId, phase: ActionPhase) -> Self {
        Self {
            action,
            context,
            phase,
        }
    }

    /// Borrow the framework-neutral semantic action identifier.
    #[must_use]
    pub const fn action(&self) -> &ActionId {
        &self.action
    }

    /// Borrow the context that won precedence.
    #[must_use]
    pub const fn context(&self) -> &ContextId {
        &self.context
    }

    /// Return the semantic lifecycle phase.
    #[must_use]
    pub const fn phase(&self) -> ActionPhase {
        self.phase
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveAction {
    action: ActionId,
    context: ContextId,
    repeat: RepeatPolicy,
}

impl ActiveAction {
    fn from_binding(binding: Binding) -> Self {
        Self {
            action: binding.action().clone(),
            context: binding.context().clone(),
            repeat: binding.repeat(),
        }
    }

    fn event(&self, phase: ActionPhase) -> ActionEvent {
        ActionEvent::new(self.action.clone(), self.context.clone(), phase)
    }
}

/// Deterministic held-state, context and semantic action router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputRouter {
    map: BindingMap,
    active_contexts: BTreeSet<ContextId>,
    held: BTreeSet<InputAtom>,
    modifiers: Modifiers,
    active_actions: BTreeMap<InputChord, ActiveAction>,
    focused: bool,
    captured: bool,
}

impl InputRouter {
    /// Construct a router with all global contexts active and other contexts
    /// inactive.
    #[must_use]
    pub fn new(map: BindingMap) -> Self {
        let active_contexts = map
            .contexts()
            .iter()
            .filter(|context| context.kind() == ContextKind::Global)
            .map(|context| context.id().clone())
            .collect();
        Self {
            map,
            active_contexts,
            held: BTreeSet::new(),
            modifiers: Modifiers::NONE,
            active_actions: BTreeMap::new(),
            focused: true,
            captured: false,
        }
    }

    /// Activate or deactivate one declared context.
    ///
    /// Deactivation, modal activation and text activation cancel active actions
    /// that are no longer eligible.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::ContextNotFound`] for an undeclared context.
    pub fn set_context_active(
        &mut self,
        id: &ContextId,
        active: bool,
    ) -> Result<Vec<ActionEvent>, InputError> {
        if self.map.context(id).is_none() {
            return Err(InputError::ContextNotFound);
        }
        if active {
            self.active_contexts.insert(id.clone());
        } else {
            self.active_contexts.remove(id);
        }
        Ok(self.cancel_ineligible())
    }

    /// Process one normalized event and return deterministic semantic outputs.
    pub fn process(&mut self, event: &NormalizedInputEvent) -> Vec<ActionEvent> {
        match event {
            NormalizedInputEvent::Key {
                code,
                state,
                modifiers,
                repeat,
            } => self.process_button(InputAtom::Key(*code), *state, *modifiers, *repeat),
            NormalizedInputEvent::MouseButton {
                button,
                state,
                modifiers,
            } => self.process_button(InputAtom::Mouse(*button), *state, *modifiers, false),
            NormalizedInputEvent::Wheel { delta, modifiers } => {
                self.modifiers = *modifiers;
                if !self.focused {
                    return Vec::new();
                }
                let mut events = Vec::new();
                for direction in delta.directions() {
                    events.extend(self.process_impulse(InputAtom::Wheel(direction), *modifiers));
                }
                events
            }
            NormalizedInputEvent::FocusChanged { focused } => {
                self.focused = *focused;
                if *focused {
                    Vec::new()
                } else {
                    let events = self.cancel_all();
                    self.held.clear();
                    self.modifiers = Modifiers::NONE;
                    events
                }
            }
            NormalizedInputEvent::CaptureChanged { captured } => {
                self.captured = *captured;
                if *captured {
                    Vec::new()
                } else {
                    let events = self.cancel_mouse_actions();
                    self.held.retain(|atom| !atom.is_mouse_button());
                    events
                }
            }
            NormalizedInputEvent::DeviceLost => {
                let events = self.cancel_all();
                self.held.clear();
                self.modifiers = Modifiers::NONE;
                self.captured = false;
                events
            }
            NormalizedInputEvent::PointerMoved { .. } | NormalizedInputEvent::TextCommitted(_) => {
                Vec::new()
            }
        }
    }

    fn process_button(
        &mut self,
        atom: InputAtom,
        state: ButtonState,
        modifiers: Modifiers,
        repeat: bool,
    ) -> Vec<ActionEvent> {
        self.modifiers = modifiers;
        if !self.focused {
            return Vec::new();
        }

        match state {
            ButtonState::Pressed if repeat => self.repeat_for_atom(atom),
            ButtonState::Pressed => {
                if !self.held.insert(atom) {
                    return Vec::new();
                }
                if self.held.len() > MAX_CHORD_INPUTS {
                    return Vec::new();
                }
                let Ok(chord) = InputChord::new(modifiers, self.held.iter().copied().collect())
                else {
                    return Vec::new();
                };
                let Some(binding) = self.map.resolve(&self.active_contexts, &chord) else {
                    return Vec::new();
                };

                let mut events = self.cancel_strict_subsets(&chord);
                if self.active_actions.contains_key(&chord) {
                    return events;
                }
                let active = ActiveAction::from_binding(binding);
                events.push(active.event(ActionPhase::Started));
                self.active_actions.insert(chord, active);
                events
            }
            ButtonState::Released => {
                let events = self.end_actions_containing(atom);
                self.held.remove(&atom);
                events
            }
        }
    }

    fn process_impulse(&self, atom: InputAtom, modifiers: Modifiers) -> Vec<ActionEvent> {
        let Ok(chord) = InputChord::new(modifiers, vec![atom]) else {
            return Vec::new();
        };
        let Some(binding) = self.map.resolve(&self.active_contexts, &chord) else {
            return Vec::new();
        };
        let active = ActiveAction::from_binding(binding);
        vec![
            active.event(ActionPhase::Started),
            active.event(ActionPhase::Ended),
        ]
    }

    fn repeat_for_atom(&self, atom: InputAtom) -> Vec<ActionEvent> {
        if !self.held.contains(&atom) {
            return Vec::new();
        }
        self.active_actions
            .iter()
            .filter(|(chord, active)| chord.contains(atom) && active.repeat == RepeatPolicy::Allow)
            .map(|(_, active)| active.event(ActionPhase::Repeated))
            .collect()
    }

    fn cancel_strict_subsets(&mut self, chord: &InputChord) -> Vec<ActionEvent> {
        let keys: Vec<InputChord> = self
            .active_actions
            .keys()
            .filter(|active| *active != chord && active.is_subset_of(chord))
            .cloned()
            .collect();
        self.remove_actions(keys, ActionPhase::Cancelled)
    }

    fn end_actions_containing(&mut self, atom: InputAtom) -> Vec<ActionEvent> {
        let keys: Vec<InputChord> = self
            .active_actions
            .keys()
            .filter(|chord| chord.contains(atom))
            .cloned()
            .collect();
        self.remove_actions(keys, ActionPhase::Ended)
    }

    fn cancel_mouse_actions(&mut self) -> Vec<ActionEvent> {
        let keys: Vec<InputChord> = self
            .active_actions
            .keys()
            .filter(|chord| chord.inputs().iter().any(|atom| atom.is_mouse_button()))
            .cloned()
            .collect();
        self.remove_actions(keys, ActionPhase::Cancelled)
    }

    fn cancel_ineligible(&mut self) -> Vec<ActionEvent> {
        let keys: Vec<InputChord> = self
            .active_actions
            .iter()
            .filter(|(_, active)| {
                !self
                    .map
                    .is_context_eligible(&self.active_contexts, &active.context)
            })
            .map(|(chord, _)| chord.clone())
            .collect();
        self.remove_actions(keys, ActionPhase::Cancelled)
    }

    fn cancel_all(&mut self) -> Vec<ActionEvent> {
        let keys: Vec<InputChord> = self.active_actions.keys().cloned().collect();
        self.remove_actions(keys, ActionPhase::Cancelled)
    }

    fn remove_actions(&mut self, keys: Vec<InputChord>, phase: ActionPhase) -> Vec<ActionEvent> {
        keys.into_iter()
            .filter_map(|key| self.active_actions.remove(&key))
            .map(|active| active.event(phase))
            .collect()
    }

    /// Borrow the immutable binding map.
    #[must_use]
    pub const fn binding_map(&self) -> &BindingMap {
        &self.map
    }

    /// Borrow active context identifiers in deterministic order.
    #[must_use]
    pub const fn active_contexts(&self) -> &BTreeSet<ContextId> {
        &self.active_contexts
    }

    /// Borrow held physical inputs in deterministic order.
    #[must_use]
    pub const fn held_inputs(&self) -> &BTreeSet<InputAtom> {
        &self.held
    }

    /// Return the latest canonical modifier snapshot.
    #[must_use]
    pub const fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Return whether the router currently accepts focused input.
    #[must_use]
    pub const fn focused(&self) -> bool {
        self.focused
    }

    /// Return whether pointer capture is active.
    #[must_use]
    pub const fn captured(&self) -> bool {
        self.captured
    }
}
