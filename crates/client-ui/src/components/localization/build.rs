//! Compile locale templates into typed Rust formatting branches; all arguments stay at the callsite.
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    path::Path,
};

pub fn generate(root: &Path, out: &Path) -> Result<(), String> {
    let folder = root.join("src/components/localization/locales");
    let mut catalogs = Vec::new();
    for language in ["en", "de", "es", "pl", "pt", "sv"] {
        let path = folder.join(format!("{language}.json"));
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        catalogs.push(
            serde_json::from_slice::<BTreeMap<String, String>>(&bytes)
                .map_err(|e| e.to_string())?,
        );
    }
    let keys: BTreeSet<_> = catalogs[0].keys().collect();
    for catalog in &catalogs {
        if catalog.keys().collect::<BTreeSet<_>>() != keys
            || catalog.values().any(|text| text.trim().is_empty())
        {
            return Err(
                "Every supported language must translate the complete source catalog".into(),
            );
        }
    }
    let mut rust = String::from(
        "fn catalog_text(language: Language, source: &str) -> Option<&'static str> { match source {\n",
    );
    for source in catalogs[0].keys() {
        let targets: Vec<_> = catalogs
            .iter()
            .map(|catalog| catalog[source].as_str())
            .collect();
        writeln!(rust, "{source:?} => Some({targets:?}[language as usize]),")
            .map_err(|e| e.to_string())?;
    }
    rust.push_str("_ => None } }\n");
    rust.push_str("#[macro_export] macro_rules! tr {\n");
    for source in catalogs[0].keys() {
        writeln!(
            rust,
            "({source:?}) => {{ $crate::localization::tr({source:?}) }};"
        )
        .map_err(|e| e.to_string())?;
    }
    rust.push_str("($source:literal) => { compile_error!(\"Missing client text in the six-language catalog\") }; }\n");
    let formats_path = folder.join("formats.json");
    println!("cargo:rerun-if-changed={}", formats_path.display());
    let formats: BTreeSet<String> =
        serde_json::from_slice(&std::fs::read(formats_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    for source in &formats {
        let expected = fields(source)?;
        for catalog in &catalogs {
            let target = catalog
                .get(source)
                .ok_or_else(|| format!("Missing format: {source}"))?;
            if fields(target)? != expected {
                return Err(format!(
                    "Translation changed formatting arguments: {source}"
                ));
            }
        }
    }
    for (name, original, destination) in [
        ("lfmt", "format", false),
        ("lwrite", "write", true),
        ("lwriteln", "writeln", true),
        ("lprintln", "println", false),
        ("leprintln", "eprintln", false),
    ] {
        writeln!(rust, "#[macro_export] macro_rules! {name} {{").map_err(|e| e.to_string())?;
        for source in &formats {
            if !catalogs[0].contains_key(source) {
                return Err(format!("Missing format source: {source}"));
            }
            for arguments in [false, true] {
                let prefix = if destination { "$dst:expr," } else { "" };
                let suffix = if arguments { ", $($args:tt)*" } else { "$(,)?" };
                write!(
                    rust,
                    "({prefix}{source:?}{suffix}) => {{ match $crate::localization::current() {{"
                )
                .map_err(|e| e.to_string())?;
                for (index, language) in
                    ["En", "De", "Es", "Pl", "Pt", "Sv"].into_iter().enumerate()
                {
                    let target = &catalogs[index][source];
                    let dst = if destination { "$dst," } else { "" };
                    let args = if arguments { ",$($args)*" } else { "" };
                    write!(rust, "$crate::localization::Language::{language} => ::std::{original}!({dst}{target:?}{args}),").map_err(|e| e.to_string())?;
                }
                rust.push_str("} };\n");
            }
        }
        writeln!(
            rust,
            "($($args:tt)*) => {{ compile_error!(\"Missing client format in the six-language catalog\") }}; }}"
        )
        .map_err(|e| e.to_string())?;
    }
    std::fs::write(out.join("localized_messages.rs"), rust).map_err(|e| e.to_string())
}

fn fields(text: &str) -> Result<Vec<&str>, String> {
    let bytes = text.as_bytes();
    let mut fields = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'{' if bytes.get(index + 1) == Some(&b'{') => index += 2,
            b'}' if bytes.get(index + 1) == Some(&b'}') => index += 2,
            b'{' => {
                let end = bytes[index + 1..]
                    .iter()
                    .position(|b| *b == b'}')
                    .map(|offset| index + 1 + offset)
                    .ok_or_else(|| format!("Invalid format: {text}"))?;
                fields.push(&text[index..=end]);
                index = end + 1;
            }
            b'}' => return Err(format!("Invalid format: {text}")),
            _ => index += 1,
        }
    }
    Ok(fields)
}
