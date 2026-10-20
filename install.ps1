<#
.SYNOPSIS
  Seamless Glance installer for Windows.

.DESCRIPTION
  Downloads the latest seamless-glance release binary (or a pinned version),
  verifies its checksum, and installs it to a per-user directory on PATH.

.EXAMPLE
  irm https://raw.githubusercontent.com/fells-code/seamless-glance/main/install.ps1 | iex

.EXAMPLE
  $env:SEAMLESS_GLANCE_VERSION = "1.2.3"; irm https://raw.githubusercontent.com/fells-code/seamless-glance/main/install.ps1 | iex
#>
$ErrorActionPreference = "Stop"

$Repo    = "fells-code/seamless-glance"
$BinName = "seamless-glance"
$Target  = "x86_64-pc-windows-msvc"
$Version = if ($env:SEAMLESS_GLANCE_VERSION) { $env:SEAMLESS_GLANCE_VERSION } else { "latest" }
$InstallDir = if ($env:SEAMLESS_GLANCE_INSTALL_DIR) {
  $env:SEAMLESS_GLANCE_INSTALL_DIR
} else {
  Join-Path $env:LOCALAPPDATA "Programs\seamless-glance"
}

if ($Version -eq "latest") {
  Write-Host "Resolving latest release..."
  $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" `
    -Headers @{ "User-Agent" = "seamless-glance-installer" }
  $Version = $rel.tag_name -replace '^v', ''
}

$File        = "$BinName-$Version-$Target.exe"
$Url         = "https://github.com/$Repo/releases/download/v$Version/$File"
$ChecksumUrl = "https://github.com/$Repo/releases/download/v$Version/SHA256SUMS.txt"

$Work = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ("sg-" + [guid]::NewGuid()))
try {
  $binPath = Join-Path $Work $File
  Write-Host "Downloading Seamless Glance $Version ($Target)..."
  Invoke-WebRequest -Uri $Url -OutFile $binPath -UseBasicParsing

  # Fail closed: never install a binary that was not checked against the
  # release's SHA256SUMS.txt. Any failure here stops the install.
  $sumsPath = Join-Path $Work "SHA256SUMS.txt"
  Invoke-WebRequest -Uri $ChecksumUrl -OutFile $sumsPath -UseBasicParsing
  $entry = Select-String -Path $sumsPath -Pattern ("\s" + [regex]::Escape($File) + "$") |
    Select-Object -First 1
  if (-not $entry) {
    throw "SHA256SUMS.txt has no entry for $File, refusing to install."
  }
  $expected = ($entry.Line -split '\s+')[0].ToLower()
  $actual = (Get-FileHash -Path $binPath -Algorithm SHA256).Hash.ToLower()
  if ($expected -ne $actual) {
    throw "Checksum mismatch for ${File}: expected $expected, got $actual"
  }
  Write-Host "Checksum verified."

  New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
  $dest = Join-Path $InstallDir "$BinName.exe"
  Move-Item -Force -Path $binPath -Destination $dest
  # Convenience alias `glance`.
  Copy-Item -Force -Path $dest -Destination (Join-Path $InstallDir "glance.exe")

  $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
  if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to your user PATH (restart your shell to pick it up)."
  }

  Write-Host ""
  Write-Host "Seamless Glance $Version installed to $dest"
  Write-Host "Run: seamless-glance   (or: glance)"
} finally {
  Remove-Item -Recurse -Force $Work
}
