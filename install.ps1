# TwoCents installer.
#
# Default (current user, %LOCALAPPDATA%\TwoCents):
#   irm https://raw.githubusercontent.com/Nucletheus/TwoCents/main/install.ps1 | iex
#
# Program Files (all users of this PC) - run in an ELEVATED PowerShell:
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/Nucletheus/TwoCents/main/install.ps1))) -Machine
#
# Everything is contained in the install folder: the executable and the
# `data` subfolder (the SQLite database) live side by side. Deleting the
# folder removes the app and its data.
param(
  [switch]$Machine
)

$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Repo = "Nucletheus/TwoCents"

if ($Machine) {
  $isElevated = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
  if (-not $isElevated) {
    throw "The -Machine option installs to Program Files and requires an elevated (Run as Administrator) PowerShell."
  }
  $Dest = Join-Path $env:ProgramFiles "TwoCents"
  $startMenu = Join-Path $env:ProgramData "Microsoft\Windows\Start Menu\Programs"
} else {
  $Dest = Join-Path $env:LOCALAPPDATA "TwoCents"
  $startMenu = [Environment]::GetFolderPath("Programs")
}

Write-Host "TwoCents installer" -ForegroundColor Cyan
Write-Host "Install location: $Dest"

$release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$asset = $release.assets | Where-Object { $_.name -like "*.zip" } | Select-Object -First 1
if (-not $asset) { throw "No .zip asset found on the latest release." }

Write-Host "Downloading $($asset.name) ($([math]::Round($asset.size/1MB, 1)) MB)..."
$tmpZip = Join-Path $env:TEMP $asset.name
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $tmpZip -UseBasicParsing

# Preserve the data folder across reinstalls and updates.
$dataDir = Join-Path $Dest "data"
$dataBackup = $null
if (Test-Path $dataDir) {
  $dataBackup = Join-Path $env:TEMP "TwoCents-data-backup"
  if (Test-Path $dataBackup) { Remove-Item $dataBackup -Recurse -Force }
  Move-Item $dataDir $dataBackup
}

if (Test-Path $Dest) { Remove-Item $Dest -Recurse -Force }
Expand-Archive -Path $tmpZip -DestinationPath $Dest -Force
Remove-Item $tmpZip

New-Item -ItemType Directory -Force -Path $dataDir | Out-Null
if ($dataBackup) {
  Move-Item "$dataBackup\*" $dataDir -Force
  Remove-Item $dataBackup -Force
}

# In the Program Files variant, standard users cannot write to the install
# folder, so grant write access to the data subfolder only (by SID, so this
# works on non-English Windows: *S-1-5-32-545 = BUILTIN\Users).
if ($Machine) {
  icacls $dataDir /grant "*S-1-5-32-545:(OI)(CI)M" | Out-Null
}

$exe = Get-ChildItem $Dest -Recurse -Filter "*.exe" | Where-Object { $_.Name -like "twocents*" } | Select-Object -First 1
if (-not $exe) { throw "Installed archive did not contain the TwoCents executable." }

$lnk = Join-Path $startMenu "TwoCents.lnk"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($lnk)
$shortcut.TargetPath = $exe.FullName
$shortcut.WorkingDirectory = $exe.DirectoryName
$shortcut.Save()

Write-Host ""
Write-Host "Installed to $Dest" -ForegroundColor Green
Write-Host "Your data lives in $dataDir"
Write-Host "Launch it from the Start Menu ('TwoCents') or run: $($exe.FullName)"
