//! Pure House ACL list identifiers, permissions and editor values.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HouseAccess {
    NotInvited,
    Guest,
    Subowner,
    Owner,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseList {
    Guest,
    Subowner,
    Door(u32),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseEditor {
    pub editor_id: [u8; 16],
    pub house_key: String,
    pub list: HouseList,
    pub ownership_revision: u64,
    pub acl_revision: u64,
    pub allow_everyone: bool,
    pub members: Vec<[u8; 16]>,
    pub text: String,
}
pub fn list_id(list: HouseList) -> Result<i64, &'static str> {
    match list {
        HouseList::Guest => Ok(-1),
        HouseList::Subowner => Ok(-2),
        HouseList::Door(id) if id > 0 => Ok(i64::from(id)),
        _ => Err("zero House door"),
    }
}
pub fn permission(access: HouseAccess, list: HouseList) -> bool {
    access == HouseAccess::Owner || access == HouseAccess::Subowner && list == HouseList::Guest
}
/// The grant from one Guest/Subowner list after its membership facts were read.
pub fn list_access(list_id: i64, allow_everyone: bool, member: bool) -> HouseAccess {
    if allow_everyone || member {
        if list_id == -2 {
            HouseAccess::Subowner
        } else {
            HouseAccess::Guest
        }
    } else {
        HouseAccess::NotInvited
    }
}

/// Bound text before resolving native names in the owning World.
pub fn list_lines(text: &str) -> Result<Vec<&str>, &'static str> {
    if text.len() > 10_000 {
        return Err("House list bound");
    }
    let lines: Vec<_> = text.lines().collect();
    if lines.len() > 100 {
        return Err("House list line bound");
    }
    Ok(lines)
}

pub enum HouseListLine {
    Ignore,
    Everyone,
    Member(oteryn_gameplay_domain::domain::character_name::CharacterName),
}

/// Source comments and `*` are supported; guild/rank lines need an admitted guild owner.
pub fn parse_list_line(line: &str) -> Result<HouseListLine, &'static str> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(HouseListLine::Ignore);
    }
    if line == "*" {
        return Ok(HouseListLine::Everyone);
    }
    if line.contains('@') {
        return Err("House guild authority unavailable");
    }
    if line.len() > 100 || line.contains(['!', '*', '?']) {
        return Ok(HouseListLine::Ignore);
    }
    let name = oteryn_gameplay_domain::domain::character_name::CharacterName::parse(line)
        .map_err(|_| "invalid native House member name")?;
    Ok(HouseListLine::Member(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guest_only_subowner_and_no_zero_door() {
        assert!(permission(HouseAccess::Subowner, HouseList::Guest));
        assert!(!permission(HouseAccess::Subowner, HouseList::Subowner));
        assert!(!permission(HouseAccess::Subowner, HouseList::Door(1)));
        assert!(permission(HouseAccess::Owner, HouseList::Door(1)));
        assert!(list_id(HouseList::Door(0)).is_err());
    }

    #[test]
    fn membership_grants_the_highest_guest_or_subowner_role() {
        for (list, role) in [(-1, HouseAccess::Guest), (-2, HouseAccess::Subowner)] {
            assert_eq!(list_access(list, false, false), HouseAccess::NotInvited);
            assert_eq!(list_access(list, true, false), role);
            assert_eq!(list_access(list, false, true), role);
            assert_eq!(list_access(list, true, true), role);
        }
        assert_eq!(
            HouseAccess::Guest.max(HouseAccess::Subowner),
            HouseAccess::Subowner
        );
    }

    #[test]
    fn list_text_bounds_comments_wildcards_and_native_names_are_preserved() {
        assert_eq!(list_lines(&"x".repeat(10_001)), Err("House list bound"));
        assert_eq!(list_lines(&"\n".repeat(101)), Err("House list line bound"));
        assert!(list_lines(&"x".repeat(10_000)).is_ok());
        assert!(list_lines(&"\n".repeat(100)).is_ok());
        for line in [
            "",
            "  ",
            " # comment",
            "Player!",
            "Player?",
            "Pl*ayer",
            &"x".repeat(101),
        ] {
            assert!(matches!(parse_list_line(line), Ok(HouseListLine::Ignore)));
        }
        assert!(matches!(
            parse_list_line(" * "),
            Ok(HouseListLine::Everyone)
        ));
        assert!(matches!(
            parse_list_line(" Test Player "),
            Ok(HouseListLine::Member(_))
        ));
        assert!(matches!(
            parse_list_line("rank@guild"),
            Err("House guild authority unavailable")
        ));
        assert!(matches!(
            parse_list_line("123"),
            Err("invalid native House member name")
        ));
    }

    /// S05 parity with Crystal `201189f8` `AccessList::parseList` (`src/map/house/house.cpp`
    /// 647-699): `#` comments, empty lines and lines over 100 characters are skipped, a lone `*`
    /// admits everyone, any other line with `!`, `*` or `?` is skipped, and names are members.
    /// Recorded divergences: Crystal resolves `@guild` and `rank@guild` lines, strips characters
    /// outside `[a-zA-Z' *!@#]` and silently drops names over 30 characters; Oteryn refuses
    /// them.
    #[test]
    fn s05_crystal_access_list_lines() {
        let ignored = |line: &str| matches!(parse_list_line(line), Ok(HouseListLine::Ignore));
        for line in [
            "",
            "\t",
            "#comment",
            "  # indented",
            "Bob!",
            "B*b",
            "Bo?",
            "*Bob",
        ] {
            assert!(ignored(line), "{line:?}");
        }
        assert!(ignored(&"a".repeat(101)));
        assert!(matches!(parse_list_line("*"), Ok(HouseListLine::Everyone)));
        assert!(matches!(
            parse_list_line("  Knight Bob  "),
            Ok(HouseListLine::Member(_))
        ));
        for line in ["@guild", "leader@guild", "Bob2", &"a".repeat(31)] {
            assert!(parse_list_line(line).is_err(), "{line:?}");
        }
        // Crystal admits a Subowner to the Guest list only; the Owner edits every list.
        for list in [HouseList::Guest, HouseList::Subowner, HouseList::Door(7)] {
            assert!(permission(HouseAccess::Owner, list));
            assert_eq!(
                permission(HouseAccess::Subowner, list),
                list == HouseList::Guest
            );
            assert!(!permission(HouseAccess::Guest, list));
        }
    }
}
