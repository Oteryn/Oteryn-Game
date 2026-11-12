//! Client half of the existing asymmetric HOUSE-TRANSFER-1 codec.
use oteryn_protocol_oteryn::charm::CyclopediaWireError as Error;
pub use oteryn_protocol_oteryn::house_transfer::*;

pub fn encode_house_transfer_intent(intent: &HouseTransferIntent) -> Result<Vec<u8>, Error> {
    if intent.house.len() > 512 {
        return Err(Error::LimitExceeded);
    }
    let mut out = vec![8, if intent.exit { 2 } else { 1 }, 18];
    varint(&mut out, intent.house.len() as u64);
    out.extend_from_slice(intent.house.as_bytes());
    // Keep the protocol owner authoritative for input validation.
    decode_house_transfer_intent(&out)?;
    Ok(out)
}
fn varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        out.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
fn read(input: &mut &[u8]) -> Result<u64, Error> {
    let mut value = 0;
    for shift in (0..70).step_by(7) {
        let (&byte, rest) = input.split_first().ok_or(Error::Malformed)?;
        *input = rest;
        if shift == 63 && byte > 1 {
            return Err(Error::Malformed);
        }
        value |= u64::from(byte & 127) << shift;
        if byte & 128 == 0 {
            return Ok(value);
        }
    }
    Err(Error::Malformed)
}
fn bytes<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    let len = usize::try_from(read(input)?).map_err(|_| Error::Malformed)?;
    let value = input.get(..len).ok_or(Error::Malformed)?;
    *input = &input[len..];
    Ok(value)
}

pub fn decode_house_transfer_ack(mut input: &[u8]) -> Result<HouseTransferAck, Error> {
    if input.len() > MAX_ACK_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut ids = [None; 4];
    let mut house = None;
    let mut values = [None; 6];
    let mut selection = Vec::new();
    while !input.is_empty() {
        let tag = read(&mut input)?;
        match tag {
            10 | 18 | 26 | 34 => {
                let slot = &mut ids[(tag / 8 - 1) as usize];
                if slot.is_some() {
                    return Err(Error::Malformed);
                }
                *slot = Some(
                    bytes(&mut input)?
                        .try_into()
                        .map_err(|_| Error::Malformed)?,
                );
            }
            42 => {
                if house.is_some() {
                    return Err(Error::Malformed);
                }
                house = Some(
                    std::str::from_utf8(bytes(&mut input)?)
                        .map_err(|_| Error::Malformed)?
                        .to_owned(),
                );
            }
            48 | 56 | 64 | 80 | 88 | 96 => {
                let index = match tag {
                    48 => 0,
                    56 => 1,
                    64 => 2,
                    80 => 3,
                    88 => 4,
                    _ => 5,
                };
                if values[index].is_some() {
                    return Err(Error::Malformed);
                }
                values[index] = Some(read(&mut input)?);
            }
            72 => {
                if selection.len() == oteryn_protocol_oteryn::MAX_CAPABILITY_COUNT {
                    return Err(Error::LimitExceeded);
                }
                selection.push(u32::try_from(read(&mut input)?).map_err(|_| Error::Malformed)?);
            }
            _ => return Err(Error::Malformed),
        }
    }
    let coordinate = |index: usize| -> Result<i32, Error> {
        let value = values[index].ok_or(Error::Malformed)?;
        let value = u32::try_from(value).map_err(|_| Error::Malformed)?;
        Ok(((value >> 1) as i32) ^ -((value & 1) as i32))
    };
    let ack = HouseTransferAck {
        transfer: ids[0].ok_or(Error::Malformed)?,
        session: ids[1].ok_or(Error::Malformed)?,
        world: ids[2].ok_or(Error::Malformed)?,
        channel: ids[3].ok_or(Error::Malformed)?,
        house,
        scope_generation: values[0].ok_or(Error::Malformed)?,
        connection_generation: values[1].ok_or(Error::Malformed)?,
        lease_generation: values[2].ok_or(Error::Malformed)?,
        selection,
        position: [coordinate(3)?, coordinate(4)?, coordinate(5)?],
    };
    encode_house_transfer_ack(&ack)?;
    Ok(ack)
}
