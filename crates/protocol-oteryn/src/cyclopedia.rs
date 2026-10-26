//! LOOP-MAP-1 basic map wire. Fixed, little-endian scalar layout, no unbounded fields.
//! Query: empty. Intent: operation:u8 (1 start, 2 shuffle, 3 find, 4 donate),
//! area:u32, amount:u32 (zero except donate). Results are documented in the contract.
pub const COMMAND_TYPE_CYCLOPEDIA_QUERY: u32 = 72;
pub const COMMAND_TYPE_CYCLOPEDIA_INTENT: u32 = 73;
pub const CAPABILITY_CYCLOPEDIA_MAP_V1: u32 = 57;
pub const COMMAND_TYPE_SUBAREA_QUERY: u32 = 109;
pub const CAPABILITY_SUBAREA_V1: u32 = 93;

/// SUBAREA-ADMIT-1: presence byte, then region and subarea cipsoft/map_area source ids.
/// Zero ids mean unknown coverage; never substitute the basic area's id.
pub fn encode_current_subarea(membership: Option<(u32, u32)>) -> [u8; 9] {
    let mut bytes = [0; 9];
    if let Some((region, subarea)) = membership {
        bytes[0] = 1;
        bytes[1..5].copy_from_slice(&region.to_le_bytes());
        bytes[5..9].copy_from_slice(&subarea.to_le_bytes());
    }
    bytes
}
pub const MAX_REQUEST_BYTES: usize = 9;
pub const MAX_RESULT_BYTES: usize = 29 + 8 * 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapIntent {
    pub operation: u8,
    pub amount: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapWireError;

pub fn decode_request(kind: u32, bytes: &[u8]) -> Result<MapIntent, MapWireError> {
    if kind == COMMAND_TYPE_CYCLOPEDIA_QUERY && bytes.is_empty() {
        return Ok(MapIntent {
            operation: 0,
            amount: 0,
        });
    }
    if kind != COMMAND_TYPE_CYCLOPEDIA_INTENT || bytes.len() != MAX_REQUEST_BYTES {
        return Err(MapWireError);
    }
    let area = u32::from_le_bytes(bytes[1..5].try_into().map_err(|_| MapWireError)?);
    let amount = u32::from_le_bytes(bytes[5..9].try_into().map_err(|_| MapWireError)?);
    if area != 1
        || !(1..=4).contains(&bytes[0])
        || (bytes[0] != 4 && amount != 0)
        || (bytes[0] == 4 && !(1..=10_000_000).contains(&amount))
    {
        return Err(MapWireError);
    }
    Ok(MapIntent {
        operation: bytes[0],
        amount,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapResult {
    /// 0 success, 1 invalid, 2 Premium required, 3 discovered, 4 no exploration,
    /// 5 insufficient reachable pool, 6 insufficient funds, 7 currently improved.
    pub code: u8,
    pub revision: u64,
    pub discovered: bool,
    pub found: u8,
    pub pool: u64,
    pub epoch: u64,
    pub improved: bool,
    /// Only unfound points within the requesting character's same-floor viewport.
    pub visible: Vec<(u16, u16, i8)>,
}
pub fn encode_result(result: &MapResult) -> Result<Vec<u8>, MapWireError> {
    if result.code > 7
        || result.revision == 0
        || result.found > 7
        || result.visible.len() > 8
        || (result.discovered && (result.found != 7 || !result.visible.is_empty()))
    {
        return Err(MapWireError);
    }
    if result
        .visible
        .iter()
        .enumerate()
        .any(|(i, p)| !(-15..=0).contains(&p.2) || result.visible[..i].contains(p))
    {
        return Err(MapWireError);
    }
    let mut bytes = Vec::with_capacity(29 + 5 * result.visible.len());
    bytes.push(result.code);
    bytes.extend_from_slice(&result.revision.to_le_bytes());
    bytes.push(u8::from(result.discovered));
    bytes.push(result.found);
    bytes.extend_from_slice(&result.pool.to_le_bytes());
    bytes.extend_from_slice(&result.epoch.to_le_bytes());
    bytes.push(if result.improved { 2 } else { 1 });
    bytes.push(result.visible.len() as u8);
    for (x, y, floor) in &result.visible {
        bytes.extend_from_slice(&x.to_le_bytes());
        bytes.extend_from_slice(&y.to_le_bytes());
        bytes.push(*floor as u8);
    }
    Ok(bytes)
}
/// Validate retained results again before the transport emits persisted bytes.
pub fn decode_result(bytes: &[u8]) -> Result<MapResult, MapWireError> {
    if !(29..=MAX_RESULT_BYTES).contains(&bytes.len()) {
        return Err(MapWireError);
    }
    let count = usize::from(bytes[28]);
    if count > 8
        || bytes.len() != 29 + count * 5
        || !matches!(bytes[9], 0 | 1)
        || !matches!(bytes[27], 1 | 2)
    {
        return Err(MapWireError);
    }
    let mut visible = Vec::with_capacity(count);
    for point in bytes[29..].as_chunks::<5>().0 {
        let pos = (
            u16::from_le_bytes([point[0], point[1]]),
            u16::from_le_bytes([point[2], point[3]]),
            point[4] as i8,
        );
        if !(-15..=0).contains(&pos.2) || visible.contains(&pos) {
            return Err(MapWireError);
        }
        visible.push(pos);
    }
    let r = MapResult {
        code: bytes[0],
        revision: u64::from_le_bytes(bytes[1..9].try_into().map_err(|_| MapWireError)?),
        discovered: bytes[9] == 1,
        found: bytes[10],
        pool: u64::from_le_bytes(bytes[11..19].try_into().map_err(|_| MapWireError)?),
        epoch: u64::from_le_bytes(bytes[19..27].try_into().map_err(|_| MapWireError)?),
        improved: bytes[27] == 2,
        visible,
    };
    encode_result(&r)?;
    Ok(r)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    #[test]
    fn cyclopedia_codec_rejects_invalid_and_bounds_results() {
        assert!(decode_request(72, &[]).is_ok());
        let valid = [4, 1, 0, 0, 0, 1, 0, 0, 0];
        assert!(decode_request(73, &valid).is_ok());
        for len in 0..9 {
            assert!(decode_request(73, &valid[..len]).is_err());
        }
        for invalid in [
            vec![0; 9],
            vec![1; 10],
            vec![1; 10000],
            vec![4, 1, 0, 0, 0, 0, 0, 0, 0],
            vec![1, 1, 0, 0, 0, 1, 0, 0, 0],
            vec![4, 2, 0, 0, 0, 1, 0, 0, 0],
        ] {
            assert!(decode_request(73, &invalid).is_err());
        }
        let mut r = MapResult {
            code: 0,
            revision: 1,
            discovered: false,
            found: 0,
            pool: 0,
            epoch: 0,
            improved: false,
            visible: (1..=8).map(|x| (x, 2, -7)).collect(),
        };
        assert!(encode_result(&r).is_ok_and(|bytes| bytes.len() <= MAX_RESULT_BYTES));
        let bytes = encode_result(&r)
            .map_err(|_| ())
            .expect("valid bounded result");
        assert_eq!(decode_result(&bytes), Ok(r.clone()));
        for length in 0..bytes.len() {
            assert!(decode_result(&bytes[..length]).is_err());
        }
        r.visible.push((1, 2, -7));
        assert!(encode_result(&r).is_err());
    }
}
