//! APP-3 / CP D1094 proposed wire layout. Capability 25 stays unoffered until slot A2.
//! Strict proto3 subset using the native parser; identities, sex and authority never come
//! from a change payload. See the APP-3 task record for fields and lifetime rules.
use std::collections::BTreeSet;

pub use crate::charm_wire::CyclopediaWireError as Error;
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint};
use crate::world_spatial_entities::EntityRef;

pub const CAPABILITY_APPEARANCE_V1: u32 = 25;
pub const CAPABILITY_APPEARANCE_V1_REQUIRES: [u32; 1] = [6];
pub const COMMAND_TYPE_APPEARANCE_CHANGE: u32 = 25;
pub const STATE_DOMAIN_APPEARANCE: u32 = 21;
pub const SNAPSHOT_TYPE_APPEARANCE_V1: u32 = 1;
pub const DELTA_TYPE_APPEARANCE_MAPPINGS_V1: u32 = 1;
pub const DELTA_TYPE_APPEARANCE_CATALOGUE_V1: u32 = 2;
/// Native production content key bound; provenance follows CHARACTER_APPEARANCE §4.2.
pub const MAX_KEY_BYTES: usize = 512;
pub const MAX_PROVENANCE_BYTES: usize = 128;
pub const MAX_SELECTION_BYTES: usize = 1_100;
pub const MAX_CHANGE_BYTES: usize = 4_096;
pub const MAX_RESULT_BYTES: usize = 16;
pub const MAX_PALETTE_COLOURS: usize = 256;
pub const MAX_CATALOGUE_OUTFITS: usize = 256;
pub const MAX_CATALOGUE_MOUNTS: usize = 256;
pub const MAX_CATALOGUE_BYTES: usize = 196_608;
pub const MAX_BINDINGS: usize = crate::world_spatial_entities::MAX_SNAPSHOT_ENTITIES;
pub const MAX_BINDING_BYTES: usize = 256;
pub const MAX_MAPPINGS_BYTES: usize = 131_072;
pub const MAX_SNAPSHOT_BYTES: usize = 262_144;
const _: () = assert!(MAX_CHANGE_BYTES <= crate::MAX_COMMAND_PAYLOAD_BYTES);
const _: () = assert!(MAX_RESULT_BYTES <= crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES);
const _: () = assert!(MAX_MAPPINGS_BYTES <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES);
const _: () = assert!(MAX_CATALOGUE_BYTES <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES);
const _: () = assert!(MAX_SNAPSHOT_BYTES <= crate::MAX_SNAPSHOT_CHUNK_BYTES);

type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context {
    /// Client artifact generation, exactly the generation in native VIS-2 payloads.
    pub content: [u8; 32],
    /// Exact cosmetics document identity, including palette and per-sex look definitions.
    pub cosmetics: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub outfit: String,
    pub colours: [u16; 4],
    pub addons: u8,
    pub mount: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selected {
    pub selection: Selection,
    pub revision: u64,
    pub provenance: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub context: Context,
    pub expected_character_revision: u64,
    pub before: Option<Selected>,
    pub selection: Selection,
    pub content_revision: String,
}
/// CommandId and connection generation stay in the existing CommandResult envelope.
/// Unknown storage/COMMIT outcomes do not have a terminal result payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeResult {
    Committed { revision: u64 },
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutfitOption {
    pub key: String,
    pub look_type: u32,
    pub allowed_addons: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalogue {
    pub context: Context,
    pub character_revision: u64,
    pub selected: Option<Selected>,
    pub palette: Vec<[u8; 3]>,
    /// Only this character's currently authorized choices, strictly ascending by key.
    pub outfits: Vec<OutfitOption>,
    pub mounts: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Look {
    Outfit {
        look_type: u32,
        colours: [u16; 4],
        addons: u8,
    },
    /// The current native Item-definition dictionary, never an appearance_ref or sprite ID.
    Item { definition_ref: u32 },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub reference: u32,
    pub entity: EntityRef,
    pub context: Context,
    pub character_revision: u64,
    /// Absent when the world presents an implicit fallback without a durable selection.
    pub selection_revision: Option<u64>,
    pub condition_revision: u64,
    pub owner_time_us: u64,
    pub look: Look,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mappings {
    pub context: Context,
    pub bindings: Vec<Binding>,
    /// Explicit visibility leave/retirement. Omission never clears a temporary appearance.
    pub forget: Vec<EntityRef>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub catalogue: Catalogue,
    pub bindings: Vec<Binding>,
    /// Allocation tombstone watermark, including references of actors no longer visible.
    pub high_water: u32,
}

#[derive(Clone, Copy)]
enum Value<'a> {
    Scalar(u64),
    Bytes(&'a [u8]),
}
struct Fields<'a>(Vec<(u64, Value<'a>)>);
impl<'a> Fields<'a> {
    fn parse(input: &'a [u8], limit: usize, count: usize) -> Result<Self> {
        if input.len() > limit {
            return Err(Error::LimitExceeded);
        }
        let mut cursor = 0;
        let mut values = Vec::new();
        while cursor < input.len() {
            if values.len() == count {
                return Err(Error::LimitExceeded);
            }
            let tag = read_varint(input, &mut cursor)?;
            if tag >> 3 == 0 {
                return Err(Error::Malformed);
            }
            let value = match tag & 7 {
                0 => Value::Scalar(read_varint(input, &mut cursor)?),
                2 => Value::Bytes(read_bytes(input, &mut cursor)?),
                _ => return Err(Error::Malformed),
            };
            values.push((tag >> 3, value));
        }
        Ok(Self(values))
    }
    fn optional(&mut self, field: u64) -> Result<Option<Value<'a>>> {
        let mut result = None;
        self.0.retain(|(key, value)| {
            if *key == field {
                if result.is_none() {
                    result = Some(Ok(*value));
                } else {
                    result = Some(Err(Error::Malformed));
                }
                false
            } else {
                true
            }
        });
        result.transpose()
    }
    fn scalar(&mut self, field: u64) -> Result<u64> {
        match self.optional(field)? {
            Some(Value::Scalar(value)) => Ok(value),
            _ => Err(Error::Malformed),
        }
    }
    fn bytes(&mut self, field: u64) -> Result<&'a [u8]> {
        match self.optional(field)? {
            Some(Value::Bytes(value)) => Ok(value),
            _ => Err(Error::Malformed),
        }
    }
    fn optional_bytes(&mut self, field: u64) -> Result<Option<&'a [u8]>> {
        match self.optional(field)? {
            Some(Value::Bytes(value)) => Ok(Some(value)),
            None => Ok(None),
            _ => Err(Error::Malformed),
        }
    }
    fn repeated(&mut self, field: u64, maximum: usize) -> Result<Vec<&'a [u8]>> {
        let mut values = Vec::new();
        for (key, value) in &self.0 {
            if *key == field {
                if values.len() == maximum {
                    return Err(Error::LimitExceeded);
                }
                match value {
                    Value::Bytes(bytes) => values.push(*bytes),
                    _ => return Err(Error::Malformed),
                }
            }
        }
        self.0.retain(|(key, _)| *key != field);
        Ok(values)
    }
    fn finish(self) -> Result<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(Error::Malformed)
        }
    }
}
fn number<T: TryFrom<u64>>(value: u64) -> Result<T> {
    T::try_from(value).map_err(|_| Error::Malformed)
}
fn nonzero(value: u64) -> Result<u64> {
    if value == 0 {
        Err(Error::Malformed)
    } else {
        Ok(value)
    }
}
fn text(bytes: &[u8], limit: usize) -> Result<String> {
    if bytes.is_empty() || bytes.len() > limit {
        return Err(Error::LimitExceeded);
    }
    let value = std::str::from_utf8(bytes).map_err(|_| Error::Malformed)?;
    if value.chars().any(char::is_control) {
        return Err(Error::Malformed);
    }
    Ok(value.to_owned())
}
fn array<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes.try_into().map_err(|_| Error::Malformed)
}
fn bounded(bytes: Vec<u8>, limit: usize) -> Result<Vec<u8>> {
    if bytes.len() > limit {
        Err(Error::LimitExceeded)
    } else {
        Ok(bytes)
    }
}
fn put_text(out: &mut Vec<u8>, field: u64, value: &str, maximum: usize) -> Result<()> {
    text(value.as_bytes(), maximum)?;
    push_message_field(out, field, value.as_bytes());
    Ok(())
}
fn colours(bytes: &[u8]) -> Result<[u16; 4]> {
    let bytes: [u8; 8] = array(bytes)?;
    Ok(std::array::from_fn(|i| {
        u16::from_be_bytes([bytes[i * 2], bytes[i * 2 + 1]])
    }))
}
fn put_colours(out: &mut Vec<u8>, field: u64, value: [u16; 4]) {
    let bytes: Vec<u8> = value.into_iter().flat_map(u16::to_be_bytes).collect();
    push_message_field(out, field, &bytes);
}
fn put_context(out: &mut Vec<u8>, context: Context) {
    push_message_field(out, 1, &context.content);
    push_message_field(out, 2, &context.cosmetics);
}
fn context(fields: &mut Fields<'_>) -> Result<Context> {
    Ok(Context {
        content: array(fields.bytes(1)?)?,
        cosmetics: array(fields.bytes(2)?)?,
    })
}

pub fn encode_selection(value: &Selection) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    put_text(&mut out, 1, &value.outfit, MAX_KEY_BYTES)?;
    put_colours(&mut out, 2, value.colours);
    push_varint_field(&mut out, 3, u64::from(value.addons));
    if let Some(mount) = &value.mount {
        put_text(&mut out, 4, mount, MAX_KEY_BYTES)?;
    }
    bounded(out, MAX_SELECTION_BYTES)
}
pub fn decode_selection(input: &[u8]) -> Result<Selection> {
    let mut f = Fields::parse(input, MAX_SELECTION_BYTES, 5)?;
    let value = Selection {
        outfit: text(f.bytes(1)?, MAX_KEY_BYTES)?,
        colours: colours(f.bytes(2)?)?,
        addons: number(f.scalar(3)?)?,
        mount: f
            .optional_bytes(4)?
            .map(|b| text(b, MAX_KEY_BYTES))
            .transpose()?,
    };
    f.finish()?;
    Ok(value)
}
fn encode_selected(value: &Selected) -> Result<Vec<u8>> {
    nonzero(value.revision)?;
    let mut out = Vec::new();
    push_message_field(&mut out, 1, &encode_selection(&value.selection)?);
    push_varint_field(&mut out, 2, value.revision);
    put_text(&mut out, 3, &value.provenance, MAX_PROVENANCE_BYTES)?;
    Ok(out)
}
fn decode_selected(input: &[u8]) -> Result<Selected> {
    let mut f = Fields::parse(input, MAX_CHANGE_BYTES, 4)?;
    let value = Selected {
        selection: decode_selection(f.bytes(1)?)?,
        revision: nonzero(f.scalar(2)?)?,
        provenance: text(f.bytes(3)?, MAX_PROVENANCE_BYTES)?,
    };
    f.finish()?;
    Ok(value)
}
pub fn encode_change(value: &Change) -> Result<Vec<u8>> {
    nonzero(value.expected_character_revision)?;
    if value
        .before
        .as_ref()
        .is_some_and(|b| b.revision > value.expected_character_revision)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    put_context(&mut out, value.context);
    push_varint_field(&mut out, 3, value.expected_character_revision);
    if let Some(before) = &value.before {
        push_message_field(&mut out, 4, &encode_selected(before)?);
    }
    push_message_field(&mut out, 5, &encode_selection(&value.selection)?);
    put_text(&mut out, 6, &value.content_revision, MAX_PROVENANCE_BYTES)?;
    bounded(out, MAX_CHANGE_BYTES)
}
pub fn decode_change(input: &[u8]) -> Result<Change> {
    let mut f = Fields::parse(input, MAX_CHANGE_BYTES, 7)?;
    let value = Change {
        context: context(&mut f)?,
        expected_character_revision: nonzero(f.scalar(3)?)?,
        before: f.optional_bytes(4)?.map(decode_selected).transpose()?,
        selection: decode_selection(f.bytes(5)?)?,
        content_revision: text(f.bytes(6)?, MAX_PROVENANCE_BYTES)?,
    };
    f.finish()?;
    encode_change(&value)?;
    Ok(value)
}
pub fn encode_result(value: ChangeResult) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    push_varint_field(
        &mut out,
        1,
        if matches!(value, ChangeResult::Rejected) {
            2
        } else {
            1
        },
    );
    if let ChangeResult::Committed { revision } = value {
        push_varint_field(&mut out, 2, nonzero(revision)?);
    }
    bounded(out, MAX_RESULT_BYTES)
}
pub fn decode_result(input: &[u8]) -> Result<ChangeResult> {
    let mut f = Fields::parse(input, MAX_RESULT_BYTES, 3)?;
    let value = match f.scalar(1)? {
        1 => ChangeResult::Committed {
            revision: nonzero(f.scalar(2)?)?,
        },
        2 => ChangeResult::Rejected,
        _ => return Err(Error::Malformed),
    };
    f.finish()?;
    Ok(value)
}
fn encode_option(value: &OutfitOption) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    put_text(&mut out, 1, &value.key, MAX_KEY_BYTES)?;
    push_varint_field(&mut out, 2, nonzero(u64::from(value.look_type))?);
    push_varint_field(&mut out, 3, u64::from(value.allowed_addons));
    Ok(out)
}
fn decode_option(input: &[u8]) -> Result<OutfitOption> {
    let mut f = Fields::parse(input, MAX_KEY_BYTES + 32, 4)?;
    let value = OutfitOption {
        key: text(f.bytes(1)?, MAX_KEY_BYTES)?,
        look_type: number(nonzero(f.scalar(2)?)?)?,
        allowed_addons: number(f.scalar(3)?)?,
    };
    f.finish()?;
    Ok(value)
}
fn ascending<'a>(keys: impl IntoIterator<Item = &'a str>) -> Result<()> {
    let mut previous = None;
    for key in keys {
        if previous.is_some_and(|p| p >= key) {
            return Err(Error::Malformed);
        }
        previous = Some(key);
    }
    Ok(())
}
pub fn encode_catalogue(value: &Catalogue) -> Result<Vec<u8>> {
    nonzero(value.character_revision)?;
    if value.palette.is_empty()
        || value.palette.len() > MAX_PALETTE_COLOURS
        || value.outfits.len() > MAX_CATALOGUE_OUTFITS
        || value.mounts.len() > MAX_CATALOGUE_MOUNTS
    {
        return Err(Error::LimitExceeded);
    }
    if value
        .selected
        .as_ref()
        .is_some_and(|s| s.revision > value.character_revision)
    {
        return Err(Error::Malformed);
    }
    ascending(value.outfits.iter().map(|o| o.key.as_str()))?;
    ascending(value.mounts.iter().map(String::as_str))?;
    let mut out = Vec::new();
    put_context(&mut out, value.context);
    push_varint_field(&mut out, 3, value.character_revision);
    if let Some(selected) = &value.selected {
        push_message_field(&mut out, 4, &encode_selected(selected)?);
    }
    let palette: Vec<u8> = value.palette.iter().flatten().copied().collect();
    push_message_field(&mut out, 5, &palette);
    for outfit in &value.outfits {
        push_message_field(&mut out, 6, &encode_option(outfit)?);
    }
    for mount in &value.mounts {
        put_text(&mut out, 7, mount, MAX_KEY_BYTES)?;
    }
    bounded(out, MAX_CATALOGUE_BYTES)
}
pub fn decode_catalogue(input: &[u8]) -> Result<Catalogue> {
    let mut f = Fields::parse(
        input,
        MAX_CATALOGUE_BYTES,
        6 + MAX_CATALOGUE_OUTFITS + MAX_CATALOGUE_MOUNTS,
    )?;
    let context = context(&mut f)?;
    let character_revision = nonzero(f.scalar(3)?)?;
    let selected = f.optional_bytes(4)?.map(decode_selected).transpose()?;
    let raw = f.bytes(5)?;
    if raw.is_empty() || raw.len() % 3 != 0 || raw.len() / 3 > MAX_PALETTE_COLOURS {
        return Err(Error::LimitExceeded);
    }
    let palette = raw.chunks_exact(3).map(array).collect::<Result<Vec<_>>>()?;
    let outfits = f
        .repeated(6, MAX_CATALOGUE_OUTFITS)?
        .into_iter()
        .map(decode_option)
        .collect::<Result<_>>()?;
    let mounts = f
        .repeated(7, MAX_CATALOGUE_MOUNTS)?
        .into_iter()
        .map(|v| text(v, MAX_KEY_BYTES))
        .collect::<Result<_>>()?;
    f.finish()?;
    let value = Catalogue {
        context,
        character_revision,
        selected,
        palette,
        outfits,
        mounts,
    };
    encode_catalogue(&value)?;
    Ok(value)
}
fn encode_entity(value: EntityRef) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    push_message_field(&mut out, 1, &value.identity);
    push_varint_field(&mut out, 2, nonzero(value.generation)?);
    Ok(out)
}
fn decode_entity(input: &[u8]) -> Result<EntityRef> {
    let mut f = Fields::parse(input, 32, 3)?;
    let value = EntityRef {
        identity: array(f.bytes(1)?)?,
        generation: nonzero(f.scalar(2)?)?,
    };
    f.finish()?;
    Ok(value)
}
fn encode_look(value: &Look) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    match value {
        Look::Outfit {
            look_type,
            colours,
            addons,
        } => {
            push_varint_field(&mut out, 1, 1);
            push_varint_field(&mut out, 2, nonzero(u64::from(*look_type))?);
            put_colours(&mut out, 3, *colours);
            push_varint_field(&mut out, 4, u64::from(*addons));
        }
        Look::Item { definition_ref } => {
            push_varint_field(&mut out, 1, 2);
            push_varint_field(&mut out, 5, nonzero(u64::from(*definition_ref))?);
        }
    }
    Ok(out)
}
fn decode_look(input: &[u8]) -> Result<Look> {
    let mut f = Fields::parse(input, 48, 6)?;
    let value = match f.scalar(1)? {
        1 => Look::Outfit {
            look_type: number(nonzero(f.scalar(2)?)?)?,
            colours: colours(f.bytes(3)?)?,
            addons: number(f.scalar(4)?)?,
        },
        2 => Look::Item {
            definition_ref: number(nonzero(f.scalar(5)?)?)?,
        },
        _ => return Err(Error::Malformed),
    };
    f.finish()?;
    Ok(value)
}
pub fn encode_binding(value: &Binding) -> Result<Vec<u8>> {
    if value
        .selection_revision
        .is_some_and(|r| r == 0 || r > value.character_revision)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    put_context(&mut out, value.context);
    push_varint_field(&mut out, 3, nonzero(u64::from(value.reference))?);
    push_message_field(&mut out, 4, &encode_entity(value.entity)?);
    push_varint_field(&mut out, 5, nonzero(value.character_revision)?);
    push_varint_field(&mut out, 6, value.condition_revision);
    push_varint_field(&mut out, 7, value.owner_time_us);
    push_message_field(&mut out, 8, &encode_look(&value.look)?);
    if let Some(revision) = value.selection_revision {
        push_varint_field(&mut out, 9, revision);
    }
    bounded(out, MAX_BINDING_BYTES)
}
pub fn decode_binding(input: &[u8]) -> Result<Binding> {
    let mut f = Fields::parse(input, MAX_BINDING_BYTES, 10)?;
    let mut value = Binding {
        context: context(&mut f)?,
        reference: number(nonzero(f.scalar(3)?)?)?,
        entity: decode_entity(f.bytes(4)?)?,
        character_revision: nonzero(f.scalar(5)?)?,
        selection_revision: None,
        condition_revision: f.scalar(6)?,
        owner_time_us: f.scalar(7)?,
        look: decode_look(f.bytes(8)?)?,
    };
    value.selection_revision = match f.optional(9)? {
        Some(Value::Scalar(r)) => Some(nonzero(r)?),
        None => None,
        _ => return Err(Error::Malformed),
    };
    f.finish()?;
    encode_binding(&value)?;
    Ok(value)
}
fn validate_bindings(context: Context, bindings: &[Binding]) -> Result<()> {
    if bindings.len() > MAX_BINDINGS {
        return Err(Error::LimitExceeded);
    }
    let mut entities = BTreeSet::new();
    let mut refs = BTreeSet::new();
    let mut previous = 0;
    for binding in bindings {
        if binding.context != context
            || binding.reference <= previous
            || !entities.insert(binding.entity)
            || !refs.insert(binding.reference)
        {
            return Err(Error::Malformed);
        }
        previous = binding.reference;
    }
    Ok(())
}
pub fn encode_mappings(value: &Mappings) -> Result<Vec<u8>> {
    validate_bindings(value.context, &value.bindings)?;
    if value.bindings.len() + value.forget.len() > MAX_BINDINGS {
        return Err(Error::LimitExceeded);
    }
    let mut seen: BTreeSet<_> = value.bindings.iter().map(|b| b.entity).collect();
    let mut out = Vec::new();
    put_context(&mut out, value.context);
    for binding in &value.bindings {
        push_message_field(&mut out, 3, &encode_binding(binding)?);
    }
    for entity in &value.forget {
        if !seen.insert(*entity) {
            return Err(Error::Malformed);
        }
        push_message_field(&mut out, 4, &encode_entity(*entity)?);
    }
    bounded(out, MAX_MAPPINGS_BYTES)
}
pub fn decode_mappings(input: &[u8]) -> Result<Mappings> {
    let mut f = Fields::parse(input, MAX_MAPPINGS_BYTES, 3 + MAX_BINDINGS)?;
    let context = context(&mut f)?;
    let bindings = f
        .repeated(3, MAX_BINDINGS)?
        .into_iter()
        .map(decode_binding)
        .collect::<Result<_>>()?;
    let forget = f
        .repeated(4, MAX_BINDINGS)?
        .into_iter()
        .map(decode_entity)
        .collect::<Result<_>>()?;
    f.finish()?;
    let value = Mappings {
        context,
        bindings,
        forget,
    };
    encode_mappings(&value)?;
    Ok(value)
}
pub fn encode_snapshot(value: &Snapshot) -> Result<Vec<u8>> {
    validate_bindings(value.catalogue.context, &value.bindings)?;
    if value
        .bindings
        .iter()
        .any(|b| b.reference > value.high_water)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, &encode_catalogue(&value.catalogue)?);
    for binding in &value.bindings {
        push_message_field(&mut out, 2, &encode_binding(binding)?);
    }
    push_varint_field(&mut out, 3, u64::from(value.high_water));
    bounded(out, MAX_SNAPSHOT_BYTES)
}
pub fn decode_snapshot(input: &[u8]) -> Result<Snapshot> {
    let mut f = Fields::parse(input, MAX_SNAPSHOT_BYTES, 3 + MAX_BINDINGS)?;
    let catalogue = decode_catalogue(f.bytes(1)?)?;
    let bindings = f
        .repeated(2, MAX_BINDINGS)?
        .into_iter()
        .map(decode_binding)
        .collect::<Result<_>>()?;
    let high_water = number(f.scalar(3)?)?;
    f.finish()?;
    let value = Snapshot {
        catalogue,
        bindings,
        high_water,
    };
    encode_snapshot(&value)?;
    Ok(value)
}

/// A connection-local presentation dictionary. This is protocol state, never actor authority.
/// Transport must apply the mapping frame before the VIS-2 frame that references it, and
/// discard this entire object on reconnect (including resume), before accepting new bytes.
/// This store does not admit visible entities. The native Session VIS-2 reducer owns enter/
/// leave and EntityRef generation currentness; rendering calls resolve only for that exact
/// current visible EntityRef/reference. Unsolicited mappings never create a visible actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceStore {
    generation: u64,
    domain_revision: u64,
    item_dictionary: bool,
    catalogue: Catalogue,
    high_water: u32,
    bindings: std::collections::BTreeMap<EntityRef, Binding>,
}
impl ReferenceStore {
    pub fn from_snapshot(
        generation: u64,
        domain_revision: u64,
        selected: &[u32],
        snapshot: Snapshot,
    ) -> Result<Self> {
        nonzero(domain_revision)?;
        if !selected.contains(&CAPABILITY_APPEARANCE_V1) || !selected.contains(&6) {
            return Err(Error::Malformed);
        }
        nonzero(generation)?;
        encode_snapshot(&snapshot)?;
        let high_water = snapshot.high_water;
        let mut store = Self {
            generation,
            domain_revision,
            item_dictionary: selected.contains(&crate::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1),
            catalogue: snapshot.catalogue,
            high_water: 0,
            bindings: std::collections::BTreeMap::new(),
        };
        store.apply(
            generation,
            Mappings {
                context: store.catalogue.context,
                bindings: snapshot.bindings,
                forget: Vec::new(),
            },
        )?;
        store.high_water = high_water;
        Ok(store)
    }
    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }
    /// Authoritative same-connection resync retains allocation tombstones. A new connection
    /// uses from_snapshot instead. Outer Session still owns snapshot/sequence admission.
    pub fn replace_snapshot(
        &mut self,
        generation: u64,
        domain_revision: u64,
        snapshot: Snapshot,
    ) -> Result<()> {
        if generation != self.generation
            || domain_revision <= self.domain_revision
            || snapshot.high_water < self.high_water
            || snapshot.catalogue.context != self.catalogue.context
            || snapshot.catalogue.palette != self.catalogue.palette
            || snapshot.catalogue.character_revision < self.catalogue.character_revision
        {
            return Err(Error::Malformed);
        }
        for binding in &snapshot.bindings {
            if binding.reference <= self.high_water
                && self.bindings.get(&binding.entity) != Some(binding)
            {
                return Err(Error::Malformed);
            }
            if let Some(old) = self.bindings.get(&binding.entity)
                && (binding.character_revision < old.character_revision
                    || binding.selection_revision < old.selection_revision
                    || binding.condition_revision < old.condition_revision
                    || binding.owner_time_us < old.owner_time_us)
            {
                return Err(Error::Malformed);
            }
        }
        let caps = if self.item_dictionary {
            &[4, 6, 25][..]
        } else {
            &[6, 25][..]
        };
        let next = Self::from_snapshot(generation, domain_revision, caps, snapshot)?;
        *self = next;
        Ok(())
    }
    fn update_catalogue(&mut self, generation: u64, catalogue: Catalogue) -> Result<()> {
        encode_catalogue(&catalogue)?;
        if generation != self.generation
            || catalogue.context != self.catalogue.context
            || catalogue.palette != self.catalogue.palette
            || catalogue.character_revision < self.catalogue.character_revision
        {
            return Err(Error::Malformed);
        }
        self.catalogue = catalogue;
        Ok(())
    }
    /// Only the next actual domain21 delta may mutate this dictionary. Account unlock changes
    /// can replace a catalogue at the same Character revision; freshness is the envelope domain
    /// revision, not a manufactured Character write or an inferred catalogue counter.
    pub fn apply_delta(&mut self, generation: u64, delta: crate::StateDeltaView<'_>) -> Result<()> {
        if delta.domain_id != STATE_DOMAIN_APPEARANCE
            || delta.base_revision != self.domain_revision
            || self.domain_revision.checked_add(1) != Some(delta.new_revision)
        {
            return Err(Error::Malformed);
        }
        match delta.delta_type {
            DELTA_TYPE_APPEARANCE_MAPPINGS_V1 => {
                self.apply(generation, decode_mappings(delta.payload)?)?
            }
            DELTA_TYPE_APPEARANCE_CATALOGUE_V1 => {
                self.update_catalogue(generation, decode_catalogue(delta.payload)?)?
            }
            _ => return Err(Error::Malformed),
        }
        self.domain_revision = delta.new_revision;
        Ok(())
    }
    /// Atomic reducer: a malformed/stale batch changes no entry or allocation watermark.
    fn apply(&mut self, generation: u64, mappings: Mappings) -> Result<()> {
        encode_mappings(&mappings)?;
        if generation != self.generation || mappings.context != self.catalogue.context {
            return Err(Error::Malformed);
        }
        let mut next = self.clone();
        for entity in mappings.forget {
            next.bindings.remove(&entity);
        }
        for binding in mappings.bindings {
            if next.bindings.keys().any(|e| {
                e.identity == binding.entity.identity && e.generation > binding.entity.generation
            }) {
                return Err(Error::Malformed);
            }
            if matches!(binding.look, Look::Item { .. }) && !self.item_dictionary {
                return Err(Error::Malformed);
            }
            if let Look::Outfit { colours, .. } = &binding.look
                && colours
                    .iter()
                    .any(|c| usize::from(*c) >= self.catalogue.palette.len())
            {
                return Err(Error::Malformed);
            }
            if let Some(old) = next.bindings.get(&binding.entity) {
                if old == &binding {
                    continue;
                }
                if binding.character_revision < old.character_revision
                    || binding.selection_revision < old.selection_revision
                    || binding.condition_revision < old.condition_revision
                    || binding.owner_time_us < old.owner_time_us
                {
                    return Err(Error::Malformed);
                }
            }
            if binding.reference <= next.high_water {
                return Err(Error::Malformed);
            }
            next.high_water = binding.reference;
            // A new generation of one placement cannot inherit its previous look.
            next.bindings
                .retain(|entity, _| entity.identity != binding.entity.identity);
            next.bindings.insert(binding.entity, binding);
            if next.bindings.len() > MAX_BINDINGS {
                return Err(Error::LimitExceeded);
            }
        }
        *self = next;
        Ok(())
    }
    /// Missing/mismatched mappings require authoritative resync, never a guessed look.
    pub fn resolve(&self, generation: u64, entity: EntityRef, reference: u32) -> Result<&Binding> {
        if generation != self.generation {
            return Err(Error::Malformed);
        }
        self.bindings
            .get(&entity)
            .filter(|b| b.reference == reference)
            .ok_or(Error::Malformed)
    }
}

#[cfg(test)]
#[path = "appearance_tests.rs"]
mod tests;
