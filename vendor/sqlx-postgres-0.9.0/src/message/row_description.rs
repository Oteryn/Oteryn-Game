use sqlx_core::bytes::Buf;
use sqlx_core::net::OwnedBytes as Bytes;

use crate::error::Error;
use crate::message::{BackendMessage, BackendMessageFormat};
use crate::types::Oid;

#[derive(Debug)]
pub struct RowDescription {
    pub fields: Vec<Field>,
    _allocation: crate::statement::AllocationLease,
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

impl BackendMessage for RowDescription {
    const FORMAT: BackendMessageFormat = BackendMessageFormat::RowDescription;

    fn decode_body(mut buf: Bytes) -> Result<Self, Error> {
        if buf.len() < 2 {
            return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
        }

        let cnt = buf.get_u16();
        let mut tail = &buf[..];
        let mut name_bytes = 0usize;
        for _ in 0..cnt {
            let nul = tail
                .iter()
                .position(|b| *b == 0)
                .ok_or_else(|| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
            std::str::from_utf8(&tail[..nul])
                .map_err(|_| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
            name_bytes = name_bytes
                .checked_add(nul)
                .ok_or_else(crate::statement::allocation_denied)?;
            tail = tail
                .get(nul + 19..)
                .ok_or_else(|| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
        }
        if !tail.is_empty() {
            return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
        }
        let allocation = crate::statement::AllocationLease::reserve(
            buf.budget(),
            (cnt as usize)
                .checked_mul(std::mem::size_of::<Field>())
                .and_then(|n| n.checked_add(name_bytes))
                .ok_or_else(crate::statement::allocation_denied)?,
        )?;
        let mut fields = Vec::with_capacity(cnt as usize);

        for _ in 0..cnt {
            let name_bytes = buf.get_bytes_nul()?;
            let name = std::str::from_utf8(&name_bytes)
                .expect("preflight validated field name")
                .to_owned();

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
            _allocation: allocation,
        })
    }
}

// TODO: Unit Test RowDescription
// TODO: Benchmark RowDescription
