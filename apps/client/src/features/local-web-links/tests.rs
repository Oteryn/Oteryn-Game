use super::*;
use std::cell::RefCell;

const ALL: [WebLink; 4] = [
    WebLink::SupportTickets,
    WebLink::CookiePolicy,
    WebLink::CoinHistory,
    WebLink::CoinTransfer,
];

#[test]
fn urls_follow_the_platform_paths_in_polish_and_english() {
    let paths = [
        "support/tickets",
        "legal/cookies",
        "shop/history",
        "shop/transfer",
    ];
    for (link, path) in ALL.into_iter().zip(paths) {
        assert_eq!(
            url("https://oteryn.example", link, Language::Pl).as_deref(),
            Some(format!("https://oteryn.example/pl/{path}").as_str())
        );
        assert_eq!(
            url("https://oteryn.example", link, Language::En).as_deref(),
            Some(format!("https://oteryn.example/en/{path}").as_str())
        );
    }
}

#[test]
fn other_client_languages_fall_back_to_english() {
    for language in [Language::De, Language::Es, Language::Pt, Language::Sv] {
        assert_eq!(language_segment(language), "en");
    }
    assert_eq!(language_segment(Language::Pl), "pl");
}

#[test]
fn trailing_slashes_are_trimmed_and_non_http_bases_are_refused() {
    assert_eq!(
        url(
            " http://localhost:8080// ",
            WebLink::CookiePolicy,
            Language::En
        )
        .as_deref(),
        Some("http://localhost:8080/en/legal/cookies")
    );
    for bad in [
        "",
        "oteryn.example",
        "ftp://x",
        "https://",
        "https:///a",
        "https://a b",
    ] {
        assert_eq!(url(bad, WebLink::CoinHistory, Language::En), None, "{bad}");
    }
}

#[test]
fn open_passes_the_built_url_to_the_system_browser_opener() {
    let seen = RefCell::new(Vec::new());
    let opener = |url: &str| {
        seen.borrow_mut().push(url.to_owned());
        true
    };
    assert!(open_with(
        "https://oteryn.example/",
        &opener,
        WebLink::SupportTickets,
        Language::Pl
    ));
    assert_eq!(
        *seen.borrow(),
        ["https://oteryn.example/pl/support/tickets"]
    );
    assert!(!open_with(
        "nope",
        &opener,
        WebLink::SupportTickets,
        Language::Pl
    ));
    assert_eq!(seen.borrow().len(), 1);
    assert!(!open_with(
        "https://a",
        &|_: &str| false,
        WebLink::CoinTransfer,
        Language::En
    ));
}
