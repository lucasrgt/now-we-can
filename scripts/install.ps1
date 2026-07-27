$ErrorActionPreference = "Stop"

$target = "x86_64-pc-windows-msvc"
$destination = if ($env:NWC_INSTALL_DIR) { $env:NWC_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\nwc\bin" }
$archive = Join-Path $env:TEMP "nwc-$target.zip"
$extract = Join-Path $env:TEMP "nwc-$target"

New-Item -ItemType Directory -Force -Path $destination | Out-Null
Invoke-WebRequest "https://github.com/lucasrgt/now-we-can/releases/latest/download/nwc-$target.zip" -OutFile $archive
Remove-Item -Recurse -Force $extract -ErrorAction SilentlyContinue
Expand-Archive $archive $extract -Force
$binary = Get-ChildItem $extract -Recurse -Filter nwc.exe | Select-Object -First 1
Copy-Item $binary.FullName (Join-Path $destination "nwc.exe") -Force

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (($userPath -split ";") -notcontains $destination) {
    [Environment]::SetEnvironmentVariable("Path", (($userPath.TrimEnd(";") + ";" + $destination).TrimStart(";")), "User")
}

Remove-Item -Recurse -Force $extract
Remove-Item -Force $archive
Write-Output "Installed nwc to $destination\nwc.exe"
