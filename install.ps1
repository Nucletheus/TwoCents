# TwoCents one-line installer.
#   irm https://raw.githubusercontent.com/Nucletheus/TwoCents/main/install.ps1 | iex
# Downloads the latest release binary, installs to %LOCALAPPDATA%\TwoCents,
# and adds a TwoCents shortcut to the Start Menu.
$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Repo = "Nucletheus/TwoCents"
$Dest = Join-Path $env:LOCALAPPDATA "TwoCents"

Write-Host "TwoCents installer" -ForegroundColor Cyan

$release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$asset = $release.assets | Where-Object { $_.name -like "*.zip" } | Select-Object -First 1
if (-not $asset) { throw "No .zip asset found on the latest release." }

Write-Host "Downloading $($asset.name) ($([math]::Round($asset.size/1MB, 1)) MB)..."
$tmpZip = Join-Path $env:TEMP $asset.name
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $tmpZip -UseBasicParsing

if (Test-Path $Dest) { Remove-Item $Dest -Recurse -Force }
Expand-Archive -Path $tmpZip -DestinationPath $Dest -Force
Remove-Item $tmpZip

$exe = Get-ChildItem $Dest -Recurse -Filter "*.exe" | Where-Object { $_.Name -like "twocents*" } | Select-Object -First 1
if (-not $exe) { throw "Installed archive did not contain the TwoCents executable." }

$startMenu = [Environment]::GetFolderPath("Programs")
$lnk = Join-Path $startMenu "TwoCents.lnk"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($lnk)
$shortcut.TargetPath = $exe.FullName
$shortcut.WorkingDirectory = $exe.DirectoryName
$shortcut.Save()

Write-Host ""
Write-Host "Installed to $Dest" -ForegroundColor Green
Write-Host "Launch it from the Start Menu ('TwoCents') or run: $($exe.FullName)"
