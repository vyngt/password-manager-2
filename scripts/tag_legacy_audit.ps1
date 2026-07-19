<#
.SYNOPSIS
    Prove — for real, from evidence — that NO pre-5.6.0 legacy (KEK-sealed) tag survives on this
    machine, so slice 5.9 ② can retire the legacy-tag read path.

.DESCRIPTION
    5.6.0 gave every tag its own per-row DEK (`tags.dek_wrapped`). A tag written by a pre-5.6.0
    build is sealed DIRECTLY under the KEK and has `dek_wrapped IS NULL`; at unlock, `build_index`
    migrates it to a DEK. The ONLY code that can still read a NULL-`dek_wrapped` tag is
    `decrypt_legacy_tag` + `migrate_legacy_tag`. Deleting that path while any such tag can still be
    reached (a live vault never re-opened since 5.6.0, a snapshot, or a pre-5.6.0 `.vbk` restored
    later) would brick it.

    🔴 So — like `legacy_audit.ps1` (slice 5.2.5) — this proves ABSENCE from evidence. It counts
    `tags WHERE dek_wrapped IS NULL` across every reachable surface: each registered vault's
    `vault.vdb`, each of its snapshots, and every `.vbk` found in a scannable location (extracted +
    queried; `dek_wrapped` is a plaintext column, no KEK needed). Cannot-see ⇒ FAIL: a surface it
    cannot read is a failure, never a pass. Run order: unlock/migrate every dev vault → GREEN →
    only then retire the code. An audit after the deletion proves nothing.

    ⚠️ A `.vbk` can be saved anywhere; this scans the vault-home parents + configured `backup_dir`s.
    A backup kept in an unscanned folder is invisible here — which is exactly why, if this cannot be
    made unconditionally GREEN, the honest outcome is CONFIRM-KEEP, not deletion.

.PARAMETER AppDb
    The app database listing registered vaults. Defaults to the dev data dir (`local/app.db`).

.EXAMPLE
    mise tag-legacy-audit
#>
[CmdletBinding()]
param(
    [string]$AppDb = 'local/app.db',
    # Extra folders to scan for `.vbk` backups (a backup can live anywhere the user saved it — the
    # registered-vault parents + configured backup_dirs are NOT the whole story). Pass every folder
    # you keep backups in, e.g. -ScanDir V:\tmp\BK1,D:\backups.
    [string[]]$ScanDir = @()
)

$ErrorActionPreference = 'Stop'

$failures = [System.Collections.Generic.List[string]]::new()
$legacyFindings = [System.Collections.Generic.List[string]]::new()
$surfaces = 0
function Fail([string]$msg) { $failures.Add($msg) }

# Classify a vault.vdb by its legacy-tag count. Returns 'clean', 'legacy:<n>', or 'unreadable'.
#   - the NULL-dek query succeeds  → clean / legacy by the count.
#   - it errors but `tags` has rows → the `dek_wrapped` column is missing (a pre-5.6.0 schema),
#     so every tag there is KEK-sealed = legacy.
#   - `tags` itself is unreadable   → cannot-see.
function Get-TagVerdict([string]$vdb) {
    if (-not (Test-Path -LiteralPath $vdb)) { return 'unreadable' }
    $n = (& sqlite3 $vdb "SELECT count(*) FROM tags WHERE dek_wrapped IS NULL;" 2>$null)
    if ($LASTEXITCODE -eq 0) {
        if ([int]$n -gt 0) { return "legacy:$n" }
        return 'clean'
    }
    $total = (& sqlite3 $vdb "SELECT count(*) FROM tags;" 2>$null)
    if ($LASTEXITCODE -ne 0) { return 'unreadable' }
    if ([int]$total -gt 0) { return "legacy:$total" }
    return 'clean'
}

function Check-Vdb([string]$vdb, [string]$label) {
    $script:surfaces++
    switch -Wildcard (Get-TagVerdict $vdb) {
        'clean' { }
        'legacy:*' {
            $n = ($_ -split ':', 2)[1]
            $legacyFindings.Add("$label — $n legacy (KEK-sealed) tag(s)")
            Fail "legacy tag(s) at: $label"
        }
        'unreadable' { Fail "cannot enumerate: unreadable vault.vdb at: $label" }
    }
}

Write-Host ""
Write-Host "VEdge legacy-TAG audit  (read-only — deletes nothing)"
Write-Host "  app db : $AppDb"
Write-Host ""

if (-not (Get-Command sqlite3 -ErrorAction SilentlyContinue)) {
    Fail "cannot enumerate: sqlite3 is not on PATH."
}
# Prefer Windows' own bsdtar (System32\tar.exe): it handles `V:\…` / `C:\…` paths natively, whereas
# Git's GNU tar on PATH treats a drive-colon as a remote host and mangles a Windows `-C` target.
$tarExe = Join-Path $env:SystemRoot 'System32\tar.exe'
if (-not (Test-Path -LiteralPath $tarExe)) {
    $tarExe = (Get-Command tar -ErrorAction SilentlyContinue).Source
}
if (-not $tarExe) {
    Fail "cannot enumerate: no tar available — .vbk archives can't be opened."
}
if (-not (Test-Path -LiteralPath $AppDb)) {
    Fail "cannot enumerate: app database not found at '$AppDb'. Open the app once, or pass -AppDb."
}

$parentDirs = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
foreach ($d in $ScanDir) { if ($d) { [void]$parentDirs.Add($d) } }

if ($failures.Count -eq 0) {
    $rows = @(& sqlite3 -separator '|' $AppDb "SELECT path FROM vault_registry;" 2>$null)
    if ($LASTEXITCODE -ne 0) {
        Fail "cannot enumerate: could not read 'vault_registry' from '$AppDb'."
    }

    foreach ($path in $rows) {
        if (-not $path) { continue }
        $parent = [System.IO.Path]::GetDirectoryName($path)
        if ($parent) { [void]$parentDirs.Add($parent) }

        if (-not (Test-Path -LiteralPath $path)) {
            Fail "cannot enumerate: registered vault is not present: $path"
            continue
        }

        # The live vault.
        Check-Vdb (Join-Path $path 'vault.vdb') "live: $path"

        # Every snapshot's vault.vdb (`<home>/snapshots/<id>/vault.vdb`).
        $snapDir = Join-Path $path 'snapshots'
        if (Test-Path -LiteralPath $snapDir) {
            foreach ($snap in Get-ChildItem -LiteralPath $snapDir -Directory -ErrorAction SilentlyContinue) {
                # `snapshots/objects` is the 5.3c content-addressed blob pool, not a snapshot — it has
                # no vault.vdb. Skip it; only real snapshot dirs (ULID-named) carry a vault.vdb.
                if ($snap.Name -eq 'objects') { continue }
                Check-Vdb (Join-Path $snap.FullName 'vault.vdb') "snapshot: $($snap.FullName)"
            }
        }

        # A configured backup_dir override (added 5.2.1) is another place .vbk / snapshots sit.
        $backupDir = (& sqlite3 (Join-Path $path 'vault.vdb') "SELECT COALESCE(backup_dir,'') FROM vault_config WHERE id='default';" 2>$null)
        if ($LASTEXITCODE -eq 0 -and $backupDir -and (Test-Path -LiteralPath "$backupDir")) {
            [void]$parentDirs.Add("$backupDir")
        }
    }

    # Every `.vbk` in a scannable location: extract vault.vdb to a temp dir and check it.
    foreach ($dir in $parentDirs) {
        if (-not (Test-Path -LiteralPath $dir)) { continue }
        foreach ($vbk in Get-ChildItem -LiteralPath $dir -File -Filter '*.vbk' -ErrorAction SilentlyContinue) {
            $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("vbk_" + [System.Guid]::NewGuid().ToString('N'))
            New-Item -ItemType Directory -Path $tmp -Force | Out-Null
            try {
                & $tarExe -xf $vbk.FullName -C $tmp 2>$null
                if ($LASTEXITCODE -ne 0) {
                    Fail "cannot enumerate: could not extract .vbk: $($vbk.FullName)"
                    continue
                }
                $inner = Get-ChildItem -LiteralPath $tmp -Recurse -Filter 'vault.vdb' -ErrorAction SilentlyContinue | Select-Object -First 1
                if (-not $inner) {
                    Fail "cannot enumerate: no vault.vdb inside .vbk: $($vbk.FullName)"
                    continue
                }
                Check-Vdb $inner.FullName "backup: $($vbk.FullName)"
            } finally {
                Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }
}

Write-Host "surfaces scanned : $surfaces"
Write-Host ""

if ($failures.Count -eq 0) {
    Write-Host "PROVEN CLEAN — no legacy (KEK-sealed) tag found, and every scannable surface was readable."
    Write-Host "Safe to retire the legacy-tag read path (slice 5.9 ②) — for the surfaces this can see."
    Write-Host "🔴 A .vbk saved outside the scanned locations is still invisible; weigh that before deleting."
    exit 0
}

Write-Warning "NOT clean — $($failures.Count) finding(s). CONFIRM-KEEP the legacy-tag path."
Write-Host ""
foreach ($f in $failures) { Write-Host "    $f" }
if ($legacyFindings.Count -gt 0) {
    Write-Host ""
    Write-Host "Legacy tags (unlock/migrate the vault, then re-run):"
    foreach ($l in $legacyFindings) { Write-Host "    $l" }
}
exit 1
