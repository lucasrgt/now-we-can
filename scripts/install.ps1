$ErrorActionPreference = "Stop"

$target = "x86_64-pc-windows-msvc"
$destination = if ($env:NOTYET_INSTALL_DIR) { $env:NOTYET_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\notyet\bin" }
$archive = Join-Path $env:TEMP "notyet-$target.zip"
$extract = Join-Path $env:TEMP "notyet-$target"

New-Item -ItemType Directory -Force -Path $destination | Out-Null
Invoke-WebRequest "https://github.com/lucasrgt/not-yet/releases/latest/download/notyet-$target.zip" -OutFile $archive
Remove-Item -Recurse -Force $extract -ErrorAction SilentlyContinue
Expand-Archive $archive $extract -Force
$binary = Get-ChildItem $extract -Recurse -Filter notyet.exe | Select-Object -First 1
Copy-Item $binary.FullName (Join-Path $destination "notyet.exe") -Force

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (($userPath -split ";") -notcontains $destination) {
    [Environment]::SetEnvironmentVariable("Path", (($userPath.TrimEnd(";") + ";" + $destination).TrimStart(";")), "User")
}

Remove-Item -Recurse -Force $extract
Remove-Item -Force $archive
Write-Output "Installed notyet to $destination\notyet.exe"
