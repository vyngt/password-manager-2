<#
.SYNOPSIS
    Compute SHA-256 checksums for the release installers and write SHA256SUMS.txt.

.DESCRIPTION
    VEdge 1.0 ships UNSIGNED (no code-signing certificate — Gate 1.0 §D), so the SHA-256
    checksum IS the integrity story. The user guide's install section tells users to run
    `Get-FileHash <installer> -Algorithm SHA256` and compare against the value published on
    the GitHub release, so a real release must publish those values.

    This produces them for every installer `mise build` emitted (the NSIS `-setup.exe` and the
    MSI), writing a `SHA256SUMS.txt` beside them in the standard `<hash>  <name>` format
    (compatible with `sha256sum -c`), and prints them.

    Run it AFTER `mise build`. It fails (exit 1) if no installer is found, so it can't silently
    "succeed" on an empty bundle dir.

.PARAMETER BundleDir
    The Tauri bundle output dir. Default: target/release/bundle.

.EXAMPLE
    mise build; mise checksums
#>
[CmdletBinding()]
param(
    [string]$BundleDir = "target/release/bundle"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $BundleDir)) {
    Write-Error "Bundle dir '$BundleDir' not found. Run 'mise build' first."
    exit 1
}

# The distributables a user actually downloads: the NSIS setup .exe and the MSI.
$installers = Get-ChildItem -Path $BundleDir -Recurse -File -Include *.exe, *.msi |
    Sort-Object Name

if ($installers.Count -eq 0) {
    Write-Error "No installers (*.exe / *.msi) under '$BundleDir'. Run 'mise build' first."
    exit 1
}

$lines = foreach ($f in $installers) {
    $hash = (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $($f.Name)"
}

$outFile = Join-Path (Resolve-Path -LiteralPath $BundleDir).Path "SHA256SUMS.txt"
# UTF-8 without BOM (sha256sum-friendly).
[System.IO.File]::WriteAllLines($outFile, [string[]]$lines, (New-Object System.Text.UTF8Encoding($false)))

Write-Host "SHA-256 checksums ($($installers.Count) installer(s)) -> $outFile" -ForegroundColor Green
$lines | ForEach-Object { Write-Host "  $_" }
