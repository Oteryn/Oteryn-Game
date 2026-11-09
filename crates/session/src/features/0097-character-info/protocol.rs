//! Client adapter for the existing CHARACTER-INFO-1 byte layout.
//! The server's transport decoder and durability character_title projection own this layout.
pub use oteryn_protocol_oteryn::character_info::*;

pub fn encode_operation(operation: CharacterInfoOperation) -> Vec<u8> {
    let mut bytes = Vec::new();
    let page = match operation {
        CharacterInfoOperation::QueryPremium => {
            bytes.push(5);
            None
        }
        CharacterInfoOperation::QuerySelf(page) => {
            bytes.push(0);
            Some(page)
        }
        CharacterInfoOperation::QueryVisible { character, page } => {
            bytes.push(1);
            bytes.extend_from_slice(&character);
            Some(page)
        }
        CharacterInfoOperation::LookVisible { character } => {
            bytes.push(4);
            bytes.extend_from_slice(&character);
            None
        }
        CharacterInfoOperation::Unlock(title) => {
            bytes.extend_from_slice(&[2, title.id()]);
            None
        }
        CharacterInfoOperation::Select(title) => {
            bytes.extend_from_slice(&[3, title.map_or(0, CharacterTitle::id)]);
            None
        }
    };
    if let Some(page) = page {
        bytes.extend_from_slice(&page.offset.to_le_bytes());
        bytes.push(page.limit);
    }
    bytes
}

/// Decode only the result corresponding to the sent operation; never replace confirmed state
/// with a malformed or unrelated reply. History on this wire is counts, not history entries.
pub fn result_text(operation: CharacterInfoOperation, bytes: &[u8]) -> Option<String> {
    match operation {
        CharacterInfoOperation::Unlock(title) if bytes == [2, title.id()] => {
            Some(format!("Title unlocked: {}", title.name()))
        }
        CharacterInfoOperation::Select(title) if bytes.len() == 9 && bytes[0] == 3 => {
            let revision = u64::from_le_bytes(bytes[1..].try_into().ok()?);
            Some(format!(
                "Selected title: {}; revision {revision}",
                title.map_or("None", CharacterTitle::name)
            ))
        }
        CharacterInfoOperation::QuerySelf(_)
        | CharacterInfoOperation::QueryVisible { .. }
        | CharacterInfoOperation::LookVisible { .. } => {
            if bytes.first() != Some(&0) || bytes.len() < 71 {
                return None;
            }
            let mut values = [0i64; 8];
            for (i, value) in values.iter_mut().enumerate() {
                *value = i64::from_le_bytes(bytes[1 + i * 8..9 + i * 8].try_into().ok()?);
                if *value < 0 {
                    return None;
                }
            }
            let deaths = u16::from_le_bytes(bytes[65..67].try_into().ok()?);
            let kills = u16::from_le_bytes(bytes[67..69].try_into().ok()?);
            let limit = match operation {
                CharacterInfoOperation::QuerySelf(page)
                | CharacterInfoOperation::QueryVisible { page, .. } => page.limit,
                _ => 1,
            };
            let count = usize::from(bytes[69]);
            if deaths > u16::from(limit)
                || kills > u16::from(limit)
                || count > TITLE_COUNT
                || bytes.len() != 71 + count
            {
                return None;
            }
            let mut titles = Vec::new();
            for raw in &bytes[70..70 + count] {
                let title = CharacterTitle::from_id(*raw)?;
                if titles.contains(&title) {
                    return None;
                }
                titles.push(title);
            }
            let selected = bytes[70 + count];
            let selected = if selected == 0 {
                "None"
            } else {
                let title = CharacterTitle::from_id(selected)?;
                if !titles.contains(&title) {
                    return None;
                }
                title.name()
            };
            let names = titles
                .iter()
                .map(|t| t.name())
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!(
                "Level {} | Experience {}\r\nAchievements {} | Appearances {}\r\nInventory {} | Depot {} | Stash {} | Inbox {}\r\nPage deaths {deaths} | PvP kills {kills}\r\nTitles: {names}\r\nSelected: {selected}",
                values[0],
                values[1],
                values[2],
                values[3],
                values[4],
                values[5],
                values[6],
                values[7]
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_character_info_layout_is_strict_and_correlated()
    -> Result<(), Box<dyn std::error::Error>> {
        let page = PageRequest::new(513, 50).ok_or("page")?;
        assert_eq!(
            encode_operation(CharacterInfoOperation::QuerySelf(page)),
            [0, 1, 2, 50]
        );
        let title = CharacterTitle::AdeptArmorer;
        assert_eq!(
            encode_operation(CharacterInfoOperation::Unlock(title)),
            [2, 1]
        );
        assert_eq!(
            encode_operation(CharacterInfoOperation::Select(None)),
            [3, 0]
        );
        assert!(result_text(CharacterInfoOperation::Unlock(title), &[2, 2]).is_none());
        assert!(result_text(CharacterInfoOperation::Select(None), &[3]).is_none());
        let operation = CharacterInfoOperation::QuerySelf(page);
        let mut bytes = vec![0];
        bytes.extend_from_slice(&42i64.to_le_bytes());
        bytes.resize(69, 0);
        bytes.extend_from_slice(&[1, 1, 1]);
        let text = result_text(operation, &bytes).ok_or("reply")?;
        assert!(text.contains("Level 42"));
        assert!(text.contains("Selected: Adept Armorer"));
        for end in 0..bytes.len() {
            assert!(result_text(operation, &bytes[..end]).is_none());
        }
        bytes.push(0);
        assert!(result_text(operation, &bytes).is_none());
        bytes.pop();
        bytes[71] = 255;
        assert!(result_text(operation, &bytes).is_none());
        bytes[71] = 1;
        bytes[65..67].copy_from_slice(&51u16.to_le_bytes());
        assert!(result_text(operation, &bytes).is_none());
        bytes[65..67].copy_from_slice(&0u16.to_le_bytes());
        bytes[1..9].copy_from_slice(&(-1i64).to_le_bytes());
        assert!(result_text(operation, &bytes).is_none());
        Ok(())
    }
}
