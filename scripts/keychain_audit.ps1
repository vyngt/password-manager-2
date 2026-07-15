<#
.SYNOPSIS
    Audit — and optionally purge — the OS-keychain entries VEdge has left behind.

.DESCRIPTION
    VEdge stores two credentials per vault in the Windows Credential Manager, keyed on the
    vault's intrinsic `vault_uuid` (slice 5.2.0):

        secret:{uuid}.vedge      the vault's Secret Key      🔴 LOSING THIS LOSES THE VAULT
        counter:{uuid}.vedge     the rollback baseline (5.2c)  (harmless to lose; re-established)
        vault:{path}.vedge       LEGACY pre-5.2.0 path-keyed Secret Key

    Nothing ever deletes them. A vault removed from the picker, a temp vault from a dev run, an
    e2e vault whose folder is long gone — each leaves its pair behind forever. And because they
    are keyed on an opaque ULID, **you cannot tell which is which by looking**. That is the
    problem this script exists to solve: it decides orphan-vs-live from *evidence*, not from a
    name.

    HOW IT DECIDES (this is the whole safety argument — read it before running -Delete):

      1. Read the vaults you have registered, from `local/app.db` → `vault_registry.path`.
      2. For each one still on disk, read `vault_config.vault_uuid` out of its `vault.vdb`.
         That set of uuids is LIVE. Everything protecting them is KEPT.
      3. Every other entry under the service is an ORPHAN.

    🔴 THE LIMIT OF THAT ARGUMENT: a vault that exists on disk but is NOT registered in
    `vault_registry` is invisible to step 1, so its Secret Key would be classed as an orphan.
    Open every vault you care about once (so it lands in the picker) BEFORE running -Delete.
    The script prints exactly what it will protect; check that list against what you own.

    Losing a `secret:` entry is recoverable ONLY with that vault's Emergency Kit.

.PARAMETER Delete
    Actually remove the orphans. Without it, the script only reports (default, and safe).

.PARAMETER Service
    Which keychain service to audit. Default `vedge` (production).

    Pass `vedge-e2e-test` to purge the e2e suite's entries. Those are unconditionally orphans —
    the harness writes them under their own service precisely so they can be deleted without any
    of the reasoning above (see `resolve_keychain` in vedge-tauri/src/setup/services.rs).

.PARAMETER AppDb
    The app database to read registered vaults from. Defaults to the dev data dir (`local/app.db`
    — vedge-tauri points `app_dir` there under `cfg(debug_assertions)`).

.EXAMPLE
    mise keychain-audit          # report only — start here
    mise keychain-clean          # delete the orphans it found
    mise e2e-clean               # delete every e2e entry (always safe)
#>
[CmdletBinding()]
param(
    [switch]$Delete,
    [string]$Service = 'vedge',
    [string]$AppDb = 'local/app.db'
)

$ErrorActionPreference = 'Stop'

# The e2e suite files its entries under a service of their own, so every entry there is a test
# artifact by construction — no app.db, no uuid matching, no risk.
$isTestService = $Service -like '*-e2e-test'

# ---------------------------------------------------------------------------
# 1. Which vaults do you actually have? (skipped for the test service)
# ---------------------------------------------------------------------------

$liveUuids = @()
$livePaths = @()

if (-not $isTestService) {
    if (-not (Get-Command sqlite3 -ErrorAction SilentlyContinue)) {
        throw "sqlite3 is not on PATH — it is needed to read your registered vaults, and without that this script cannot tell an orphan from your real Secret Key."
    }
    if (-not (Test-Path $AppDb)) {
        throw "app database not found at '$AppDb'. Without it this script cannot see which vaults are yours. Pass -AppDb <path>, or open the app once."
    }

    foreach ($p in @(& sqlite3 $AppDb "SELECT path FROM vault_registry;")) {
        if (-not $p) { continue }
        $vdb = Join-Path $p 'vault.vdb'
        if (Test-Path $vdb) {
            $u = (& sqlite3 $vdb "SELECT vault_uuid FROM vault_config WHERE id='default';")
            if ($u) {
                $liveUuids += $u
                $livePaths += "$u  $p"
            }
        }
    }
}

# ---------------------------------------------------------------------------
# 2. What is in the credential store?
# ---------------------------------------------------------------------------

# `cmdkey /list` prints e.g. `Target: LegacyGeneric:target=secret:{uuid}.vedge`
# The service is the SUFFIX, so anchor on it to avoid matching `vedge-e2e-test` when
# auditing `vedge`.
$targets = (cmdkey /list) |
    Select-String -Pattern "Target:\s*(\S+\.$([regex]::Escape($Service)))\s*$" |
    ForEach-Object { $_.Matches[0].Groups[1].Value } |
    Sort-Object -Unique

$keep = [System.Collections.Generic.List[string]]::new()
$orphans = [System.Collections.Generic.List[string]]::new()

foreach ($t in $targets) {
    if ($isTestService) { $orphans.Add($t); continue }

    if ($t -match 'target=(?:secret|counter):([^.]+)\.') {
        # uuid-keyed (5.2.0+). Live iff a registered, on-disk vault carries that uuid.
        if ($liveUuids -contains $Matches[1]) { $keep.Add($t) } else { $orphans.Add($t) }
    }
    elseif ($t -match 'target=vault:(.+)\.' + [regex]::Escape($Service) + '$') {
        # LEGACY path-keyed. Live iff that path still exists (5.2.0 migrates these on open).
        if (Test-Path -LiteralPath $Matches[1]) { $keep.Add($t) } else { $orphans.Add($t) }
    }
    else {
        # Unrecognised shape — never delete something we do not understand.
        $keep.Add($t)
    }
}

# ---------------------------------------------------------------------------
# 3. Report
# ---------------------------------------------------------------------------

Write-Host ""
Write-Host "service         : $Service"
if (-not $isTestService) {
    Write-Host "registered vaults on disk : $($liveUuids.Count)"
    foreach ($l in $livePaths) { Write-Host "    KEEP  $l" }
} else {
    Write-Host "(test service — every entry here is an e2e artifact by construction)"
}
Write-Host ""
Write-Host "credentials found : $($targets.Count)"
Write-Host "  protected       : $($keep.Count)"
Write-Host "  orphaned        : $($orphans.Count)"

if ($orphans.Count -eq 0) {
    Write-Host ""
    Write-Host "Nothing to clean."
    exit 0
}

if (-not $Delete) {
    Write-Host ""
    Write-Host "Orphans (not deleted — this is a dry run):"
    foreach ($o in $orphans) { Write-Host "    $o" }
    Write-Host ""
    Write-Host "Re-run with -Delete (or ``mise keychain-clean``) to remove them."
    exit 0
}

# ---------------------------------------------------------------------------
# 4. Delete — with the one guard that matters
# ---------------------------------------------------------------------------

# 🔴 If we could not see a single vault of yours, we have no evidence about ANY of these
# entries — and "delete everything" is exactly the wrong answer to "I can't see anything".
# That is how a Secret Key gets destroyed.
if (-not $isTestService -and $liveUuids.Count -eq 0) {
    Write-Host ""
    Write-Warning "REFUSING to delete: no registered, on-disk vault was found via '$AppDb', so every entry looks like an orphan — including, possibly, the key to a real vault."
    Write-Warning "Open the vaults you care about once (so they register in the picker), then re-run."
    exit 1
}

$failed = 0
foreach ($o in $orphans) {
    cmdkey /delete:$o | Out-Null
    if ($LASTEXITCODE -eq 0) { Write-Host "  deleted $o" } else { $failed++; Write-Warning "  FAILED  $o" }
}

Write-Host ""
Write-Host "Removed $($orphans.Count - $failed) of $($orphans.Count) orphaned entries; $($keep.Count) protected."
if ($failed -gt 0) { exit 1 }
