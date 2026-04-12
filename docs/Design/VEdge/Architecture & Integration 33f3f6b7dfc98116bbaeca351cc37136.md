# Architecture & Integration

The *how* — layer boundaries, tech stack decisions, API contracts, and concrete implementation guides. Open these when starting a new feature or wiring a new integration.

---

## Core principle: platform-agnostic by design

`vedge-core` is a pure Rust library with zero platform dependencies. Every deployment target is a thin shell adapter that calls into the same core. Business logic is written once, runs anywhere.

```jsx
vedge-core  ←  vedge-tauri   (Tauri desktop app)
            ←  vedge-server  (Axum: PKI automation, sync server)
            ←  vedge-cli     (scripting, CI/CD)
            ←  vedge-wasm    (future web app)
```

---

## Tech stack

| Layer | Technology | Rationale |
| --- | --- | --- |
| UI | Leptos (Rust → WASM) | Full Rust stack, no JS except browser extension |
| Core | Rust (Tauri) | Crypto, vault, PKI, steganography, LAN sync |
| Sidecar | Elixir (OTP) | Scheduler, breach monitoring, background jobs |
| Sync server | Elixir + Phoenix | Self-hosted, WebSocket, Presence, fault-tolerant |
| Database (local) | SQLite via sqlx | Embedded, encrypted, no server needed |
| Database (sync server) | PostgreSQL + Oban | Background jobs with retry, dead-letter queue |

### Key Rust crates

| Feature | Crate |
| --- | --- |
| Encryption | `ring`, `age`, `aes-gcm` |
| Password hashing | `argon2` |
| SQLite | `sqlx` |
| PKI / Certs | `rcgen`, `x509-parser`, `rustls` |
| SSH keys | `ssh-key` |
| mDNS discovery | `mdns-sd` |
| QUIC transport | `quinn` |
| Noise Protocol | `snow` |
| CRDT sync | `automerge` |
| Steganography | `steganography` or custom LSB |
| QR code | `qrcode` |

---

## LAN sync architecture

Devices on the same WiFi network sync without any cloud server:

```jsx
Device A                          Device B
   |                                 |
mDNS broadcast                  mDNS discovery
"_vedge._tcp.local"      →      finds Device A
   |                                 |
   └──────── QUIC / TLS 1.3 ────────┘
                  |
          Noise Protocol
        (P2P handshake,
         mutual auth)
                  |
          CRDT sync engine
          (automerge-rs)
          only diffs synced
```

**Pairing flow:**

1. Device A shows QR code containing its public key + address
2. Device B scans QR → has Device A's public key
3. Noise handshake → mutual authentication
4. Encrypted QUIC channel established
5. CRDT sync begins — only diffs, not the full vault

No server involved. Encryption happens before data leaves the device.

---

## Elixir sidecar

The Elixir sidecar runs alongside the Tauri app as a child process (Tauri external binary support). It handles tasks that benefit from OTP supervision and scheduling:

- **Dead Man's Switch** — GenServer monitors check-in intervals, sends emergency access if user goes silent
- **Breach monitoring** — Oban worker pool checks HaveIBeenPwned for all emails, throttled, auto-retry
- **Sync coordination** — optional bridge to the self-hosted Elixir sync server

The sidecar communicates with the Rust core via Unix socket or local HTTP. It never holds decryption keys.

---

## Self-hosted sync server

Users who want cross-internet sync (not just LAN) can self-host the Vedge sync server — a Docker image built on Elixir + Phoenix.

- Phoenix Channels for WebSocket connections
- One GenServer process per connected device
- Phoenix.Presence for online/offline tracking
- PubSub for broadcasting diffs between devices
- All payloads are E2E encrypted — server sees only opaque blobs

---

## Distribution

| Platform | Format | Tooling |
| --- | --- | --- |
| macOS | `.dmg`  • notarization | Tauri built-in + GitHub Actions |
| Windows | `.msi`  • NSIS | Tauri built-in |
| Linux | `.AppImage`, `.deb`, `.rpm` | Tauri built-in |
| Browser extension | Chrome Web Store, Firefox AMO | WebExtension API (JS shell only) |
| Sync server | Docker image | Elixir release + Docker |

Auto-update via Tauri updater (delta updates, signature verified before install).

---

## Styling

[Styling Approach — Tailwind v4 + plain CSS](Architecture%20&%20Integration/UI/Styling%20Approach%2033f3f6b7dfc98156b083fe0ceab46723.md)

[tokens.css — Reference](Architecture%20&%20Integration/UI/tokens%20css%20%E2%80%94%20Reference%2033f3f6b7dfc98121add4c87a8f49da07.md)

---

## Theme system

[Theme System Architecture & Integration Guide](Architecture%20&%20Integration/UI/Theme%20System%20Architecture%20&%20Integration%20Guide%2033f3f6b7dfc981e5bbfec5f2ef183806.md)

---

## Sub-sections

[Application Architecture — Clean Architecture & Workspace Design](Architecture%20&%20Integration/Application%20Architecture%20%E2%80%94%20Clean%20Architecture%20&%20Wo%203403f6b7dfc9811ca55afebfbd22879e.md)

[UI](Architecture%20&%20Integration/UI%2033f3f6b7dfc98031ace9f61e778a357f.md)