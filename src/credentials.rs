//! OS keychain credential storage for GemStone account passwords.
//!
//! Uses the system keyring (Keychain on macOS, libsecret/kwallet on Linux,
//! Windows Credential Manager on Windows). Falls back gracefully if unavailable.

const SERVICE: &str = "vellum-fe-tabbed";

/// Store a password for the given account name in the OS keychain.
pub fn store_password(account: &str, password: &str) {
    let entry = keyring::Entry::new(SERVICE, account);
    match entry {
        Ok(e) => { let _ = e.set_password(password); }
        Err(_) => {}
    }
}

/// Retrieve a stored password for the given account name, if any.
pub fn get_password(account: &str) -> Option<String> {
    let entry = keyring::Entry::new(SERVICE, account).ok()?;
    entry.get_password().ok()
}

/// Delete a stored password for the given account name.
pub fn delete_password(account: &str) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, account) {
        let _ = entry.delete_password();
    }
}
