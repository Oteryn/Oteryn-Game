// Launcher PE image version reserved for support of the shared release-slot format.
pub const PE_IMAGE_VERSION: (u16, u16) = (u16::from_be_bytes(*b"OT"), 1);
