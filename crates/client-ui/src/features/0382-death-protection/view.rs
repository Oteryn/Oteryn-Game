use super::DeathProtection;
pub fn notice(value: &DeathProtection) -> String {
    let names: Vec<String> = value
        .consumed
        .iter()
        .map(|key| {
            key.strip_prefix("oteryn:blessing.")
                .unwrap_or(key)
                .split('_')
                .map(|word| {
                    let mut chars = word.chars();
                    chars.next().map_or_else(String::new, |first| {
                        first.to_uppercase().chain(chars).collect()
                    })
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    lfmt!(
        "Death — consumed: {}",
        if names.is_empty() {
            tr!("No blessings or Twist of Fate").into()
        } else {
            names.join(", ")
        }
    )
}
