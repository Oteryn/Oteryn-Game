use super::*;

fn entry(id: u8, name: &str, weight: u64, quantity: u32, expiry: Option<i64>, bag: bool) -> Entry {
    Entry {
        id: [id; 16],
        parent: [0; 16],
        ordinal: u64::from(id),
        nested: false,
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: name.into(),
            revision_ref: "r1".into(),
        },
        quantity,
        bag,
        name: name.into(),
        weight,
        expiry,
    }
}
#[test]
fn container_sort_each_key_both_directions_bags_first_and_ties() {
    let entries = [
        entry(3, "c", 30, 3, None, false),
        entry(1, "a", 10, 1, Some(10), false),
        entry(2, "b", 20, 2, Some(20), false),
        entry(9, "z", 99, 99, None, true),
    ];
    for key in [
        SortKey::Name,
        SortKey::Weight,
        SortKey::Expiry,
        SortKey::StackSize,
    ] {
        for descending in [false, true] {
            let request = Request {
                view: 0,
                key,
                descending,
                recursive: false,
            };
            let mut sorted: Vec<_> = entries.iter().collect();
            sorted.sort_by(|a, b| compare(a, b, request));
            let ids: Vec<_> = sorted.iter().map(|e| e.id[0]).collect();
            assert_eq!(
                ids,
                if descending {
                    vec![9, 3, 2, 1]
                } else {
                    vec![9, 1, 2, 3]
                },
                "{key:?} {descending}"
            );
        }
    }
    let a = entry(1, "same", 1, 1, None, false);
    let b = entry(2, "same", 1, 1, None, false);
    let c = entry(3, "z", 1, 1, None, false);
    for descending in [false, true] {
        assert!(
            compare(
                &a,
                &b,
                Request {
                    view: 0,
                    key: SortKey::Name,
                    descending,
                    recursive: false
                }
            )
            .is_lt()
        );
        for key in [SortKey::Weight, SortKey::Expiry, SortKey::StackSize] {
            let r = Request {
                view: 0,
                key,
                descending,
                recursive: false,
            };
            assert!(compare(&a, &b, r).is_lt());
            assert!(compare(&b, &c, r).is_lt());
        }
    }
}
#[test]
fn container_sort_recursive_permutations_conserve_every_parent_and_ordinal()
-> Result<(), &'static str> {
    let mut entries = vec![
        entry(1, "z", 9, 9, Some(90), false),
        entry(2, "a", 1, 1, Some(10), true),
        entry(3, "z", 9, 9, Some(90), false),
        entry(4, "a", 1, 1, Some(10), false),
    ];
    entries[2].parent = [2; 16];
    entries[2].nested = true;
    entries[3].parent = [2; 16];
    entries[3].nested = true;
    for key in [
        SortKey::Name,
        SortKey::Weight,
        SortKey::Expiry,
        SortKey::StackSize,
    ] {
        for descending in [false, true] {
            for recursive in [false, true] {
                let r = Request {
                    view: 0,
                    key,
                    descending,
                    recursive,
                };
                let p = permutation(&entries, [0; 16], r);
                let rows = p.as_array().ok_or("array")?;
                assert_eq!(rows.len(), if recursive { 4 } else { 2 });
                for parent in [[0; 16], [2; 16]] {
                    let selected: Vec<_> = rows
                        .iter()
                        .filter(|x| x["parent"] == uuid_string(parent))
                        .collect();
                    let mut before: Vec<_> =
                        selected.iter().map(|x| x["before"].to_string()).collect();
                    let mut after: Vec<_> =
                        selected.iter().map(|x| x["after"].to_string()).collect();
                    before.sort();
                    after.sort();
                    assert_eq!(before, after);
                }
                assert_eq!(rows[0]["item"], uuid_string([2; 16]));
                if recursive {
                    assert_eq!(
                        rows[2]["item"],
                        uuid_string([if descending { 3 } else { 4 }; 16])
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn slotted_move_reads_newest_numeric_ordinal_first() -> Result<(), Box<dyn std::error::Error>> {
    use sqlx::{Connection, PgConnection};
    let Ok(url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    const FIXTURE: &str = "CREATE TEMP TABLE game_item_container_entries (parent_item_instance_id uuid, item_instance_id uuid, placed_transaction_id uuid, placement_ordinal numeric);
        CREATE TEMP TABLE game_item_nested_entries (LIKE game_item_container_entries);
        INSERT INTO game_item_container_entries(parent_item_instance_id,placement_ordinal) VALUES ('00000000-0000-0000-0000-000000000000',9);
        INSERT INTO game_item_nested_entries(parent_item_instance_id,placement_ordinal) VALUES ('00000000-0000-0000-0000-000000000000',10)";
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut conn = PgConnection::connect(&url).await?;
            sqlx::raw_sql(FIXTURE).execute(&mut conn).await?;
            let rows = sqlx::query(SLOT_ENTRIES_SQL)
                .bind([0_u8; 16].as_slice())
                .bind(2_i64)
                .fetch_all(&mut conn)
                .await?;
            let ordinals = rows
                .iter()
                .map(|row| row.try_get::<String, _>("placement_ordinal"))
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(ordinals, ["10", "9"]);
            Ok(())
        })
}
