//! Container sort command 168. The payload is four canonical bytes: view, key,
//! descending, recursive. Both flags are exactly 0 or 1; keys are 1..=4.
pub const CAPABILITY_CONTAINER_SORT_V1: u32 = 146;
pub const COMMAND_TYPE_CONTAINER_SORT: u32 = 168;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SortKey {
    Name = 1,
    Weight = 2,
    Expiry = 3,
    StackSize = 4,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub view: u8,
    pub key: SortKey,
    pub descending: bool,
    pub recursive: bool,
}
impl Request {
    pub fn encode(self) -> [u8; 4] {
        [
            self.view,
            self.key as u8,
            u8::from(self.descending),
            u8::from(self.recursive),
        ]
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let [view @ 0..=15, key, descending @ 0..=1, recursive @ 0..=1] = *bytes else {
            return None;
        };
        Some(Self {
            view,
            key: match key {
                1 => SortKey::Name,
                2 => SortKey::Weight,
                3 => SortKey::Expiry,
                4 => SortKey::StackSize,
                _ => return None,
            },
            descending: descending == 1,
            recursive: recursive == 1,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn container_sort_canonical_wire() {
        for key in [
            SortKey::Name,
            SortKey::Weight,
            SortKey::Expiry,
            SortKey::StackSize,
        ] {
            for descending in [false, true] {
                for recursive in [false, true] {
                    let request = Request {
                        view: 15,
                        key,
                        descending,
                        recursive,
                    };
                    assert_eq!(Request::decode(&request.encode()), Some(request));
                }
            }
        }
        for bytes in [
            &[][..],
            &[0, 1, 0][..],
            &[16, 1, 0, 0][..],
            &[0, 0, 0, 0][..],
            &[0, 1, 2, 0][..],
            &[0, 1, 0, 2][..],
            &[0, 1, 0, 0, 0][..],
        ] {
            assert_eq!(Request::decode(bytes), None);
        }
    }
}
