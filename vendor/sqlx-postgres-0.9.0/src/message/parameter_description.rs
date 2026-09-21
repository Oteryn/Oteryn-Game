use smallvec::SmallVec;
use sqlx_core::bytes::{Buf, Bytes};
use sqlx_core::net::ResourceReservation;
use std::sync::Arc;

use crate::error::Error;
use crate::message::{BackendMessage, BackendMessageFormat};
use crate::types::Oid;

#[derive(Debug)]
pub struct ParameterDescription {
    pub types: SmallVec<[Oid; 6]>,
    _allocation: Option<Arc<ResourceReservation>>,
}

pub(crate) fn validate_wp3_first_slice_body(buf: &[u8]) -> Result<(), Error> {
    let count_bytes = buf
        .get(..2)
        .ok_or_else(|| err_protocol!("truncated WP3 ParameterDescription count"))?;
    let count = u16::from_be_bytes([count_bytes[0], count_bytes[1]]) as usize;
    if count > 32 {
        return Err(err_protocol!(
            "WP3 ParameterDescription parameter count exceeds 32"
        ));
    }
    let expected = 2usize
        .checked_add(
            count
                .checked_mul(4)
                .ok_or_else(|| err_protocol!("WP3 ParameterDescription size overflow"))?,
        )
        .ok_or_else(|| err_protocol!("WP3 ParameterDescription size overflow"))?;
    if buf.len() != expected {
        return Err(err_protocol!(
            "WP3 ParameterDescription body length is out of profile"
        ));
    }
    Ok(())
}

impl BackendMessage for ParameterDescription {
    const FORMAT: BackendMessageFormat = BackendMessageFormat::ParameterDescription;

    fn decode_body(buf: Bytes) -> Result<Self, Error> {
        Self::decode_impl(buf, None)
    }

    fn decode_body_charged(
        buf: Bytes,
        allocation: Option<Arc<ResourceReservation>>,
    ) -> Result<Self, Error> {
        Self::decode_impl(buf, allocation)
    }
}

impl ParameterDescription {
    fn decode_impl(
        mut buf: Bytes,
        allocation: Option<Arc<ResourceReservation>>,
    ) -> Result<Self, Error> {
        if allocation.is_some() {
            validate_wp3_first_slice_body(&buf)?;
        }
        // Note: this is correct, max parameters is 65535, not 32767
        // https://github.com/launchbadge/sqlx/issues/3464
        let cnt = buf.get_u16();
        let decoded_allocation = if let Some(message_allocation) = allocation.as_ref() {
            let count = cnt as usize;
            if count <= 6 {
                None
            } else {
                let bytes = count
                    .checked_mul(std::mem::size_of::<Oid>())
                    .ok_or_else(|| err_protocol!("WP3 ParameterDescription allocation overflow"))?;
                Some(
                    ResourceReservation::try_new_shared(message_allocation.budget(), bytes)
                        .map_err(|_| Error::Io(std::io::ErrorKind::OutOfMemory.into()))?,
                )
            }
        } else {
            None
        };
        let mut types = SmallVec::with_capacity(cnt as usize);

        for _ in 0..cnt {
            types.push(Oid(buf.get_u32()));
        }

        Ok(Self {
            types,
            _allocation: decoded_allocation,
        })
    }
}

#[test]
fn test_decode_parameter_description() {
    const DATA: &[u8] = b"\x00\x02\x00\x00\x00\x00\x00\x00\x05\x00";

    let m = ParameterDescription::decode_body(DATA.into()).unwrap();

    assert_eq!(m.types.len(), 2);
    assert_eq!(m.types[0], Oid(0x0000_0000));
    assert_eq!(m.types[1], Oid(0x0000_0500));
}

#[test]
fn test_decode_empty_parameter_description() {
    const DATA: &[u8] = b"\x00\x00";

    let m = ParameterDescription::decode_body(DATA.into()).unwrap();

    assert!(m.types.is_empty());
}

#[test]
fn wp3_first_slice_parameter_description_bounds_are_preallocation_checked() {
    let mut max = Vec::new();
    max.extend_from_slice(&32u16.to_be_bytes());
    max.resize(2 + 32 * 4, 0);
    assert!(validate_wp3_first_slice_body(&max).is_ok());

    let mut over = Vec::new();
    over.extend_from_slice(&33u16.to_be_bytes());
    over.resize(2 + 33 * 4, 0);
    assert!(validate_wp3_first_slice_body(&over).is_err());

    let mut trailing = max.clone();
    trailing.push(0);
    assert!(validate_wp3_first_slice_body(&trailing).is_err());
    assert!(validate_wp3_first_slice_body(&[0, 1, 0, 0, 0]).is_err());
}

#[cfg(all(test, not(debug_assertions)))]
#[bench]
fn bench_decode_parameter_description(b: &mut test::Bencher) {
    const DATA: &[u8] = b"\x00\x02\x00\x00\x00\x00\x00\x00\x05\x00";

    b.iter(|| {
        ParameterDescription::decode_body(test::black_box(Bytes::from_static(DATA))).unwrap();
    });
}
