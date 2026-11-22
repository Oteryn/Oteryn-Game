//! Browser links to the Platform web pages the in-game menus point at: support tickets,
//! cookie policy, coin history and coin transfer. The Platform base URL is the configured
//! native login one; the page language is the client language with `en` as the fallback.
use crate::NativeLoginConfig;
use oteryn_client_ui::localization::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebLink {
    SupportTickets,
    CookiePolicy,
    CoinHistory,
    CoinTransfer,
}
impl WebLink {
    const fn path(self) -> &'static str {
        match self {
            Self::SupportTickets => "support/tickets",
            Self::CookiePolicy => "legal/cookies",
            Self::CoinHistory => "shop/history",
            Self::CoinTransfer => "shop/transfer",
        }
    }
}

/// The Platform web languages are `pl` and `en`; every other client language reads `en`.
#[must_use]
pub const fn language_segment(language: Language) -> &'static str {
    match language {
        Language::Pl => "pl",
        _ => "en",
    }
}

/// `{platform}/{lang}/{path}`, or `None` when the base is not an http(s) URL with a host.
#[must_use]
pub fn url(platform_url: &str, link: WebLink, language: Language) -> Option<String> {
    destination_url(
        platform_url,
        &format!("{}/{}", language_segment(language), link.path()),
    )
}

pub(crate) fn destination_url(platform_url: &str, path: &str) -> Option<String> {
    let raw = platform_url.trim();
    let host = raw
        .strip_prefix("https://")
        .or_else(|| raw.strip_prefix("http://"))?;
    if host.is_empty() || host.starts_with('/') || host.contains(char::is_whitespace) {
        return None;
    }
    let mut base = url::Url::parse(raw).ok()?;
    base.host_str()?;
    base.set_query(None);
    base.set_fragment(None);
    base.set_path(&format!("{}/", base.path().trim_end_matches('/')));
    base.join(path).ok().map(Into::into)
}

/// Opens `link` in the system browser; `false` when the URL is invalid or the opener failed.
pub fn open_with(
    platform_url: &str,
    open_browser: &dyn Fn(&str) -> bool,
    link: WebLink,
    language: Language,
) -> bool {
    url(platform_url, link, language).is_some_and(|url| open_browser(&url))
}

/// Opens `link` with the configured opener in the current client language.
pub fn open(config: &NativeLoginConfig, link: WebLink) -> bool {
    open_with(
        &config.platform_url,
        &*config.open_browser,
        link,
        oteryn_client_ui::localization::current(),
    )
}

#[cfg(test)]
mod tests;
