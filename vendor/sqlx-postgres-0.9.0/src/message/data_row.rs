use byteorder::{BigEndian, ByteOrder};
use sqlx_core::bytes::Bytes;
use sqlx_core::net::ResourceReservation;
use std::ops::{Deref, Range};
use std::sync::Arc;

use crate::error::Error;
use crate::message::{BackendMessage, BackendMessageFormat};

#[derive(Clone, Debug)]
pub(crate) struct ChargedBytes {
    bytes: Bytes,
    allocation: Option<Arc<ResourceReservation>>,
}

impl ChargedBytes {
    pub(crate) fn unowned(bytes: Bytes) -> Self {
        Self {
            bytes,
            allocation: None,
        }
    }

    pub(crate) fn slice_ref(&self, subset: &[u8]) -> Self {
        Self {
            bytes: self.bytes.slice_ref(subset),
            allocation: self.allocation.clone(),
        }
    }
}

impl Deref for ChargedBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.bytes
    }
}

/// A row of data from the database.
#[derive(Debug)]
pub struct DataRow {
    pub(crate) storage: ChargedBytes,
    _values_allocation: Option<Arc<ResourceReservation>>,

    /// Ranges into the stored row data.
    /// This uses `u32` instead of usize to reduce the size of this type. Values cannot be larger
    /// than `i32` in postgres.
    pub(crate) values: Vec<Option<Range<u32>>>,
}

impl DataRow {
    #[inline]
    pub(crate) fn get(&self, index: usize) -> Option<&'_ [u8]> {
        self.values[index]
            .as_ref()
            .map(|col| &self.storage[(col.start as usize)..(col.end as usize)])
    }
}

pub(crate) fn validate_wp3_first_slice_body(buf: &[u8]) -> Result<(), Error> {
    if buf.len() < 2 {
        return Err(err_protocol!("truncated WP3 DataRow count"));
    }
    let count = BigEndian::read_u16(buf) as usize;
    if count > 32 {
        return Err(err_protocol!("WP3 DataRow field count exceeds 32"));
    }
    let mut offset = 2usize;
    let mut value_bytes = 0usize;
    for _ in 0..count {
        let prefix = buf
            .get(offset..offset.checked_add(4).ok_or_else(|| err_protocol!("WP3 DataRow offset overflow"))?)
            .ok_or_else(|| err_protocol!("truncated WP3 DataRow length"))?;
        let length = BigEndian::read_i32(prefix);
        offset += 4;
        match length {
            -1 => {}
            length if length >= 0 => {
                let length = usize::try_from(length)
                    .map_err(|_| err_protocol!("invalid WP3 DataRow length"))?;
                value_bytes = value_bytes
                    .checked_add(length)
                    .ok_or_else(|| err_protocol!("WP3 DataRow value byte overflow"))?;
                if value_bytes > 131_072 {
                    return Err(err_protocol!("WP3 DataRow value bytes exceed 131072"));
                }
                offset = offset
                    .checked_add(length)
                    .ok_or_else(|| err_protocol!("WP3 DataRow value offset overflow"))?;
                if offset > buf.len() {
                    return Err(err_protocol!("truncated WP3 DataRow value"));
                }
            }
            _ => return Err(err_protocol!("invalid negative WP3 DataRow length")),
        }
    }
    if offset != buf.len() {
        return Err(err_protocol!("WP3 DataRow has trailing bytes"));
    }
    Ok(())
}

impl BackendMessage for DataRow {
    const FORMAT: BackendMessageFormat = BackendMessageFormat::DataRow;

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

impl DataRow {
    fn decode_impl(
        buf: Bytes,
        allocation: Option<Arc<ResourceReservation>>,
    ) -> Result<Self, Error> {
        if buf.len() < 2 {
            return Err(err_protocol!(
                "expected at least 2 bytes, got {}",
                buf.len()
            ));
        }

        let cnt = BigEndian::read_u16(&buf) as usize;
        let values_allocation = if let Some(message_allocation) = allocation.as_ref() {
            validate_wp3_first_slice_body(&buf)?;
            let bytes = cnt
                .checked_mul(std::mem::size_of::<Option<Range<u32>>>())
                .ok_or_else(|| err_protocol!("WP3 DataRow vector allocation overflow"))?;
            if bytes == 0 {
                None
            } else {
                Some(
                    ResourceReservation::try_new_shared(message_allocation.budget(), bytes)
                        .map_err(|_| Error::Io(std::io::ErrorKind::OutOfMemory.into()))?,
                )
            }
        } else {
            None
        };

        let mut values = Vec::with_capacity(cnt);
        let mut offset: u32 = 2;

        for _ in 0..cnt {
            let value_start = offset
                .checked_add(4)
                .ok_or_else(|| err_protocol!("next value start out of range (offset: {offset})"))?;

            // widen both to a larger type for a safe comparison
            if (buf.len() as u64) < (value_start as u64) {
                return Err(err_protocol!(
                    "expected 4 bytes at offset {offset}, got {}",
                    (value_start as u64) - (buf.len() as u64)
                ));
            }

            // Length of the column value, in bytes (this count does not include itself).
            // Can be zero. As a special case, -1 indicates a NULL column value.
            // No value bytes follow in the NULL case.
            //
            // we know `offset` is within range of `buf.len()` from the above check
            #[allow(clippy::cast_possible_truncation)]
            let length = BigEndian::read_i32(&buf[(offset as usize)..]);

            if let Ok(length) = u32::try_from(length) {
                let value_end = value_start.checked_add(length).ok_or_else(|| {
                    err_protocol!("value_start + length out of range ({offset} + {length})")
                })?;

                values.push(Some(value_start..value_end));
                offset = value_end;
            } else {
                // Negative values signify NULL
                values.push(None);
                // `value_start` is actually the next value now.
                offset = value_start;
            }
        }

        Ok(Self {
            storage: ChargedBytes {
                bytes: buf,
                allocation,
            },
            _values_allocation: values_allocation,
            values,
        })
    }
}

#[test]
fn test_decode_data_row() {
    const DATA: &[u8] = b"\
        \x00\x08\
        \xff\xff\xff\xff\
        \x00\x00\x00\x04\
        \x00\x00\x00\n\
        \xff\xff\xff\xff\
        \x00\x00\x00\x04\
        \x00\x00\x00\x14\
        \xff\xff\xff\xff\
        \x00\x00\x00\x04\
        \x00\x00\x00(\
        \xff\xff\xff\xff\
        \x00\x00\x00\x04\
        \x00\x00\x00P";

    let row = DataRow::decode_body(DATA.into()).unwrap();

    assert_eq!(row.values.len(), 8);

    assert!(row.get(0).is_none());
    assert_eq!(row.get(1).unwrap(), &[0_u8, 0, 0, 10][..]);
    assert!(row.get(2).is_none());
    assert_eq!(row.get(3).unwrap(), &[0_u8, 0, 0, 20][..]);
    assert!(row.get(4).is_none());
    assert_eq!(row.get(5).unwrap(), &[0_u8, 0, 0, 40][..]);
    assert!(row.get(6).is_none());
    assert_eq!(row.get(7).unwrap(), &[0_u8, 0, 0, 80][..]);
}

#[test]
fn wp3_first_slice_data_row_bounds_are_preallocation_checked() {
    let mut max_nulls = Vec::new();
    max_nulls.extend_from_slice(&32u16.to_be_bytes());
    for _ in 0..32 {
        max_nulls.extend_from_slice(&(-1i32).to_be_bytes());
    }
    assert!(validate_wp3_first_slice_body(&max_nulls).is_ok());

    let mut over_fields = Vec::new();
    over_fields.extend_from_slice(&33u16.to_be_bytes());
    for _ in 0..33 {
        over_fields.extend_from_slice(&(-1i32).to_be_bytes());
    }
    assert!(validate_wp3_first_slice_body(&over_fields).is_err());

    let mut max_value = Vec::new();
    max_value.extend_from_slice(&1u16.to_be_bytes());
    max_value.extend_from_slice(&131_072i32.to_be_bytes());
    max_value.resize(2 + 4 + 131_072, 0);
    assert!(validate_wp3_first_slice_body(&max_value).is_ok());

    let mut over_value = Vec::new();
    over_value.extend_from_slice(&1u16.to_be_bytes());
    over_value.extend_from_slice(&131_073i32.to_be_bytes());
    assert!(validate_wp3_first_slice_body(&over_value).is_err());

    let mut trailing = max_nulls.clone();
    trailing.push(0);
    assert!(validate_wp3_first_slice_body(&trailing).is_err());
    assert!(validate_wp3_first_slice_body(&[0, 1, 0, 0]).is_err());
    assert!(validate_wp3_first_slice_body(&[0, 1, 0xff, 0xff, 0xff, 0xfe]).is_err());
}

#[cfg(all(test, not(debug_assertions)))]
#[bench]
fn bench_data_row_get(b: &mut test::Bencher) {
    const DATA: &[u8] = b"\x00\x08\xff\xff\xff\xff\x00\x00\x00\x04\x00\x00\x00\n\xff\xff\xff\xff\x00\x00\x00\x04\x00\x00\x00\x14\xff\xff\xff\xff\x00\x00\x00\x04\x00\x00\x00(\xff\xff\xff\xff\x00\x00\x00\x04\x00\x00\x00P";

    let row = DataRow::decode_body(test::black_box(Bytes::from_static(DATA))).unwrap();

    b.iter(|| {
        let _value = test::black_box(&row).get(3);
    });
}

#[cfg(all(test, not(debug_assertions)))]
#[bench]
fn bench_decode_data_row(b: &mut test::Bencher) {
    const DATA: &[u8] = b"\x00\x08\xff\xff\xff\xff\x00\x00\x00\x04\x00\x00\x00\n\xff\xff\xff\xff\x00\x00\x00\x04\x00\x00\x00\x14\xff\xff\xff\xff\x00\x00\x00\x04\x00\x00\x00(\xff\xff\xff\xff\x00\x00\x00\x04\x00\x00\x00P";

    b.iter(|| {
        let _ = DataRow::decode_body(test::black_box(Bytes::from_static(DATA)));
    });
}
