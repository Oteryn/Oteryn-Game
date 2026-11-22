use oteryn_client_ui::localization::{Language, translate, with_language};

#[test]
fn movement_spell_words_are_identical_in_every_locale() {
    use oteryn_client_ui::chat_public_channels::ChatPanel;
    use oteryn_client_ui::spell_action_bar::movement::{MovementSpell, request};
    for language in Language::ALL {
        with_language(language, || {
            for (spell, words) in [
                (MovementSpell::LevitateUp, "exani hur \"up"),
                (MovementSpell::LevitateDown, "exani hur \"down"),
                (MovementSpell::MagicRope, "exani tera"),
            ] {
                let mut panel = ChatPanel {
                    speech_selected: true,
                    ..Default::default()
                };
                assert_eq!(request(&mut panel, spell), Some(words), "{language:?}");
                assert!(panel.pending);
            }
        });
    }
}

#[test]
fn logout_messages_describe_logging_out() {
    assert_eq!(
        translate(Language::De, "You cannot logout now, try again"),
        "Du kannst dich jetzt nicht abmelden. Versuche es erneut."
    );
    assert_eq!(
        translate(Language::Es, "You cannot logout now, try again"),
        "No puedes cerrar sesión ahora. Inténtalo de nuevo."
    );
    assert_eq!(
        translate(Language::Pl, "You cannot logout now, try again"),
        "Nie możesz się teraz wylogować. Spróbuj ponownie."
    );
    assert_eq!(
        translate(Language::Pt, "You cannot logout now, try again"),
        "Você não pode sair agora. Tente novamente."
    );
    assert_eq!(
        translate(Language::Sv, "You cannot logout now, try again"),
        "Du kan inte logga ut nu. Försök igen."
    );
    assert_eq!(
        translate(Language::De, "You may not logout during a fight ({} s)"),
        "Du kannst dich während eines Kampfes nicht abmelden ({} s)"
    );
    assert_eq!(
        translate(Language::Es, "You may not logout during a fight ({} s)"),
        "No puedes cerrar sesión durante un combate ({} s)"
    );
    assert_eq!(
        translate(Language::Pl, "You may not logout during a fight ({} s)"),
        "Nie możesz się wylogować podczas walki ({} s)"
    );
    assert_eq!(
        translate(Language::Pt, "You may not logout during a fight ({} s)"),
        "Você não pode sair durante uma luta ({} s)"
    );
    assert_eq!(
        translate(Language::Sv, "You may not logout during a fight ({} s)"),
        "Du kan inte logga ut under en strid ({} s)"
    );
    assert_eq!(
        translate(
            Language::Pl,
            "Last server announcement: server save in {minutes} minute{}. Please find a safe place to log out."
        ),
        "Ostatni komunikat serwera: zapis serwera za {minutes} minut{}. Znajdź bezpieczne miejsce, aby się wylogować."
    );
    assert_eq!(
        translate(
            Language::Pl,
            "Recommendations\r\n\r\nBefore exploring: check equipment, carrying capacity and healing supplies.\r\nBefore a hunt: choose a suitable target, check your route back and keep reserve supplies.\r\nDuring a hunt: monitor health, mana and conditions; stop and recover before continuing if resources are low.\r\nBefore server-save: read the latest announcement in Notices and find a safe place to log out.\r\nAfter connection loss: return to sign-in and complete sign-in in your browser again.\r\nIf something looks wrong: record reproduction steps and review a bug report before sending it."
        ),
        "Zalecenia\r\n\r\nPrzed odkryciem: sprawdź sprzęt, przewoźnik i dostawy leczenia.\r\nPrzed polowaniem: wybierz odpowiedni cel, sprawdź swoją trasę i zachowaj rezerwę dostaw.\r\nPodczas polowania: monitorowanie zdrowia, stanu i warunków; zatrzymanie i odzyskiwanie przed kontynuacją, jeśli zasoby są niskie.\r\nPrzed przechowywaniem serwera: przeczytaj najnowsze ogłoszenie w Notatki i znajdź bezpieczne miejsce do wylogowania się.\r\nPo utracie połączenia: powrót do rejestracji i zakończenie rejestracji w przeglądarce ponownie.\r\nJeśli coś wydaje się nie tak: zapisuj kroki odtwarzania i przeglądaj raport błędu przed jego wysłaniem."
    );
    assert_eq!(
        translate(
            Language::Pl,
            "The server refused: leaving is blocked during logout"
        ),
        "Serwer odmówił: opuszczenie jest zablokowane podczas wylogowywania"
    );
    assert_eq!(
        translate(
            Language::Sv,
            "The server refused: leaving is blocked during logout"
        ),
        "Servern nekade: det går inte att lämna under utloggning"
    );
}

#[test]
fn validation_messages_preserve_upper_bounds() {
    assert_eq!(
        translate(
            Language::Es,
            "Enter a display content key of at most 160 bytes"
        ),
        "Introduce una clave de contenido de hasta 160 bytes"
    );
    assert_eq!(
        translate(Language::De, "At most 32 presets"),
        "Höchstens 32 Voreinstellungen"
    );
    assert_eq!(
        translate(Language::Es, "At most 32 presets"),
        "Como máximo 32 preajustes"
    );
    assert_eq!(
        translate(Language::Pt, "At most 32 presets"),
        "No máximo 32 predefinições"
    );
    assert_eq!(
        translate(Language::Es, "At most ten quests can be tracked"),
        "Se pueden seguir como máximo diez misiones"
    );
    assert_eq!(
        translate(Language::Pl, "At most ten quests can be tracked"),
        "Można śledzić najwyżej dziesięć zadań"
    );
    assert_eq!(
        translate(Language::De, "Party summary supports at most 50 members"),
        "Die Gruppenzusammenfassung unterstützt höchstens 50 Mitglieder"
    );
    assert_eq!(
        translate(Language::Es, "Party summary supports at most 50 members"),
        "El resumen del grupo admite como máximo 50 miembros"
    );
    assert_eq!(
        translate(Language::Pl, "Party summary supports at most 50 members"),
        "Podsumowanie drużyny obsługuje najwyżej 50 członków"
    );
    assert_eq!(
        translate(Language::Pt, "Party summary supports at most 50 members"),
        "O resumo do grupo permite no máximo 50 membros"
    );
}

#[test]
fn table_headers_preserve_numeric_columns() {
    assert_eq!(
        translate(
            Language::De,
            "Imported party session · advisory\r\nVerify the supplied names and values before using these suggestions.\r\nName | loot value | supply cost | balance\r\n"
        ),
        "Importierter Parteitag · Beratung\r\nÜberprüfen Sie die angebotenen Namen und Werte, bevor Sie diese Vorschläge verwenden.\r\nName | Beutewert | Verbrauchskosten | Bilanz\r\n"
    );
    assert_eq!(
        translate(
            Language::Es,
            "Imported party session · advisory\r\nVerify the supplied names and values before using these suggestions.\r\nName | loot value | supply cost | balance\r\n"
        ),
        "Sesión del partido de importación · asesoramiento\r\nVerifique los nombres y valores suministrados antes de utilizar estas sugerencias.\r\nNombre | valor del botín | coste de suministros | saldo\r\n"
    );
    assert_eq!(
        translate(
            Language::Pl,
            "Imported party session · advisory\r\nVerify the supplied names and values before using these suggestions.\r\nName | loot value | supply cost | balance\r\n"
        ),
        "Zgromadzenie imprezowe · Poradnik\r\nSprawdź podawane nazwy i wartości przed użyciem tych sugestii.\r\nImię | wartość łupów | koszt zapasów | saldo\r\n"
    );
    assert_eq!(
        translate(
            Language::Pt,
            "Imported party session · advisory\r\nVerify the supplied names and values before using these suggestions.\r\nName | loot value | supply cost | balance\r\n"
        ),
        "Sessão do Partido Importado · Consultoria\r\nVerifique os nomes e valores fornecidos antes de usar essas sugestões.\r\nNome | valor do saque | custo dos suprimentos | saldo\r\n"
    );
    assert_eq!(
        translate(
            Language::Sv,
            "Imported party session · advisory\r\nVerify the supplied names and values before using these suggestions.\r\nName | loot value | supply cost | balance\r\n"
        ),
        "Importerad parti session · rådgivande\r\nKontrollera de levererade namn och värden innan du använder dessa förslag.\r\nNamn | bytesvärde | förbrukningskostnad | saldo\r\n"
    );
    assert_eq!(
        translate(
            Language::De,
            "\nPage {} — snapshot {} (Unix milliseconds)\nRank | Name | Vocation | Level | Value"
        ),
        "\nSeite {} - Snapshot {} (Unix Millisekunden)\nRang | Name | Beruf | Stufe | Wert"
    );
    assert_eq!(
        translate(
            Language::Es,
            "\nPage {} — snapshot {} (Unix milliseconds)\nRank | Name | Vocation | Level | Value"
        ),
        "\nPágina {} Síntomas {} (en un millisecondes)\nPuesto | Nombre | Vocación | Nivel | Valor"
    );
    assert_eq!(
        translate(
            Language::Pl,
            "\nPage {} — snapshot {} (Unix milliseconds)\nRank | Name | Vocation | Level | Value"
        ),
        "\nStrona {} - snapshot {} (w odniesieniu do 1 milisekund)\nPozycja | Imię | Profesja | Poziom | Wartość"
    );
    assert_eq!(
        translate(
            Language::Pt,
            "\nPage {} — snapshot {} (Unix milliseconds)\nRank | Name | Vocation | Level | Value"
        ),
        "\nPágina {} - Síntese {} (Unix milisecundos)\nPosição | Nome | Vocação | Nível | Valor"
    );
    assert_eq!(
        translate(
            Language::Sv,
            "\nPage {} — snapshot {} (Unix milliseconds)\nRank | Name | Vocation | Level | Value"
        ),
        "\nsidan {} - Snabbbild {} (Unix milisekunder)\nPlacering | Namn | Yrke | Nivå | Värde"
    );
}

#[test]
fn imported_summary_renders_localized_columns_over_real_values() -> Result<(), &'static str> {
    use oteryn_client_ui::analyser::party_import::Summary;
    let summary =
        Summary::parse("OTERYN-PARTY-SUMMARY\t1\nName\tLoot\tSupplies\nAlice\t100\t40\n")?;
    for (language, header) in [
        (Language::De, "Name | Beutewert | Verbrauchskosten | Bilanz"),
        (
            Language::Es,
            "Nombre | valor del botín | coste de suministros | saldo",
        ),
        (Language::Pl, "Imię | wartość łupów | koszt zapasów | saldo"),
        (
            Language::Pt,
            "Nome | valor do saque | custo dos suprimentos | saldo",
        ),
        (
            Language::Sv,
            "Namn | bytesvärde | förbrukningskostnad | saldo",
        ),
    ] {
        let rendered = with_language(language, || summary.text());
        assert!(
            rendered.lines().any(|line| line == header),
            "{language:?}: {rendered}"
        );
        assert!(rendered.contains("Alice | 100 | 40 | 60"));
    }
    Ok(())
}

#[test]
fn portuguese_pvp_modes_preserve_zero_through_three() {
    assert_eq!(
        translate(
            Language::Pt,
            "Expert: 0 dove, 1 white hand, 2 yellow hand, 3 red fist"
        ),
        "Especialista: 0 pomba, 1 mão branca, 2 mão amarela, 3 punho vermelho"
    );
}

#[test]
fn diagnostic_consent_preserves_literal_yes() {
    assert_eq!(
        translate(
            Language::Pt,
            "Optional build details: enter YES to include build, OS and architecture"
        ),
        "Detalhes opcionais da compilação: digite YES para incluir compilação, sistema operacional e arquitetura"
    );
    assert_eq!(
        translate(
            Language::Es,
            "Category: {}. {guidance} Map reports need x,y,floor; other categories omit the position. Build details are attached only if you enter YES."
        ),
        "Categoría : {}. {guidance} Los informes de mapa necesitan x, y, suelo; otras categorías omitan la posición. Los detalles de compilación se adjuntan solo si introduces YES."
    );
    assert_eq!(
        translate(
            Language::Pl,
            "Category: {}. {guidance} Map reports need x,y,floor; other categories omit the position. Build details are attached only if you enter YES."
        ),
        "w kategorii: {}. {guidance} Raporty mapy wymagają x, y, podłogi; inne kategorie pomijają pozycję. Szczegóły kompilacji są dołączane tylko po wpisaniu YES."
    );
    assert_eq!(
        translate(
            Language::Sv,
            "Category: {}. {guidance} Map reports need x,y,floor; other categories omit the position. Build details are attached only if you enter YES."
        ),
        "Kategorierna : {}. {guidance} Kartrapporter kräver x,y,floor; andra kategorier missar positionen. Bygginformation bifogas endast om du skriver YES."
    );
}

#[test]
fn portuguese_previous_controls_describe_backward_selection() {
    assert_eq!(translate(Language::Pt, "Previous charm"), "Charme anterior");
    assert_eq!(
        translate(Language::Pt, "Previous eligible race"),
        "Raça elegível anterior"
    );
}

#[test]
fn polish_active_session_guidance_requests_logout_before_login() {
    assert_eq!(
        translate(
            Language::Pl,
            "Character is already active. Log out its other session or wait for it to end, then sign in again."
        ),
        "Postać jest już aktywna. Wyloguj ją z innej sesji lub poczekaj na jej zakończenie, a następnie zaloguj się ponownie."
    );
}
