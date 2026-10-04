# TwoCents installer (current user only, %LOCALAPPDATA%\TwoCents):
#   irm https://raw.githubusercontent.com/Nucletheus/TwoCents/main/install.ps1 | iex
#
# The install folder holds the executable and a `data` subfolder with the
# SQLite database. Updates replace only the application files; `data` is
# never moved, deleted or overwritten.
$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Repo = "Nucletheus/TwoCents"
$AssetName = "twocents-x86_64-pc-windows-msvc.zip"

$Dest = Join-Path $env:LOCALAPPDATA "TwoCents"
$startMenu = [Environment]::GetFolderPath("Programs")

Write-Host "TwoCents installer" -ForegroundColor Cyan
Write-Host "Install location: $Dest"

if (Get-Process -Name "twocents-desktop" -ErrorAction SilentlyContinue) {
  throw "TwoCents is running. Close it and run the installer again."
}

$release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$asset = $release.assets | Where-Object { $_.name -eq $AssetName } | Select-Object -First 1
if (-not $asset) { throw "Release $($release.tag_name) has no $AssetName asset." }

$tmpZip = Join-Path $env:TEMP ("TwoCents-" + [Guid]::NewGuid().ToString("N") + ".zip")
$stage = Join-Path $Dest ".new"

try {
  Write-Host "Downloading $($asset.name) ($([math]::Round($asset.size/1MB, 1)) MB)..."
  Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $tmpZip -UseBasicParsing

  if ($asset.digest) {
    $expected = ($asset.digest -replace '^sha256:', '').ToLowerInvariant()
    $actual = (Get-FileHash -Path $tmpZip -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw "Downloaded archive failed its SHA-256 checksum." }
  } else {
    Write-Warning "GitHub published no digest for this asset; skipping checksum verification."
  }

  if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
  Expand-Archive -Path $tmpZip -DestinationPath $stage -Force

  $stagedExe = Get-ChildItem $stage -Recurse -Filter "*.exe" |
    Where-Object { $_.Name -like "twocents*" } |
    Select-Object -First 1
  if (-not $stagedExe) { throw "Installed archive did not contain the TwoCents executable." }

  New-Item -ItemType Directory -Force -Path $Dest | Out-Null
  Copy-Item (Join-Path $stage "*") $Dest -Recurse -Force
} finally {
  if (Test-Path $tmpZip) { Remove-Item $tmpZip -Force }
  if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
}

New-Item -ItemType Directory -Force -Path (Join-Path $Dest "data") | Out-Null

$exe = Get-ChildItem $Dest -Recurse -Filter "*.exe" |
  Where-Object { $_.Name -like "twocents*" } |
  Select-Object -First 1
if (-not $exe) { throw "TwoCents executable is missing from $Dest." }

$lnk = Join-Path $startMenu "TwoCents.lnk"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($lnk)
$shortcut.TargetPath = $exe.FullName
$shortcut.WorkingDirectory = $exe.DirectoryName
$shortcut.IconLocation = "$($exe.FullName),0"
$shortcut.Save()

Write-Host ""
Write-Host "Installed $($release.tag_name) to $Dest" -ForegroundColor Green
Write-Host "Your data lives in $(Join-Path $Dest 'data')"
Write-Host "Launch it from the Start Menu ('TwoCents') or run: $($exe.FullName)"
