# Concepts & Philosophy

The *why* behind every decision in Vedge — principles, mental models, and rules that apply across the entire product. Read these once to understand the system. They change rarely, and only when a foundational principle changes.

---

## Product philosophy

Vedge is a **personal security operations platform** — not a password manager. It fills the gap between tools that are too simple (1Password) and too complex (HashiCorp Vault), for individuals and small teams who want everything local-first with no cloud dependency.

Four principles drive every product and architecture decision:

1. **Local-first** — data lives on the user's device. Cloud is opt-in, never required. The app is fully functional with no internet connection.
2. **Zero-trust toward infrastructure** — even the optional sync server never sees plaintext. All encryption and decryption happens on the device.
3. **Progressive disclosure** — show only what is needed at each moment. Complexity is one keystroke away, never in the way.
4. **Depth over breadth** — each feature must be done to its absolute limit before adding the next. A vault that unlocks in 200ms and never corrupts data beats a vault with 20 extra features that occasionally fails. People recommend tools that never let them down, not tools that do many things poorly.

> *1Password didn't win because it had the most features. It won because autofill never failed.*
> 

> *Obsidian didn't win because it did the most things. It won because the editor never lost data.*
> 

If Vedge's vault is the fastest and most reliable, its steganography leaves zero detectable artifacts, its PKI works in one command, and its document export looks as good as Notion — people will use it and recommend it. That is the bar.

*Security through obscurity* (steganography) complements *security through encryption* (XChaCha20-Poly1305 + Argon2id). Two layers combined create a defense-in-depth that no mainstream tool offers.

Three principles guide every product decision:

1. **Local-first** — data lives on the user's device. Cloud is opt-in, not required. The app must be fully functional with no internet connection.
2. **Zero-trust toward infrastructure** — even the optional sync server never sees plaintext. All encryption and decryption happens on the device.
3. **Progressive disclosure** — the interface shows only what is needed at each moment. Complexity is one keystroke away, never in the way.

---

## Design system overview

[UX/UI Design System — Modern Minimal Tauri App](Concepts%20&%20Philosophy/UX%20UI%20Component/UX%20UI%20Design%20System%20%E2%80%94%20Modern%20Minimal%20Tauri%20App%2033f3f6b7dfc98156a49cd30363bb6655.md)

---

## Visual language

[Typography & Spacing Framework](Concepts%20&%20Philosophy/UX%20UI%20Component/Typography%20&%20Spacing%20Framework%2033f3f6b7dfc9814dbe6dd61aa98283c3.md)

[Motion & Elevation Framework](Concepts%20&%20Philosophy/UX%20UI%20Component/Motion%20&%20Elevation%20Framework%2033f3f6b7dfc981c9b3f1e7bf6330cb3c.md)

---

## Token system

[Color Token Framework](Concepts%20&%20Philosophy/UX%20UI%20Component/Color%20Token%20Framework%2033f3f6b7dfc98185a624d3e857f4de87.md)

[Color System — Deep Specification](Concepts%20&%20Philosophy/UX%20UI%20Component/Color%20Token%20Framework/Color%20System%20%E2%80%94%20Deep%20Specification%2033f3f6b7dfc98167a1fed98f96bc8a82.md)

---

## Component rules

[Component System Rules & Philosophy](Concepts%20&%20Philosophy/UX%20UI%20Component/System%20Rules%20&%20Philosophy%2033f3f6b7dfc98138bb34d3fc660ebfff.md)

[UX/UI/Component](Concepts%20&%20Philosophy/UX%20UI%20Component%2033f3f6b7dfc980eeac5eef169a776349.md)

---

## Core & backend concepts

[Core & Backend](Concepts%20&%20Philosophy/Core%20&%20Backend%2033f3f6b7dfc980bc8934f7b9f91282cb.md)