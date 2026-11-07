//! How a play session ends: the public class a session error returns to login with.

use oteryn_platform_client::native_login::PublicClass;
use oteryn_session::SessionError;

/// Any session error ends play and returns to login with this public class.
#[must_use]
pub const fn public_class(_error: &SessionError) -> PublicClass {
    PublicClass::SessionUnavailable
}

/// How a play session ended: the server ended it (back to login with this public class), or the
/// player logged out and the server committed it (back to character selection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayEnd {
    Ended(PublicClass),
    LoggedOut,
}
