use super::*;
fn text(out: &mut Vec<u8>, value: &str) -> WireResult<()> {
    let length = u16::try_from(value.len()).map_err(|_| "modal text too large")?;
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}
pub(super) fn encode(reply: &Reply) -> WireResult<Vec<u8>> {
    let mut out = Vec::new();
    match reply {
        Reply::None => out.push(0),
        Reply::Answered(answer) | Reply::AnsweredAndLoggedOut(answer) => {
            out.push(if matches!(reply, Reply::AnsweredAndLoggedOut(_)) {
                3
            } else {
                2
            });
            out.extend(answer.encode()?);
        }
        Reply::Dialog(dialog) => {
            if !dialog.valid() {
                return Err("invalid modal dialog");
            }
            out.push(1);
            out.extend_from_slice(&dialog.id.to_le_bytes());
            text(&mut out, &dialog.title)?;
            text(&mut out, &dialog.message)?;
            for items in [&dialog.choices, &dialog.buttons] {
                out.push(items.len() as u8);
                for (id, label) in items {
                    out.push(*id);
                    text(&mut out, label)?;
                }
            }
            out.push(u8::from(dialog.cancel_button.is_some()));
            if let Some(button) = dialog.cancel_button {
                out.push(button);
            }
            out.push(u8::from(dialog.image.is_some()));
            if let Some(image) = &dialog.image {
                out.extend_from_slice(&image.width.to_le_bytes());
                out.extend_from_slice(&image.height.to_le_bytes());
                out.extend_from_slice(&image.rgba);
            }
        }
    }
    if out.len() > MAX_REPLY_BYTES {
        return Err("modal reply exceeds its bound");
    }
    Ok(out)
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> WireResult<&'a [u8]> {
        let value = self.0.get(..count).ok_or("truncated modal reply")?;
        self.0 = &self.0[count..];
        Ok(value)
    }
    fn byte(&mut self) -> WireResult<u8> {
        Ok(self.take(1)?[0])
    }
    fn short(&mut self) -> WireResult<u16> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(|_| "modal short")?,
        ))
    }
    fn text(&mut self, maximum: usize) -> WireResult<String> {
        let length = usize::from(self.short()?);
        if length > maximum {
            return Err("modal text exceeds its bound");
        }
        let text = std::str::from_utf8(self.take(length)?).map_err(|_| "invalid modal UTF-8")?;
        if !valid_text(text, maximum) {
            return Err("invalid modal text");
        }
        Ok(text.into())
    }
    fn items(&mut self, maximum: usize) -> WireResult<Vec<(u8, String)>> {
        let count = usize::from(self.byte()?);
        if count > maximum {
            return Err("too many modal controls");
        }
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            let id = self.byte()?;
            items.push((id, self.text(256)?));
        }
        Ok(items)
    }
}
pub(super) fn decode(bytes: &[u8]) -> WireResult<Reply> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err("modal reply exceeds its bound");
    }
    let mut reader = Reader(bytes);
    let reply = match reader.byte()? {
        0 => Reply::None,
        tag @ (2 | 3) => {
            let answer = Answer::decode(reader.0)?;
            reader.0 = &[];
            if tag == 3 {
                Reply::AnsweredAndLoggedOut(answer)
            } else {
                Reply::Answered(answer)
            }
        }
        1 => {
            let id = u32::from_le_bytes(reader.take(4)?.try_into().map_err(|_| "modal id")?);
            let title = reader.text(256)?;
            let message = reader.text(8192)?;
            let choices = reader.items(MAX_CHOICES)?;
            let buttons = reader.items(MAX_BUTTONS)?;
            let cancel_button = match reader.byte()? {
                0 => None,
                1 => Some(reader.byte()?),
                _ => return Err("invalid modal cancel flag"),
            };
            let image = match reader.byte()? {
                0 => None,
                1 => {
                    let width = reader.short()?;
                    let height = reader.short()?;
                    if !(1..=MAX_IMAGE_EDGE).contains(&width)
                        || !(1..=MAX_IMAGE_EDGE).contains(&height)
                    {
                        return Err("invalid modal image dimensions");
                    }
                    let size = usize::from(width) * usize::from(height) * 4;
                    Some(Image {
                        width,
                        height,
                        rgba: reader.take(size)?.to_vec(),
                    })
                }
                _ => return Err("invalid modal image flag"),
            };
            let dialog = Dialog {
                id,
                title,
                message,
                choices,
                buttons,
                cancel_button,
                image,
            };
            if !dialog.valid() {
                return Err("invalid modal dialog");
            }
            Reply::Dialog(dialog)
        }
        _ => return Err("unknown modal reply"),
    };
    if !reader.0.is_empty() {
        return Err("trailing modal bytes");
    }
    Ok(reply)
}
