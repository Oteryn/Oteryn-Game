//! Versioned, server-only static-cell carrier for engineering qualification.
//!
//! Its input is explicitly synthetic engineering provenance. The carrier has no Reference
//! evidence admission or activation conversion; a later Reference claim requires a separate
//! exact-field admission path. Only a direct addressed cell is read by `lookup`.

use super::{CollisionClass, ContentLockBinding, ContentLockEntry, CoordinateFrameRef, LogicalCell, MapRevisionRef, ProductionAtom, ProductionKey, Sha256HexDigest};
use crate::foundation::WorldId;
use std::collections::BTreeMap;

pub const STATIC_CELL_ENGINE_PROFILE: &str = "ENGINE_STATIC_CELL_CARRIER/v1";
pub const STATIC_CELL_ENGINE_CANDIDATE_MAX_CELLS: usize = 4;
// Engineering candidate only. This is not an admitted Reference corpus or registry maximum.
pub const STATIC_CELL_ENGINE_CANDIDATE_MAX_BYTES: usize = 2_743;
const MAGIC: &[u8; 8] = b"OTSCENG1";
const MAX_ATOM: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineeringStaticCellScope {
    pub world_id: WorldId,
    pub coordinate_frame: CoordinateFrameRef,
    pub map_revision: MapRevisionRef,
    /// Opaque digest supplied by the generation owner; the carrier cannot grant active status.
    pub generation_digest: [u8; 32],
    pub content_lock: ContentLockBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineeringCollisionClaim {
    Qualified(CollisionClass),
    Unqualified,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineeringStaticCellClaim {
    pub scope: EngineeringStaticCellScope,
    pub cell: LogicalCell,
    pub collision: EngineeringCollisionClaim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticCellEngineError {
    Empty,
    TooManyCells,
    TooManyBytes,
    Overflow,
    InvalidEncoding,
    InvalidLock,
    Duplicate,
    ScopeMismatch,
    Absent,
    Unqualified,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineeringStaticCellIndex {
    scope: EngineeringStaticCellScope,
    cells: BTreeMap<LogicalCell, EngineeringCollisionClaim>,
}

impl EngineeringStaticCellIndex {
    pub fn from_claims(claims: Vec<EngineeringStaticCellClaim>) -> Result<Self, StaticCellEngineError> {
        if claims.is_empty() { return Err(StaticCellEngineError::Empty); }
        if claims.len() > STATIC_CELL_ENGINE_CANDIDATE_MAX_CELLS {
            return Err(StaticCellEngineError::TooManyCells);
        }
        let scope = claims[0].scope.clone();
        validate_lock(&scope.content_lock)?;
        let mut cells = BTreeMap::new();
        for claim in claims {
            if claim.scope != scope { return Err(StaticCellEngineError::ScopeMismatch); }
            if cells.insert(claim.cell, claim.collision).is_some() {
                return Err(StaticCellEngineError::Duplicate);
            }
        }
        Ok(Self { scope, cells })
    }

    /// The caller must supply its independently resolved active scope. Equality with a stored
    /// claim alone does not establish that a generation remains active.
    pub fn lookup(&self, active_scope: &EngineeringStaticCellScope, cell: LogicalCell) -> Result<CollisionClass, StaticCellEngineError> {
        if active_scope != &self.scope { return Err(StaticCellEngineError::ScopeMismatch); }
        match self.cells.get(&cell).ok_or(StaticCellEngineError::Absent)? {
            EngineeringCollisionClaim::Qualified(value) => Ok(*value),
            EngineeringCollisionClaim::Unqualified => Err(StaticCellEngineError::Unqualified),
            EngineeringCollisionClaim::Conflict => Err(StaticCellEngineError::Conflict),
        }
    }

    /// Canonical bounded engineering encoding; there is no client projection or legality bit.
    pub fn encode_engineering(&self) -> Result<Vec<u8>, StaticCellEngineError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(self.scope.world_id.as_bytes());
        bytes.extend_from_slice(&self.scope.generation_digest);
        write_atom(&mut bytes, self.scope.coordinate_frame.as_str())?;
        write_atom(&mut bytes, self.scope.map_revision.as_str())?;
        write_atom(&mut bytes, self.scope.content_lock.revision_digest_token.as_str())?;
        let entry = validate_lock(&self.scope.content_lock)?;
        write_atom(&mut bytes, entry.package_key.as_str())?;
        write_atom(&mut bytes, entry.package_revision.as_str())?;
        bytes.extend_from_slice(entry.package_provenance_digest.as_str().as_bytes());
        bytes.push(u8::try_from(self.cells.len()).map_err(|_| StaticCellEngineError::Overflow)?);
        for (cell, claim) in &self.cells {
            bytes.extend_from_slice(&cell.x.to_le_bytes());
            bytes.extend_from_slice(&cell.y.to_le_bytes());
            bytes.extend_from_slice(&cell.z.to_le_bytes());
            bytes.push(match claim {
                EngineeringCollisionClaim::Qualified(CollisionClass::Walkable) => 1,
                EngineeringCollisionClaim::Qualified(CollisionClass::Blocked) => 2,
                EngineeringCollisionClaim::Unqualified => 3,
                EngineeringCollisionClaim::Conflict => 4,
            });
        }
        if bytes.len() > STATIC_CELL_ENGINE_CANDIDATE_MAX_BYTES { return Err(StaticCellEngineError::TooManyBytes); }
        Ok(bytes)
    }

    pub fn decode_engineering(bytes: &[u8]) -> Result<Self, StaticCellEngineError> {
        if bytes.len() > STATIC_CELL_ENGINE_CANDIDATE_MAX_BYTES { return Err(StaticCellEngineError::TooManyBytes); }
        let mut cursor = Cursor { bytes, position: 0 };
        if cursor.take(8)? != MAGIC { return Err(StaticCellEngineError::InvalidEncoding); }
        let world_id = WorldId::decode(cursor.take(16)?).map_err(|_| StaticCellEngineError::InvalidEncoding)?;
        let generation_digest: [u8; 32] = cursor.take(32)?.try_into().map_err(|_| StaticCellEngineError::InvalidEncoding)?;
        let coordinate_frame = CoordinateFrameRef::new(cursor.atom()?).map_err(|_| StaticCellEngineError::InvalidEncoding)?;
        let map_revision = MapRevisionRef::new(cursor.atom()?).map_err(|_| StaticCellEngineError::InvalidEncoding)?;
        let revision_digest_token = ProductionAtom::new("engineering Content Lock token", cursor.atom()?).map_err(|_| StaticCellEngineError::InvalidEncoding)?;
        let package_key = ProductionKey::new(cursor.atom()?).map_err(|_| StaticCellEngineError::InvalidEncoding)?;
        let package_revision = ProductionAtom::new("engineering package revision", cursor.atom()?).map_err(|_| StaticCellEngineError::InvalidEncoding)?;
        let digest = std::str::from_utf8(cursor.take(64)?).map_err(|_| StaticCellEngineError::InvalidEncoding)?;
        let package_provenance_digest = Sha256HexDigest::new(digest).map_err(|_| StaticCellEngineError::InvalidEncoding)?;
        let count = usize::from(cursor.take(1)?[0]);
        if count == 0 { return Err(StaticCellEngineError::Empty); }
        if count > STATIC_CELL_ENGINE_CANDIDATE_MAX_CELLS { return Err(StaticCellEngineError::TooManyCells); }
        let scope = EngineeringStaticCellScope {
            world_id, coordinate_frame, map_revision, generation_digest,
            content_lock: ContentLockBinding {
                revision_digest_token,
                entries: vec![ContentLockEntry::exact(package_key, package_revision, package_provenance_digest)],
            },
        };
        let mut claims = Vec::with_capacity(count);
        for _ in 0..count {
            let x = i32::from_le_bytes(cursor.take(4)?.try_into().map_err(|_| StaticCellEngineError::InvalidEncoding)?);
            let y = i32::from_le_bytes(cursor.take(4)?.try_into().map_err(|_| StaticCellEngineError::InvalidEncoding)?);
            let z = i32::from_le_bytes(cursor.take(4)?.try_into().map_err(|_| StaticCellEngineError::InvalidEncoding)?);
            let collision = match cursor.take(1)?[0] {
                1 => EngineeringCollisionClaim::Qualified(CollisionClass::Walkable),
                2 => EngineeringCollisionClaim::Qualified(CollisionClass::Blocked),
                3 => EngineeringCollisionClaim::Unqualified,
                4 => EngineeringCollisionClaim::Conflict,
                _ => return Err(StaticCellEngineError::InvalidEncoding),
            };
            claims.push(EngineeringStaticCellClaim { scope: scope.clone(), cell: LogicalCell { x, y, z }, collision });
        }
        if cursor.position != bytes.len() { return Err(StaticCellEngineError::InvalidEncoding); }
        let index = Self::from_claims(claims)?;
        if index.encode_engineering()?.as_slice() != bytes { return Err(StaticCellEngineError::InvalidEncoding); }
        Ok(index)
    }
}

fn validate_lock(lock: &ContentLockBinding) -> Result<&ContentLockEntry, StaticCellEngineError> {
    match lock.entries.as_slice() {
        [entry] if !entry.floating && !entry.dependency => Ok(entry),
        _ => Err(StaticCellEngineError::InvalidLock),
    }
}

fn write_atom(bytes: &mut Vec<u8>, value: &str) -> Result<(), StaticCellEngineError> {
    if value.is_empty() || value.len() > MAX_ATOM { return Err(StaticCellEngineError::InvalidEncoding); }
    let length = u16::try_from(value.len()).map_err(|_| StaticCellEngineError::Overflow)?;
    let next = bytes.len().checked_add(2).and_then(|n| n.checked_add(value.len())).ok_or(StaticCellEngineError::Overflow)?;
    if next > STATIC_CELL_ENGINE_CANDIDATE_MAX_BYTES { return Err(StaticCellEngineError::TooManyBytes); }
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

struct Cursor<'a> { bytes: &'a [u8], position: usize }
impl<'a> Cursor<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], StaticCellEngineError> {
        let end = self.position.checked_add(length).ok_or(StaticCellEngineError::Overflow)?;
        let result = self.bytes.get(self.position..end).ok_or(StaticCellEngineError::InvalidEncoding)?;
        self.position = end;
        Ok(result)
    }
    fn atom(&mut self) -> Result<&'a str, StaticCellEngineError> {
        let length = usize::from(u16::from_le_bytes(self.take(2)?.try_into().map_err(|_| StaticCellEngineError::InvalidEncoding)?));
        if length == 0 || length > MAX_ATOM { return Err(StaticCellEngineError::InvalidEncoding); }
        std::str::from_utf8(self.take(length)?).map_err(|_| StaticCellEngineError::InvalidEncoding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scope() -> EngineeringStaticCellScope {
        let mut world = [1u8; 16]; world[6] = 0x71; world[8] = 0x81;
        EngineeringStaticCellScope {
            world_id: WorldId::decode(&world).unwrap(),
            coordinate_frame: CoordinateFrameRef::new("engineering-frame").unwrap(),
            map_revision: MapRevisionRef::new("engineering-map").unwrap(),
            generation_digest: [7; 32],
            content_lock: ContentLockBinding {
                revision_digest_token: ProductionAtom::new("lock", "engineering-lock").unwrap(),
                entries: vec![ContentLockEntry::exact(
                    ProductionKey::new("engineering:package").unwrap(),
                    ProductionAtom::new("package", "engineering-revision").unwrap(),
                    Sha256HexDigest::new(&"a".repeat(64)).unwrap(),
                )],
            },
        }
    }
    fn claim(scope: &EngineeringStaticCellScope, x: i32, collision: EngineeringCollisionClaim) -> EngineeringStaticCellClaim {
        EngineeringStaticCellClaim { scope: scope.clone(), cell: LogicalCell { x, y: i32::MIN, z: i32::MAX }, collision }
    }
    #[test]
    fn static_cell_engine_bounded_roundtrip_and_exact_lookup() {
        let scope = scope();
        for n in [1, 2, 4] {
            let claims = (0..n).map(|x| claim(&scope, x, EngineeringCollisionClaim::Qualified(CollisionClass::Walkable))).collect();
            let index = EngineeringStaticCellIndex::from_claims(claims).unwrap();
            let encoded = index.encode_engineering().unwrap();
            assert!(encoded.len() <= STATIC_CELL_ENGINE_CANDIDATE_MAX_BYTES);
            assert_eq!(EngineeringStaticCellIndex::decode_engineering(&encoded), Ok(index.clone()));
            assert_eq!(index.lookup(&scope, LogicalCell { x: n-1, y: i32::MIN, z: i32::MAX }), Ok(CollisionClass::Walkable));
            assert_eq!(index.lookup(&scope, LogicalCell { x: n, y: i32::MIN, z: i32::MAX }), Err(StaticCellEngineError::Absent));
        }
        let five = (0..5).map(|x| claim(&scope, x, EngineeringCollisionClaim::Qualified(CollisionClass::Blocked))).collect();
        assert_eq!(EngineeringStaticCellIndex::from_claims(five), Err(StaticCellEngineError::TooManyCells));
    }
    #[test]
    fn static_cell_engine_rejects_duplicate_unqualified_conflict_and_scope_mismatch() {
        let scope = scope();
        let walk = claim(&scope, 0, EngineeringCollisionClaim::Qualified(CollisionClass::Walkable));
        assert_eq!(EngineeringStaticCellIndex::from_claims(vec![walk.clone(), walk.clone()]), Err(StaticCellEngineError::Duplicate));
        let index = EngineeringStaticCellIndex::from_claims(vec![walk]).unwrap();
        let cell = LogicalCell { x: 0, y: i32::MIN, z: i32::MAX };
        let mut other = scope.clone(); other.generation_digest[0] ^= 1;
        assert_eq!(index.lookup(&other, cell), Err(StaticCellEngineError::ScopeMismatch));
        other = scope.clone(); other.world_id = { let mut w = [2u8;16]; w[6]=0x72; w[8]=0x82; WorldId::decode(&w).unwrap() };
        assert_eq!(index.lookup(&other, cell), Err(StaticCellEngineError::ScopeMismatch));
        other = scope.clone(); other.coordinate_frame = CoordinateFrameRef::new("other-frame").unwrap();
        assert_eq!(index.lookup(&other, cell), Err(StaticCellEngineError::ScopeMismatch));
        other = scope.clone(); other.map_revision = MapRevisionRef::new("other-map").unwrap();
        assert_eq!(index.lookup(&other, cell), Err(StaticCellEngineError::ScopeMismatch));
        other = scope.clone(); other.content_lock.revision_digest_token = ProductionAtom::new("lock", "other-lock").unwrap();
        assert_eq!(index.lookup(&other, cell), Err(StaticCellEngineError::ScopeMismatch));
        for (value, error) in [(EngineeringCollisionClaim::Unqualified, StaticCellEngineError::Unqualified), (EngineeringCollisionClaim::Conflict, StaticCellEngineError::Conflict)] {
            let index = EngineeringStaticCellIndex::from_claims(vec![claim(&scope, 0, value)]).unwrap();
            assert_eq!(index.lookup(&scope, cell), Err(error));
        }
    }
    #[test]
    fn static_cell_engine_maximum_encoded_candidate_is_exact() {
        let mut scope = scope();
        let atom = "a".repeat(MAX_ATOM);
        scope.coordinate_frame = CoordinateFrameRef::new(&atom).unwrap();
        scope.map_revision = MapRevisionRef::new(&atom).unwrap();
        scope.content_lock.revision_digest_token = ProductionAtom::new("lock", &atom).unwrap();
        scope.content_lock.entries[0].package_key = ProductionKey::new(&format!("e:{}", "a".repeat(510))).unwrap();
        scope.content_lock.entries[0].package_revision = ProductionAtom::new("revision", &atom).unwrap();
        let claims = (0..STATIC_CELL_ENGINE_CANDIDATE_MAX_CELLS).map(|x| claim(&scope, i32::try_from(x).unwrap(), EngineeringCollisionClaim::Qualified(CollisionClass::Blocked))).collect();
        let index = EngineeringStaticCellIndex::from_claims(claims).unwrap();
        let bytes = index.encode_engineering().unwrap();
        assert_eq!(bytes.len(), STATIC_CELL_ENGINE_CANDIDATE_MAX_BYTES);
        assert_eq!(EngineeringStaticCellIndex::decode_engineering(&bytes), Ok(index));
        assert_eq!(EngineeringStaticCellIndex::decode_engineering(&[bytes, vec![0]].concat()), Err(StaticCellEngineError::TooManyBytes));
    }
    #[test]
    fn static_cell_engine_rejects_invalid_codec_and_bounds() {
        let scope = scope();
        let index = EngineeringStaticCellIndex::from_claims(vec![claim(&scope, 0, EngineeringCollisionClaim::Qualified(CollisionClass::Blocked))]).unwrap();
        let bytes = index.encode_engineering().unwrap();
        assert_eq!(EngineeringStaticCellIndex::decode_engineering(&bytes[..bytes.len()-1]), Err(StaticCellEngineError::InvalidEncoding));
        assert_eq!(EngineeringStaticCellIndex::decode_engineering(&vec![0; STATIC_CELL_ENGINE_CANDIDATE_MAX_BYTES+1]), Err(StaticCellEngineError::TooManyBytes));
        let mut bad = bytes.clone(); bad[0] ^= 1;
        assert_eq!(EngineeringStaticCellIndex::decode_engineering(&bad), Err(StaticCellEngineError::InvalidEncoding));
        let mut extra = bytes; extra.push(0);
        assert_eq!(EngineeringStaticCellIndex::decode_engineering(&extra), Err(StaticCellEngineError::InvalidEncoding));
        let mut cursor = Cursor { bytes: &[], position: usize::MAX };
        assert_eq!(cursor.take(1), Err(StaticCellEngineError::Overflow));
        let mut invalid = scope;
        invalid.content_lock.entries[0].floating = true;
        assert_eq!(EngineeringStaticCellIndex::from_claims(vec![claim(&invalid, 0, EngineeringCollisionClaim::Unqualified)]), Err(StaticCellEngineError::InvalidLock));
    }
}
