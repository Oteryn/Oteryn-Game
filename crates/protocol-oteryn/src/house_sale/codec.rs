use super::Error;
pub(super) fn text(out: &mut Vec<u8>, value: &str) -> Result<(), Error> {
    let length = u16::try_from(value.len()).map_err(|_| Error::LimitExceeded)?;
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}
pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(count).ok_or(Error::LimitExceeded)?;
        let value = self.bytes.get(self.at..end).ok_or(Error::Malformed)?;
        self.at = end;
        Ok(value)
    }
    pub(super) fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    pub(super) fn number(&mut self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(
            self.take(8)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    pub(super) fn signed(&mut self) -> Result<i64, Error> {
        i64::try_from(self.number()?).map_err(|_| Error::Malformed)
    }
    pub(super) fn text(&mut self, max: usize) -> Result<String, Error> {
        let size =
            u16::from_be_bytes(self.take(2)?.try_into().map_err(|_| Error::Malformed)?) as usize;
        if size > max {
            return Err(Error::LimitExceeded);
        }
        Ok(std::str::from_utf8(self.take(size)?)
            .map_err(|_| Error::Malformed)?
            .to_owned())
    }
    pub(super) fn finished(&self) -> bool {
        self.at == self.bytes.len()
    }
}
