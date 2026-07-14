#!/usr/bin/env pwsh
#
# BEACON session helper (Windows / PowerShell).
# Manages .beacon-session.json, the marker the resume protocol and the optional
# pre-commit hook look for.
#
# Usage:
#   .beacon\session.ps1 start "session goal"   # begin a session
#   .beacon\session.ps1 end                     # end the session
#   .beacon\session.ps1 status                  # show current session
#
param(
  [ValidateSet("start", "end", "stop", "status")]
  [string] $Command = "status",
  [string] $Goal = ""
)
$ErrorActionPreference = "Stop"

$root = (git rev-parse --show-toplevel 2>$null)
if (-not $root) { $root = (Get-Location).Path }
$file = Join-Path $root ".beacon-session.json"

switch ($Command) {
  "start" {
    $ts = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $branch = (git rev-parse --abbrev-ref HEAD 2>$null)
    if (-not $branch) { $branch = "unknown" }
    [ordered]@{ started_at = $ts; branch = $branch; goal = $Goal } |
      ConvertTo-Json | Set-Content -LiteralPath $file -Encoding utf8
    $shown = if ($Goal) { $Goal } else { "<none>" }
    Write-Host "BEACON session started ($ts). Goal: $shown"
  }
  { $_ -in "end", "stop" } {
    Remove-Item -LiteralPath $file -ErrorAction SilentlyContinue
    Write-Host "BEACON session ended."
  }
  "status" {
    if (Test-Path -LiteralPath $file) { Write-Host "Active BEACON session:"; Get-Content -LiteralPath $file }
    else { Write-Host "No active BEACON session." }
  }
}
