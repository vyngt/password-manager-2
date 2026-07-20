//! Compile-time pins for the attack surface PG.2b curated and PG.3 must NOT move.
//!
//! 🔴 PG.3's acceptance criterion ①: adding the update check must leave
//! `capabilities/default.json` and `app.security.csp` **byte-identical** — the
//! check is Rust-side (`commands/update.rs`), so the frontend gains no HTTP
//! capability and the CSP's `connect-src` stays closed. These tests fail loudly
//! if either moves, so a future slice has to update the pin *out loud* — the same
//! "remove the possibility" discipline as 5.4.1's guard and 5.8's no-`..` match.
//!
//! No runtime code — only the tests below.

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    /// The exact frontend capability grants PG.2b curated (8). The update check
    /// is Rust-side, so this must not gain an `http:*`/network capability.
    const EXPECTED_PERMISSIONS: &[&str] = &[
        "core:default",
        "core:window:allow-minimize",
        "core:window:allow-maximize",
        "core:window:allow-unmaximize",
        "core:window:allow-close",
        "core:window:allow-start-dragging",
        "dialog:allow-open",
        "dialog:allow-save",
    ];

    /// The exact CSP PG.2b set. `connect-src` allows NO remote origin — the update
    /// check reaches GitHub from Rust, so this must not gain one.
    const EXPECTED_CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self'; font-src 'self'; connect-src 'self' ipc: http://ipc.localhost; object-src 'none'; base-uri 'self'; frame-ancestors 'none'; form-action 'none'";

    #[test]
    fn capability_grants_are_byte_identical_to_pg2b() {
        const RAW: &str = include_str!("../capabilities/default.json");
        let json: serde_json::Value = serde_json::from_str(RAW).unwrap();
        let perms: Vec<&str> = json["permissions"]
            .as_array()
            .expect("capabilities/default.json has a `permissions` array")
            .iter()
            .map(|v| v.as_str().expect("each permission is a string"))
            .collect();
        assert_eq!(
            perms, EXPECTED_PERMISSIONS,
            "capabilities/default.json changed — PG.3's ① requires the frontend \
             capability set stay byte-identical (the update check is Rust-side). \
             If this change is deliberate, update EXPECTED_PERMISSIONS on purpose."
        );
    }

    #[test]
    fn csp_is_byte_identical_to_pg2b() {
        const RAW: &str = include_str!("../tauri.conf.json");
        let json: serde_json::Value = serde_json::from_str(RAW).unwrap();
        let csp = json["app"]["security"]["csp"]
            .as_str()
            .expect("tauri.conf.json has an `app.security.csp` string");
        assert_eq!(
            csp, EXPECTED_CSP,
            "app.security.csp changed — PG.3's ① requires the CSP stay \
             byte-identical (no remote `connect-src` for the update check). If \
             this change is deliberate, update EXPECTED_CSP on purpose."
        );
    }
}
