use crate::error::{BoxDynError, UnexpectedNullError};
use crate::{PgTypeInfo, Postgres};
use sqlx_core::bytes::Buf;
use sqlx_core::net::OwnedBytes as Bytes;
pub(crate) use sqlx_core::value::{Value, ValueRef};
use std::borrow::Cow;
use std::str::from_utf8;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum PgValueFormat {
    Text = 0,
    Binary = 1,
}

/// Implementation of [`ValueRef`] for PostgreSQL.
#[derive(Clone)]
pub struct PgValueRef<'r> {
    pub(crate) value: Option<&'r [u8]>,
    pub(crate) row: Option<&'r Bytes>,
    pub(crate) type_info: PgTypeInfo,
    pub(crate) format: PgValueFormat,
}

/// Implementation of [`Value`] for PostgreSQL.
#[derive(Clone)]
pub struct PgValue {
    pub(crate) value: Option<Bytes>,
    pub(crate) type_info: PgTypeInfo,
    pub(crate) format: PgValueFormat,
}

impl<'r> PgValueRef<'r> {
    pub(crate) fn get(
        buf: &mut &'r [u8],
        format: PgValueFormat,
        ty: PgTypeInfo,
    ) -> Result<Self, String> {
        let element_len = buf.get_i32();

        let element_val = if element_len == -1 {
            None
        } else {
            let element_len: usize = element_len
                .try_into()
                .map_err(|_| format!("overflow converting element_len ({element_len}) to usize"))?;

            let val = &buf[..element_len];
            buf.advance(element_len);
            Some(val)
        };

        Ok(PgValueRef {
            value: element_val,
            row: None,
            type_info: ty,
            format,
        })
    }

    pub fn format(&self) -> PgValueFormat {
        self.format
    }

    pub fn as_bytes(&self) -> Result<&'r [u8], BoxDynError> {
        match &self.value {
            Some(v) => Ok(v),
            None => Err(UnexpectedNullError.into()),
        }
    }

    pub fn as_str(&self) -> Result<&'r str, BoxDynError> {
        Ok(from_utf8(self.as_bytes()?)?)
    }
}

impl Value for PgValue {
    type Database = Postgres;

    #[inline]
    fn as_ref(&self) -> PgValueRef<'_> {
        PgValueRef {
            value: self.value.as_deref(),
            row: self.value.as_ref(),
            type_info: self.type_info.clone(),
            format: self.format,
        }
    }

    fn type_info(&self) -> Cow<'_, PgTypeInfo> {
        Cow::Borrowed(&self.type_info)
    }

    fn is_null(&self) -> bool {
        self.value.is_none()
    }
}

impl<'r> ValueRef<'r> for PgValueRef<'r> {
    type Database = Postgres;

    fn to_owned(&self) -> PgValue {
        let value = match (self.row, self.value) {
            (Some(row), Some(value)) => Some(row.slice_ref(value)),

            (None, Some(value)) => Some(Bytes::copy_from_slice(value)),

            _ => None,
        };

        PgValue {
            value,
            format: self.format,
            type_info: self.type_info.clone(),
        }
    }

    fn type_info(&self) -> Cow<'_, PgTypeInfo> {
        Cow::Borrowed(&self.type_info)
    }

    fn is_null(&self) -> bool {
        self.value.is_none()
    }
}

#[cfg(test)]
mod repair1_tests {
    use super::*;
    use crate::statement::custody_test_support::Ledger;

    #[test]
    fn owned_public_roundtrips_preserve_backing_without_spare_balance() {
        let budget = Ledger::new(usize::MAX);
        let mut value = PgValue {
            value: Some(Bytes::try_copy_from_slice(b"retained", budget.clone()).unwrap()),
            type_info: PgTypeInfo::TEXT,
            format: PgValueFormat::Text,
        };
        let charge = budget.held();
        let pointer = value.as_ref().as_bytes().unwrap().as_ptr();
        budget.limit(charge);
        for _ in 0..32 {
            let descendant = ValueRef::to_owned(&value.as_ref());
            drop(value);
            assert_eq!(budget.held(), charge);
            assert_eq!(descendant.as_ref().as_bytes().unwrap().as_ptr(), pointer);
            value = descendant;
        }
        drop(value);
        assert_eq!(budget.held(), 0);
    }

    #[test]
    fn null_empty_and_owner_free_public_roundtrips_remain_compatible() {
        for bytes in [
            None,
            Some(Bytes::new()),
            Some(Bytes::from_static(b"ordinary")),
        ] {
            let mut value = PgValue {
                value: bytes,
                type_info: PgTypeInfo::TEXT,
                format: PgValueFormat::Text,
            };
            let expected = value.value.as_deref().map(<[u8]>::to_vec);
            for _ in 0..8 {
                value = ValueRef::to_owned(&value.as_ref());
                assert_eq!(value.value.as_deref(), expected.as_deref());
                assert_eq!(value.is_null(), expected.is_none());
            }
        }
    }
}
