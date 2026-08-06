use crate::{InputAtom, InputError, Modifiers};
use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

/// Maximum UTF-8 byte length of one semantic identifier.
pub const MAX_IDENTIFIER_BYTES: usize = 64;
/// Maximum number of non-modifier inputs in one chord.
pub const MAX_CHORD_INPUTS: usize = 4;

fn validate_identifier(value: &str) -> Result<(), InputError> {
    if value.is_empty() {
        return Err(InputError::EmptyIdentifier);
    }
    if value.len() > MAX_IDENTIFIER_BYTES {
        return Err(InputError::IdentifierTooLong {
            max: MAX_IDENTIFIER_BYTES,
            actual: value.len(),
        });
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':' | b'/')
    }) {
        return Err(InputError::InvalidIdentifier);
    }
    Ok(())
}

/// Framework-neutral semantic action identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId(String);

impl ActionId {
    /// Validate a semantic action identifier.
    ///
    /// # Errors
    ///
    /// Returns a stable identifier validation error.
    pub fn new(value: String) -> Result<Self, InputError> {
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    /// Borrow the stable identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ActionId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Framework-neutral input context identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContextId(String);

impl ContextId {
    /// Validate an input context identifier.
    ///
    /// # Errors
    ///
    /// Returns a stable identifier validation error.
    pub fn new(value: String) -> Result<Self, InputError> {
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    /// Borrow the stable identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ContextId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Explicit high-level context separation used during binding resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContextKind {
    /// Cross-mode actions that remain eligible in every active mode.
    Global,
    /// Normal gameplay input.
    Gameplay,
    /// Text-entry input that suppresses gameplay bindings.
    Text,
    /// Modal input that suppresses text and gameplay bindings.
    Modal,
}

impl ContextKind {
    pub(crate) const fn precedence(self) -> u8 {
        match self {
            Self::Global => 0,
            Self::Gameplay => 1,
            Self::Text => 2,
            Self::Modal => 3,
        }
    }
}

/// One declared semantic input context.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContextDefinition {
    id: ContextId,
    kind: ContextKind,
    priority: i16,
}

impl ContextDefinition {
    /// Construct a context definition.
    #[must_use]
    pub const fn new(id: ContextId, kind: ContextKind, priority: i16) -> Self {
        Self { id, kind, priority }
    }

    /// Borrow the context identifier.
    #[must_use]
    pub const fn id(&self) -> &ContextId {
        &self.id
    }

    /// Return the explicit context kind.
    #[must_use]
    pub const fn kind(&self) -> ContextKind {
        self.kind
    }

    /// Return the caller-defined precedence value.
    #[must_use]
    pub const fn priority(&self) -> i16 {
        self.priority
    }
}

/// Canonically sorted bounded physical chord.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputChord {
    modifiers: Modifiers,
    inputs: Box<[InputAtom]>,
}

impl InputChord {
    /// Normalize and validate one chord.
    ///
    /// # Errors
    ///
    /// Returns a stable empty, length, duplicate or unreachable-wheel error.
    pub fn new(modifiers: Modifiers, mut inputs: Vec<InputAtom>) -> Result<Self, InputError> {
        if inputs.is_empty() {
            return Err(InputError::EmptyChord);
        }
        if inputs.len() > MAX_CHORD_INPUTS {
            return Err(InputError::ChordTooLong {
                max: MAX_CHORD_INPUTS,
                actual: inputs.len(),
            });
        }
        inputs.sort_unstable();
        if inputs.windows(2).any(|window| window[0] == window[1]) {
            return Err(InputError::DuplicateChordInput);
        }
        if inputs.len() != 1 && inputs.iter().any(|input| input.is_wheel()) {
            return Err(InputError::InvalidWheelChord);
        }
        Ok(Self {
            modifiers,
            inputs: inputs.into_boxed_slice(),
        })
    }

    /// Return normalized modifiers.
    #[must_use]
    pub const fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Borrow canonically sorted non-modifier inputs.
    #[must_use]
    pub const fn inputs(&self) -> &[InputAtom] {
        &self.inputs
    }

    /// Return whether the chord contains one input.
    #[must_use]
    pub fn contains(&self, atom: InputAtom) -> bool {
        self.inputs.binary_search(&atom).is_ok()
    }

    pub(crate) fn is_subset_of(&self, other: &Self) -> bool {
        self.modifiers == other.modifiers
            && self
                .inputs
                .iter()
                .all(|input| other.inputs.binary_search(input).is_ok())
    }
}

/// Repeat behavior for a held semantic action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RepeatPolicy {
    /// Ignore platform repeat notifications.
    Ignore,
    /// Emit deterministic repeated action records.
    Allow,
}

/// One context-scoped semantic binding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Binding {
    context: ContextId,
    chord: InputChord,
    action: ActionId,
    repeat: RepeatPolicy,
}

impl Binding {
    /// Construct a binding definition.
    #[must_use]
    pub const fn new(
        context: ContextId,
        chord: InputChord,
        action: ActionId,
        repeat: RepeatPolicy,
    ) -> Self {
        Self {
            context,
            chord,
            action,
            repeat,
        }
    }

    /// Borrow the context identifier.
    #[must_use]
    pub const fn context(&self) -> &ContextId {
        &self.context
    }

    /// Borrow the normalized physical chord.
    #[must_use]
    pub const fn chord(&self) -> &InputChord {
        &self.chord
    }

    /// Borrow the semantic action identifier.
    #[must_use]
    pub const fn action(&self) -> &ActionId {
        &self.action
    }

    /// Return repeat behavior.
    #[must_use]
    pub const fn repeat(&self) -> RepeatPolicy {
        self.repeat
    }
}

/// Validated immutable context and binding catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingMap {
    contexts: Box<[ContextDefinition]>,
    bindings: Box<[Binding]>,
}

impl BindingMap {
    /// Validate contexts, bindings, conflicts and caller-reserved chords.
    ///
    /// # Errors
    ///
    /// Returns an explicit duplicate, unknown-context, conflict or reserved
    /// result. No binding silently overrides another.
    pub fn new(
        mut contexts: Vec<ContextDefinition>,
        mut bindings: Vec<Binding>,
        reserved: &[InputChord],
    ) -> Result<Self, InputError> {
        contexts.sort_by(|left, right| left.id.cmp(&right.id));
        if contexts
            .windows(2)
            .any(|window| window[0].id == window[1].id)
        {
            return Err(InputError::DuplicateContext);
        }

        let reserved: BTreeSet<&InputChord> = reserved.iter().collect();
        let mut occupied = BTreeSet::new();
        for binding in &bindings {
            if contexts
                .binary_search_by(|context| context.id.cmp(&binding.context))
                .is_err()
            {
                return Err(InputError::UnknownBindingContext);
            }
            if reserved.contains(&binding.chord) {
                return Err(InputError::ReservedBinding);
            }
            if !occupied.insert((binding.context.clone(), binding.chord.clone())) {
                return Err(InputError::ConflictingBinding);
            }
        }

        bindings.sort_by(|left, right| {
            left.context
                .cmp(&right.context)
                .then_with(|| left.chord.cmp(&right.chord))
                .then_with(|| left.action.cmp(&right.action))
        });

        Ok(Self {
            contexts: contexts.into_boxed_slice(),
            bindings: bindings.into_boxed_slice(),
        })
    }

    /// Borrow all declared contexts in identifier order.
    #[must_use]
    pub const fn contexts(&self) -> &[ContextDefinition] {
        &self.contexts
    }

    /// Borrow all bindings in deterministic order.
    #[must_use]
    pub const fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// Find one declared context.
    #[must_use]
    pub fn context(&self, id: &ContextId) -> Option<&ContextDefinition> {
        self.contexts
            .binary_search_by(|context| context.id.cmp(id))
            .ok()
            .map(|index| &self.contexts[index])
    }

    pub(crate) fn resolve(
        &self,
        active: &BTreeSet<ContextId>,
        chord: &InputChord,
    ) -> Option<Binding> {
        let modal_active = self
            .contexts
            .iter()
            .any(|context| context.kind == ContextKind::Modal && active.contains(&context.id));
        let text_active = !modal_active
            && self
                .contexts
                .iter()
                .any(|context| context.kind == ContextKind::Text && active.contains(&context.id));

        self.bindings
            .iter()
            .filter(|binding| binding.chord == *chord && active.contains(&binding.context))
            .filter_map(|binding| {
                let context = self.context(&binding.context)?;
                let eligible = match context.kind {
                    ContextKind::Global => true,
                    ContextKind::Modal => modal_active,
                    ContextKind::Text => text_active,
                    ContextKind::Gameplay => !modal_active && !text_active,
                };
                eligible.then_some((context, binding))
            })
            .max_by(|(left_context, _), (right_context, _)| {
                left_context
                    .priority
                    .cmp(&right_context.priority)
                    .then_with(|| {
                        left_context
                            .kind
                            .precedence()
                            .cmp(&right_context.kind.precedence())
                    })
                    .then_with(|| right_context.id.cmp(&left_context.id))
            })
            .map(|(_, binding)| binding.clone())
    }

    pub(crate) fn is_context_eligible(&self, active: &BTreeSet<ContextId>, id: &ContextId) -> bool {
        let Some(context) = self.context(id) else {
            return false;
        };
        if !active.contains(id) {
            return false;
        }
        let modal_active = self.contexts.iter().any(|candidate| {
            candidate.kind == ContextKind::Modal && active.contains(&candidate.id)
        });
        if modal_active {
            return matches!(context.kind, ContextKind::Modal | ContextKind::Global);
        }
        let text_active = self
            .contexts
            .iter()
            .any(|candidate| candidate.kind == ContextKind::Text && active.contains(&candidate.id));
        if text_active {
            return matches!(context.kind, ContextKind::Text | ContextKind::Global);
        }
        matches!(context.kind, ContextKind::Gameplay | ContextKind::Global)
    }
}
