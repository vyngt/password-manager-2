<#
.SYNOPSIS
    Prove — for real, from evidence — that NO pre-5.2 legacy artifact exists on this machine.

.DESCRIPTION
    Slice 5.2.5 retires the one-shot migration surface that carries a vault from its old shape
    (a bare `<name>.vdb` file + sibling `<name>.vedge_blobs/`, keyed in the OS keychain by
    *path* — `vault:{path}` — and by a path-hashed Windows Hello credential) to the modern
    self-contained home (`<name>.vedge/` holding `vault.vdb` + `blobs/` + `snapshots/`, keyed by
    the intrinsic `vault_uuid` — `secret:{uuid}` / `counter:{uuid}` / `vedge-biometric-<hash of
    uuid>`).

    🔴 That migration code is the ONLY thing that will ever reach a legacy artifact. Deleting it
    with one still on disk strands it forever — unreachable by every future version of VEdge. So
    this script runs BEFORE the deletion and must PROVE the machine is clean. It never deletes
    anything; it only reports and sets an exit code.

    THE DISCIPLINE (inherited from `keychain_audit.ps1`, and the whole point):
      🔴 "delete everything" is the wrong answer to "I can't see anything." This script proves
      ABSENCE. If it cannot enumerate a surface — no sqlite3, no app.db, an unreadable vault, a
      credential store it cannot list — it FAILS. Cannot-see ⇒ FAIL, never PASS.

    THE FIVE PROBES (all must pass for a clean exit):
      1. Registry rows      — every `vault_registry.path` ends in `.vedge` (not a bare `.vdb`).
      2. On disk            — no bare `*.vdb` file and no `*.vedge_blobs/` dir beside any home,
                              nor in any configured `backup_dir`.
      3. Secret-Key store   — no `vault:*` keychain account under the `vedge` service.
      4. Biometric store    — every `vedge-biometric-*` credential hashes to a known live uuid
                              (a path-hashed one would not).
      5. Vault configs      — every reachable `vault_config.vault_uuid` is non-null (no pre-4.6).

    Run order is not negotiable: convert every vault → `mise legacy-audit` GREEN → then delete.
    An audit run AFTER the deletion proves nothing; the tools that find the orphans are gone.

.PARAMETER AppDb
    The app database listing your registered vaults. Defaults to the dev data dir
    (`local/app.db` — vedge-tauri points `app_dir` there under `cfg(debug_assertions)`).

.PARAMETER Service
    The keychain service holding Secret Keys / rollback baselines. Default `vedge` (production).

.PARAMETER BiometricService
    The credential-store service holding Windows Hello entries. Default `vedge-biometric`.

.EXAMPLE
    mise legacy-audit            # prove the machine is clean; exit 0 only if it is
#>
[CmdletBinding()]
param(
    [string]$AppDb = 'local/app.db',
    [string]$Service = 'vedge',
    [string]$BiometricService = 'vedge-biometric'
)

$ErrorActionPreference = 'Stop'

# Every probe appends a human-readable line here; a non-empty list ⇒ non-zero exit. "Cannot
# enumerate" failures land here too — proving-absence and being-blind are both FAIL.
$failures = [System.Collections.Generic.List[string]]::new()
function Fail([string]$msg) { $failures.Add($msg) }

# URL-safe, no-pad base64 of SHA-256(text) — mirrors `URL_SAFE_NO_PAD.encode(sha256(uuid))` in
# `infrastructure/biometric/windows.rs::key_name`. Used to recognise a live biometric credential.
function Get-UuidHash([string]$text) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $digest = $sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($text))
    } finally {
        $sha.Dispose()
    }
    [System.Convert]::ToBase64String($digest).TrimEnd('=').Replace('+', '-').Replace('/', '_')
}

Write-Host ""
Write-Host "VEdge legacy-artifact audit  (read-only — deletes nothing)"
Write-Host "  app db            : $AppDb"
Write-Host "  keychain service  : $Service"
Write-Host "  biometric service : $BiometricService"
Write-Host ""

# ---------------------------------------------------------------------------
# Enumerate: the registered vaults, and each one's intrinsic uuid.
#   These uuids are the "known live" set that probe 4 recognises a biometric credential against.
# ---------------------------------------------------------------------------

if (-not (Get-Command sqlite3 -ErrorAction SilentlyContinue)) {
    Fail "cannot enumerate: sqlite3 is not on PATH — without it the registry and vault configs are unreadable."
}
if (-not (Test-Path -LiteralPath $AppDb)) {
    Fail "cannot enumerate: app database not found at '$AppDb'. Open the app once, or pass -AppDb <path>."
}

$knownUuids = [System.Collections.Generic.HashSet[string]]::new()
$registryReadable = $false

if ($failures.Count -eq 0) {
    # `path|uuid` rows. A read error (e.g. the table is gone) throws — which is itself a cannot-
    # enumerate FAIL, caught below.
    # A native sqlite3 error does NOT throw in PowerShell — it prints to stderr and returns a
    # non-zero exit. So redirect stderr and check $LASTEXITCODE after every query; a read we
    # cannot complete is a cannot-enumerate FAIL (or, for probe 5, a pre-4.6 tell).
    $rows = @(& sqlite3 -separator '|' $AppDb "SELECT path, COALESCE(vault_uuid, '') FROM vault_registry;" 2>$null)
    if ($LASTEXITCODE -ne 0) {
        Fail "cannot enumerate: could not read 'vault_registry' from '$AppDb' (is the app-db schema migrated?)."
    } else {
        $registryReadable = $true
    }

    # ----- Probe 1: registry rows are homes, not bare .vdb paths -----
    # ----- Probe 5: every reachable vault_config.vault_uuid is non-null -----
    # (both walk the same rows; also seed knownUuids for probe 4 and the parent dirs for probe 2)
    $parentDirs = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)

    foreach ($line in $rows) {
        if (-not $line) { continue }
        $parts = $line -split '\|', 2
        $path = $parts[0]
        $rowUuid = if ($parts.Count -gt 1) { $parts[1] } else { '' }
        if (-not $path) { continue }

        # Probe 1 — a `.vdb`-suffixed registry row is the legacy layout by definition.
        if ($path -match '(?i)\.vdb$') {
            Fail "probe 1 (registry): legacy '.vdb' path still registered: $path"
        } elseif ($path -notmatch '(?i)\.vedge$') {
            Fail "probe 1 (registry): path is neither a '.vedge' home nor a legacy '.vdb': $path"
        }

        # The parent is where a bare-`.vdb` / `.vedge_blobs` sibling would sit (probe 2).
        # ([IO.Path]::GetDirectoryName, not `Split-Path -LiteralPath … -Parent` — `-LiteralPath`
        # and `-Parent` are mutually-exclusive Split-Path parameter sets.)
        $parent = [System.IO.Path]::GetDirectoryName($path)
        if ($parent) { [void]$parentDirs.Add($parent) }

        # A row uuid (populated for newer rows) counts toward the known set even if the vault is
        # offline — so probe 4 doesn't misread an offline vault's live credential as legacy.
        if ($rowUuid) { [void]$knownUuids.Add($rowUuid) }

        # Probe 5 needs the authoritative uuid from the vault's own config.
        $vdb = Join-Path $path 'vault.vdb'
        if (-not (Test-Path -LiteralPath $path)) {
            Fail "cannot enumerate: registered vault is not present: $path (connect it / remove the stale row, then re-run)."
            continue
        }
        if (-not (Test-Path -LiteralPath $vdb)) {
            Fail "cannot enumerate: registered home has no vault.vdb: $path"
            continue
        }
        # A missing vault_uuid COLUMN (not just a null value) is itself a pre-4.6 tell.
        $cfgUuid = (& sqlite3 $vdb "SELECT COALESCE(vault_uuid, '') FROM vault_config WHERE id='default';" 2>$null)
        if ($LASTEXITCODE -ne 0) {
            Fail "probe 5 (config): pre-4.6 vault — vault_uuid is not readable: $path"
            continue
        }
        if (-not $cfgUuid) {
            Fail "probe 5 (config): pre-4.6 vault with a null vault_uuid: $path"
        } else {
            [void]$knownUuids.Add("$cfgUuid")
        }

        # ----- Probe 2 (backup_dir half): a configured snapshot-store override, if any. Best-
        #       effort — older vaults predate the column (added 5.2.1), so a read error just
        #       means "no override to scan", not a legacy artifact. -----
        $backupDir = (& sqlite3 $vdb "SELECT COALESCE(backup_dir, '') FROM vault_config WHERE id='default';" 2>$null)
        if ($LASTEXITCODE -eq 0 -and $backupDir -and (Test-Path -LiteralPath "$backupDir")) {
            [void]$parentDirs.Add("$backupDir")
        }
    }

    # ----- Probe 2: no bare .vdb file / .vedge_blobs dir beside any home (non-recursive:
    #                a home's own `<home>/vault.vdb` lives one level down and is NOT matched) -----
    foreach ($dir in $parentDirs) {
        if (-not (Test-Path -LiteralPath $dir)) { continue }
        try {
            $strayVdb = @(Get-ChildItem -LiteralPath $dir -File -Filter '*.vdb' -ErrorAction Stop)
            $strayBlobs = @(Get-ChildItem -LiteralPath $dir -Directory -Filter '*.vedge_blobs' -ErrorAction Stop)
        } catch {
            Fail "cannot enumerate: could not list directory '$dir' ($($_.Exception.Message))."
            continue
        }
        foreach ($f in $strayVdb) { Fail "probe 2 (disk): bare legacy '.vdb' file: $($f.FullName)" }
        foreach ($d in $strayBlobs) { Fail "probe 2 (disk): legacy '.vedge_blobs' dir: $($d.FullName)" }
    }
}

# ---------------------------------------------------------------------------
# Probe 3 + 4: the credential stores. `cmdkey /list` enumerates the Windows Credential Manager.
#   Target lines look like:  Target: LegacyGeneric:target=secret:{uuid}.vedge
#   The service is the SUFFIX, so anchor on it (avoids matching `vedge-e2e-test` under `vedge`).
# ---------------------------------------------------------------------------

if (-not (Get-Command cmdkey -ErrorAction SilentlyContinue)) {
    Fail "cannot enumerate: cmdkey is not available — the credential store cannot be read (probes 3 & 4)."
} else {
    $credLines = @()
    try {
        $credLines = @(cmdkey /list)
    } catch {
        Fail "cannot enumerate: 'cmdkey /list' failed ($($_.Exception.Message)) — probes 3 & 4 are blind."
    }

    # ----- Probe 3: zero `vault:*` accounts under the Secret-Key service -----
    $svcEsc = [regex]::Escape($Service)
    foreach ($t in ($credLines | Select-String -Pattern "target=(vault:.+\.$svcEsc)\s*$")) {
        Fail "probe 3 (keychain): legacy path-keyed Secret Key: $($t.Matches[0].Groups[1].Value)"
    }

    # ----- Probe 4: every `vedge-biometric-*` credential hashes to a known live uuid -----
    $knownHashes = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($u in $knownUuids) { [void]$knownHashes.Add((Get-UuidHash $u)) }

    $bioEsc = [regex]::Escape($BiometricService)
    foreach ($t in ($credLines | Select-String -Pattern "target=(vedge-biometric-([^.]+)\.$bioEsc)\s*$")) {
        $full = $t.Matches[0].Groups[1].Value
        $hash = $t.Matches[0].Groups[2].Value
        if (-not $knownHashes.Contains($hash)) {
            if (-not $registryReadable -or $knownUuids.Count -eq 0) {
                Fail "cannot enumerate: biometric credential '$full' can't be validated — no live uuids were read to compare against."
            } else {
                Fail "probe 4 (biometric): credential does not match any live vault (legacy path-hash?): $full"
            }
        }
    }
}

# ---------------------------------------------------------------------------
# Verdict
# ---------------------------------------------------------------------------

Write-Host "known live vaults : $($knownUuids.Count)"
Write-Host ""

if ($failures.Count -eq 0) {
    Write-Host "PROVEN CLEAN — no legacy artifact found, and every surface was enumerable."
    Write-Host "Safe to retire the migration surface (slice 5.2.5)."
    exit 0
}

Write-Warning "NOT clean — $($failures.Count) finding(s). Do NOT delete the migration code yet."
Write-Host ""
foreach ($f in $failures) { Write-Host "    $f" }
Write-Host ""
Write-Host "Each line is either a legacy artifact to convert/clean, or a surface this audit could"
Write-Host "not read (which is itself a failure — an audit that cannot see cannot prove absence)."
exit 1
