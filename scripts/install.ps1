$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

cargo install --path . --force --bins

@'
Installed Cloche.

Try:
  cloche doctor --format json
  cloche capture --target active --presentation both --format json
  cloche latest
  cloche preview
'@ | Write-Host
