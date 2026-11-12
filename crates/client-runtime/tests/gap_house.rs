use oteryn_client_runtime::house::{self, Action};
fn world() -> [u8; 16] {
    let mut w = [1; 16];
    w[6] = 0x70;
    w[8] = 0x80;
    w
}
#[test]
fn gap_house_exact_mutations_and_query() -> Result<(), &'static str> {
    let w = world();
    let mut binding = vec![66, 23, 10, 16];
    binding.extend(w);
    binding.extend([18, 3, b'h', b'o', b'm']);
    for (a, tail) in [
        (Action::Bid(300), vec![10, 3, 8, 172, 2]),
        (Action::Notice(300), vec![18, 3, 8, 172, 2]),
        (Action::Cancel, vec![26, 0]),
        (Action::Guests(vec![]), vec![34, 4, 8, 1, 16, 9]),
        (Action::Subowners(vec![]), vec![34, 4, 8, 2, 16, 9]),
        (
            Action::Door {
                x: 1,
                y: 2,
                floor: -7,
                revision: 8,
                members: vec![],
            },
            vec![42, 10, 10, 6, 8, 1, 16, 2, 24, 13, 16, 8],
        ),
        (Action::Leave, vec![58, 0]),
    ] {
        let cmd = house::mutation(w, "hom", 9, &a)?;
        assert_eq!(cmd.payload, [binding.as_slice(), tail.as_slice()].concat());
        assert_eq!((cmd.capability, cmd.kind), (49, 56));
    }
    let guest = house::mutation(w, "hom", 9, &Action::Guests(vec![w]))?;
    let mut tail = vec![34, 22, 8, 1, 16, 9, 26, 16];
    tail.extend(w);
    assert_eq!(
        guest.payload,
        [binding.as_slice(), tail.as_slice()].concat()
    );
    assert!(house::mutation(w, "hom", 9, &Action::Bid(0)).is_err());
    assert!(house::mutation(w, "hom", 9, &Action::Notice(0)).is_err());
    let kick = house::mutation(w, "hom", 9, &Action::Kick(w))?;
    let mut expected = binding;
    expected.extend([50, 18, 10, 16]);
    expected.extend(w);
    assert_eq!(kick.payload, expected);
    let q = house::query(w, "oteryn:content.area.city.thais", Some("hom"))?;
    assert_eq!((q.capability, q.kind), (49, 57));
    assert!(q.payload.ends_with(&[26, 3, b'h', b'o', b'm']));
    assert!(house::mutation(w, "hom", 9, &Action::Guests(vec![w, w])).is_err());
    Ok(())
}
#[test]
fn gap_house_malformed_truncated_and_bounded_pages() -> Result<(), &'static str> {
    let row = [10, 1, b'a', 16, 1, 24, 10, 32, 7, 40, 1, 56, 20, 80, 9];
    let mut valid = vec![8, 1, 26, row.len() as u8];
    valid.extend(row);
    valid.extend([34, 1, b'a']);
    let p = house::page(&valid)?;
    assert_eq!(p.rows[0].acl_revision, 9);
    assert_eq!(p.cursor.as_deref(), Some("a"));
    for end in 0..valid.len() {
        if end != 2 && end != valid.len() - 3 {
            assert!(house::page(&valid[..end]).is_err(), "prefix {end}");
        }
    }
    for bad in [
        vec![8, 2],
        vec![8, 1, 8, 1],
        vec![8, 1, 26, 0],
        vec![8, 1, 34, 1, b'z'],
        vec![8; 32769],
    ] {
        assert!(house::page(&bad).is_err());
    }
    let mut too_many = vec![8, 1];
    for index in 0..21 {
        if index == 20 {
            assert_eq!(house::page(&too_many)?.rows.len(), 20);
        }
        let mut unique = row;
        unique[2] = b'a' + index;
        too_many.extend([26, unique.len() as u8]);
        too_many.extend(unique);
    }
    assert!(house::page(&too_many).is_err());
    assert!(house::result(&[8, 1, 16]).is_err());
    Ok(())
}

#[test]
fn gap_house_members_and_largest_door_request_stay_bounded() -> Result<(), &'static str> {
    let w = world();
    let members: Vec<_> = (0..200)
        .map(|n| {
            let mut member = w;
            member[0] = n;
            member
        })
        .collect();
    let command = house::mutation(
        w,
        &"h".repeat(512),
        u64::MAX,
        &Action::Door {
            x: u16::MAX,
            y: u16::MAX,
            floor: -15,
            revision: u64::MAX,
            members,
        },
    )?;
    assert!(command.payload.len() <= 4168);
    assert!(house::mutation(w, "hom", 9, &Action::Guests(vec![w; 201])).is_err());
    assert!(house::query(w, "thais", None).is_err());
    assert!(house::query(w, "oteryn:content.area.city.thais", Some(&"a".repeat(513))).is_err());
    Ok(())
}

#[test]
fn house_metadata_refuses_partial_invalid_floor_and_duplicate_fields() -> Result<(), &'static str> {
    let base = [10, 1, b'a', 16, 1, 24, 10, 32, 7, 40, 1, 56, 20, 80, 9];
    let details = [98, 1, b'A', 104, 1, 112, 0, 120, 1, 128, 1, 2, 136, 1, 13];
    let page = |extra: &[u8]| {
        let row = [base.as_slice(), extra].concat();
        let mut body = vec![8, 1, 26, row.len() as u8];
        body.extend(row);
        body
    };
    assert_eq!(
        house::page(&page(&details))?.rows[0].entrance,
        Some((1, 2, -7))
    );
    for end in 1..details.len() {
        assert!(house::page(&page(&details[..end])).is_err());
    }
    for floor in [2, 30, 31] {
        let mut bad = details;
        bad[13] = floor;
        assert!(house::page(&page(&bad)).is_err());
    }
    assert!(house::page(&page(&[details.as_slice(), &[112, 0]].concat())).is_err());
    Ok(())
}
