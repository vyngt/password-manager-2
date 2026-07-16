<#
.SYNOPSIS
  Generate a VEdge-format import CSV of N login rows, for scale-testing import + unlock.

.DESCRIPTION
  Writes the FROZEN VEdge CSV exactly: header `name,username,password,url,notes,tags`, logins
  only (all the CSV adapter supports — slice 5.3a Decision 5). Tags in one cell are ';'-separated.
  Streams straight to disk via StreamWriter, so 1,000,000+ rows stay flat in memory.

  Every generated value is free of commas / quotes / newlines, so each line is exactly 6 fields —
  the importer parses by strict field count (a wrong count is skipped, not imported). ~1% of
  passwords start with '=' to exercise the formula-injection byte-exact round-trip.

  What it exercises downstream:
    - begin_import: parses all rows, builds a preview Vec, runs the url/psl dedup per row.
    - commit_import: one create_entry per row (fresh ULID + DEK + audit) -> a real write test.
    - unlock afterwards: the O(n) index-build loop over the seeded rows.

  NOTE the file scales to any N, but the GUI preview renders every row in a <table>, so drive the
  GUI with <= ~10k. For 100k / 1M UNLOCK numbers, prefer bumping the sizes array in
  crates/vedge-core/tests/unlock_bench.rs and running `mise bench` (no GUI, no giant file).

.PARAMETER Count   Number of login rows to generate (required).
.PARAMETER Out     Output CSV path. Default: vedge-import-<Count>.csv in the current directory.
.PARAMETER NoUrl   Leave the url column empty (skips the importer's per-row url/psl dedup parse).
.PARAMETER NoTags  Leave the tags column empty (skips create_tag calls at import).

.EXAMPLE
  ./scripts/gen_import_csv.ps1 -Count 10000
.EXAMPLE
  ./scripts/gen_import_csv.ps1 -Count 1000000 -Out big.csv -NoTags
#>
param(
    [Parameter(Mandatory)][long]$Count,
    [string]$Out,
    [switch]$NoUrl,
    [switch]$NoTags
)

if (-not $Out) { $Out = "vedge-import-$Count.csv" }
$full = if ([System.IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path (Get-Location).Path $Out }

# Small, fixed pools so import creates only a handful of distinct tags (match-or-create reuses them),
# and names/urls look like real services while staying unique per row.
$services = @('github', 'gitlab', 'aws', 'gmail', 'slack', 'jira', 'notion', 'stripe', 'okta', 'cloudflare')
$tagPool = @('work', 'personal', 'dev', 'finance', 'social', 'archive')
$nSvc = $services.Length
$nTag = $tagPool.Length

$sw = [System.IO.StreamWriter]::new($full, $false, [System.Text.UTF8Encoding]::new($false))
try {
    $sw.WriteLine('name,username,password,url,notes,tags')
    for ([long]$i = 0; $i -lt $Count; $i++) {
        $svc = $services[$i % $nSvc]
        $pw = if ($i % 97 -eq 0) { "=eq$i-pw" } else { "P@ss-$i-w0rd" }   # ~1% formula-risky
        $url = if ($NoUrl) { '' } else { "https://$svc$i.example.com" }
        $tag = if ($NoTags) { '' } else { $tagPool[$i % $nTag] }
        # name,username,password,url,notes(empty),tags  -> always exactly 6 fields
        $sw.WriteLine("$svc-$i,user$i@$svc.example,$pw,$url,,$tag")
    }
}
finally { $sw.Dispose() }

$size = (Get-Item $full).Length
Write-Output ("Wrote {0:N0} login rows to {1} ({2:N1} MiB)." -f $Count, $full, ($size / 1MB))
