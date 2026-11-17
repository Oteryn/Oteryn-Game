// Immutable qualification content is prepared before admission, never between frames.
use crate::content::native_gameplay::NativeGameplayInput;
use std::sync::OnceLock;

pub(super) fn native_input() -> TestResult<&'static NativeGameplayInput> {
    static INPUT: OnceLock<Result<NativeGameplayInput, String>> = OnceLock::new();
    INPUT
        .get_or_init(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| error.clone().into())
}

pub(super) fn native_spell_book() -> TestResult<&'static crate::spell::SpellBook> {
    static BOOK: OnceLock<Result<crate::spell::SpellBook, String>> = OnceLock::new();
    BOOK.get_or_init(|| {
        let compile = || -> TestResult<crate::spell::SpellBook> {
            let input = native_input()?;
            let catalog = crate::spell::executable_catalog::compile(
                &input.catalog.bytes,
                &input.catalog.sha256,
            )?;
            let selections = crate::spell::executable_catalog::compile_source_selection(
                &input.source_selection.bytes,
                &input.source_selection.sha256,
                &catalog,
            )?;
            Ok(catalog.into_spell_book_with_selections(&selections)?)
        };
        compile().map_err(|error| error.to_string())
    })
    .as_ref()
    .map_err(|error| error.clone().into())
}

#[cfg(test)]
mod native_content_tests {
    use super::*;

    #[test]
    fn qualification_native_content_reuses_prewarmed_instances() -> TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let input = native_input().map_err(|error| error.to_string())?;
                let book = native_spell_book().map_err(|error| error.to_string())?;
                assert!(std::ptr::eq(
                    input,
                    native_input().map_err(|error| error.to_string())?
                ));
                assert!(std::ptr::eq(
                    book,
                    native_spell_book().map_err(|error| error.to_string())?
                ));
                assert!(book.spoken("utani hur").is_some());
                Ok(())
            })?
            .join()
            .map_err(|_| "qualification content worker panicked")??;
        Ok(())
    }
}
