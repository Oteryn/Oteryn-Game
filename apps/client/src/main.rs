#[cfg(windows)]
mod windows_shell;

#[cfg(windows)]
fn main() -> Result<(), windows_shell::ShellError> {
    windows_shell::run()
}

#[cfg(not(windows))]
fn main() {
    println!("Oteryn pre-native client: Windows desktop target only");
}
