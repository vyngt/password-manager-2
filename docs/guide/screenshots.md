# Screenshots — how they're made and refreshed

The images in this guide are **captured automatically** by driving the real app through
VEdge's end-to-end test harness. There is no manual screenshotting, so the images can't
silently drift from the shipped UI: when the UI changes, you re-run one command.

## Regenerate them

```bash
mise screenshots
```

This builds the frontend and host binary, launches the app through `tauri-driver` +
`msedgedriver`, walks each documented state with disposable sample data, and writes a PNG
per figure into `docs/guide/images/`. Override the output directory with
`VEDGE_SHOTS_DIR`.

**Requirements** (same as `mise e2e`): a display (the capture uses a real, visible
window), `tauri-driver`, and a matching `msedgedriver` on `PATH`. Don't run it alongside a
heavy `cargo` build — a busy machine can starve the webview past the launch timeout.

The capture scenario lives in `crates/vedge-e2e/tests/screenshots.rs`. Because VEdge is a
frameless window that draws its own titlebar inside the webview, each capture includes the
app's full chrome — it's the whole window as a user sees it. Every vault it creates is a
throwaway temp vault, so any Secret Key or kit string on the creation screens is a
one-run throwaway, never a real secret.

> **Windows only.** VEdge 1.0 ships for Windows, so all screenshots are Windows chrome.
> Don't mix in captures from other platforms.
>
> **The SmartScreen warning is not in this set.** It's an operating-system dialog, not
> part of the app window, so WebDriver can't capture it; the install page describes it in
> text.

## The figures

| File | Shows |
| --- | --- |
| `01-welcome.png` | First-run welcome (no vaults yet) |
| `02-launch-help.png` | The pre-unlock Help dialog (recovery questions) |
| `03-create-vault.png` | Create wizard — name & location |
| `04-master-password.png` | Create wizard — master password |
| `05-secret-key.png` | Create wizard — Secret Key & Emergency Kit |
| `10-vault-list.png` | The vault with several entries |
| `11-entry-detail.png` | An entry's detail drawer |
| `12-entry-form.png` | The new-entry form |
| `13-search.png` | Filtered search |
| `20-generator.png` | The password generator |
| `30-health.png` | The Health page |
| `31-audit.png` | The Audit page |
| `40-snapshots.png` | The Snapshots page |
| `50-settings-security.png` | Settings ▸ Security |
| `51-settings-credentials.png` | Settings ▸ Credentials |
| `52-settings-appearance.png` | Settings ▸ Appearance |
| `53-settings-maintenance.png` | Settings ▸ Maintenance |
| `54-settings-backup.png` | Settings ▸ Backup |
| `55-settings-about.png` | Settings ▸ About |
| `60-change-password.png` | Change master password dialog |
| `61-rotate-secret-key.png` | Rotate Secret Key dialog |
| `62-rekey.png` | Re-key dialog |

If you add a documented state to the guide, add a capture for it in
`crates/vedge-e2e/tests/screenshots.rs` and a row here.
