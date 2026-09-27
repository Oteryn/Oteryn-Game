use sqlx_core::bytes::{Buf, Bytes};
use sqlx_core::net::ResourceReservation;
use std::sync::Arc;

use crate::error::Error;
use crate::io::BufExt;
use crate::message::{BackendMessage, BackendMessageFormat};
use crate::types::Oid;

#[derive(Debug)]
pub struct RowDescription {
    pub fields: Vec<Field>,
    _allocation: Option<Arc<ResourceReservation>>,
}

#[derive(Debug)]
pub struct Field {
    /// The name of the field.
    pub name: String,

    /// If the field can be identified as a column of a specific table, the
    /// object ID of the table; otherwise zero.
    pub relation_id: Option<Oid>,

    /// If the field can be identified as a column of a specific table, the attribute number of
    /// the column; otherwise zero.
    pub relation_attribute_no: Option<i16>,

    /// The object ID of the field's data type.
    pub data_type_id: Oid,

    /// The data type size (see pg_type.typlen). Note that negative values denote
    /// variable-width types.
    #[allow(dead_code)]
    pub data_type_size: i16,

    /// The type modifier (see pg_attribute.atttypmod). The meaning of the
    /// modifier is type-specific.
    #[allow(dead_code)]
    pub type_modifier: i32,

    /// The format code being used for the field.
    #[allow(dead_code)]
    pub format: i16,
}

pub(crate) fn validate_wp3_first_slice_body(mut body: &[u8]) -> Result<(), Error> {
    let count_bytes = body
        .get(..2)
        .ok_or_else(|| err_protocol!("truncated WP3 RowDescription count"))?;
    let count = u16::from_be_bytes([count_bytes[0], count_bytes[1]]) as usize;
    if count > 32 {
        return Err(err_protocol!("WP3 RowDescription field count exceeds 32"));
    }
    body = &body[2..];

    let mut aggregate_name_bytes = 0usize;
    for _ in 0..count {
        let nul = body
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| err_protocol!("unterminated WP3 RowDescription field name"))?;
        let name = &body[..nul];
        std::str::from_utf8(name)
            .map_err(|_| err_protocol!("WP3 RowDescription field name is not UTF-8"))?;
        if name.len() > 63 {
            return Err(err_protocol!(
                "WP3 RowDescription field name exceeds 63 bytes"
            ));
        }
        aggregate_name_bytes = aggregate_name_bytes
            .checked_add(name.len())
            .ok_or_else(|| err_protocol!("WP3 RowDescription name total overflow"))?;
        if aggregate_name_bytes > 2_016 {
            return Err(err_protocol!(
                "WP3 RowDescription name total exceeds 2016 bytes"
            ));
        }
        body = body
            .get(nul + 1 + 18..)
            .ok_or_else(|| err_protocol!("truncated WP3 RowDescription field"))?;
    }
    if !body.is_empty() {
        return Err(err_protocol!("WP3 RowDescription has trailing bytes"));
    }
    Ok(())
}

fn wp3_decoded_allocation_bytes(body: &[u8]) -> Result<usize, Error> {
    validate_wp3_first_slice_body(body)?;
    let count = u16::from_be_bytes([body[0], body[1]]) as usize;
    let mut rest = &body[2..];
    let mut name_bytes = 0usize;
    for _ in 0..count {
        let nul = rest
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| err_protocol!("unterminated WP3 RowDescription field name"))?;
        name_bytes = name_bytes
            .checked_add(nul)
            .ok_or_else(|| err_protocol!("WP3 RowDescription allocation overflow"))?;
        rest = rest
            .get(nul + 1 + 18..)
            .ok_or_else(|| err_protocol!("truncated WP3 RowDescription field"))?;
    }
    count
        .checked_mul(std::mem::size_of::<Field>())
        .and_then(|bytes| bytes.checked_add(name_bytes))
        .ok_or_else(|| err_protocol!("WP3 RowDescription allocation overflow"))
}

impl BackendMessage for RowDescription {
    const FORMAT: BackendMessageFormat = BackendMessageFormat::RowDescription;

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

impl RowDescription {
    fn decode_impl(
        mut buf: Bytes,
        allocation: Option<Arc<ResourceReservation>>,
    ) -> Result<Self, Error> {
        let decoded_allocation = if let Some(message_allocation) = allocation.as_ref() {
            let bytes = wp3_decoded_allocation_bytes(&buf)?;
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
        if buf.len() < 2 {
            return Err(err_protocol!(
                "expected at least 2 bytes, got {}",
                buf.len()
            ));
        }

        let cnt = buf.get_u16();
        let mut fields = Vec::with_capacity(cnt as usize);

        for _ in 0..cnt {
            let name = buf.get_str_nul()?.to_owned();

            if buf.len() < 18 {
                return Err(err_protocol!(
                    "expected at least 18 bytes after field name {name:?}, got {}",
                    buf.len()
                ));
            }

            let relation_id = buf.get_u32();
            let relation_attribute_no = buf.get_i16();
            let data_type_id = Oid(buf.get_u32());
            let data_type_size = buf.get_i16();
            let type_modifier = buf.get_i32();
            let format = buf.get_i16();

            fields.push(Field {
                name,
                relation_id: if relation_id == 0 {
                    None
                } else {
                    Some(Oid(relation_id))
                },
                relation_attribute_no: if relation_attribute_no == 0 {
                    None
                } else {
                    Some(relation_attribute_no)
                },
                data_type_id,
                data_type_size,
                type_modifier,
                format,
            })
        }

        Ok(Self {
            fields,
            _allocation: decoded_allocation,
        })
    }
}

#[test]
fn wp3_first_slice_row_description_bounds_are_preallocation_checked() {
    fn field(name: &[u8]) -> Vec<u8> {
        let mut field = Vec::new();
        field.extend_from_slice(name);
        field.push(0);
        field.extend_from_slice(&[0; 18]);
        field
    }

    let mut max = Vec::new();
    max.extend_from_slice(&32u16.to_be_bytes());
    for _ in 0..32 {
        max.extend_from_slice(&field(&[b'a'; 63]));
    }
    assert!(validate_wp3_first_slice_body(&max).is_ok());

    let mut over_count = Vec::new();
    over_count.extend_from_slice(&33u16.to_be_bytes());
    for _ in 0..33 {
        over_count.extend_from_slice(&field(b"a"));
    }
    assert!(validate_wp3_first_slice_body(&over_count).is_err());

    let mut over_name = Vec::new();
    over_name.extend_from_slice(&1u16.to_be_bytes());
    over_name.extend_from_slice(&field(&[b'a'; 64]));
    assert!(validate_wp3_first_slice_body(&over_name).is_err());

    let mut invalid_utf8 = Vec::new();
    invalid_utf8.extend_from_slice(&1u16.to_be_bytes());
    invalid_utf8.extend_from_slice(&field(&[0xff]));
    assert!(validate_wp3_first_slice_body(&invalid_utf8).is_err());

    let mut trailing = max.clone();
    trailing.push(0);
    assert!(validate_wp3_first_slice_body(&trailing).is_err());
}

// TODO: Benchmark RowDescription
