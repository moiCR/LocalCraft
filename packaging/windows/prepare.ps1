[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$workspaceRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Push-Location -LiteralPath $workspaceRoot
try {
    $buildMessages = & cargo build --release --locked -p LocalCraft --message-format=json
    if ($LASTEXITCODE -ne 0) {
        throw 'Could not build the LocalCraft release executable.'
    }

    $iconBuild = $buildMessages |
        ForEach-Object { ConvertFrom-Json -InputObject $_ } |
        Where-Object {
            $_.reason -eq 'build-script-executed' -and $_.package_id -match '#LocalCraft@'
        } |
        Select-Object -Last 1
    if ($null -eq $iconBuild) {
        throw 'Could not locate the LocalCraft icon build output.'
    }

    $toolchainInfo = & rustc -vV
    if ($LASTEXITCODE -ne 0) {
        throw 'Could not read the Rust toolchain target.'
    }
    $hostLine = $toolchainInfo | Where-Object { $_ -like 'host: *' } | Select-Object -First 1
    if ($null -eq $hostLine) {
        throw 'The Rust toolchain did not report a host target.'
    }
    $targetTriple = $hostLine.Substring('host: '.Length)
    if ($env:CARGO_BUILD_TARGET) {
        $targetTriple = $env:CARGO_BUILD_TARGET
    }
    $runtimeArch = switch ($targetTriple) {
        'x86_64-pc-windows-msvc' { 'x64' }
        'i686-pc-windows-msvc' { 'x86' }
        'aarch64-pc-windows-msvc' { 'arm64' }
        default { throw "Unsupported Windows packaging target: $targetTriple" }
    }

    $vswherePath = Join-Path ([Environment]::GetFolderPath('ProgramFilesX86')) 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswherePath -PathType Leaf)) {
        throw 'Visual Studio Installer is required to locate the Visual C++ redistributable.'
    }
    $runtimeCandidates = & $vswherePath -latest -products '*' -find "VC\Redist\MSVC\**\$runtimeArch\Microsoft.VC*.CRT\vcruntime140.dll"
    if ($LASTEXITCODE -ne 0) {
        throw 'Could not locate the Visual C++ redistributable.'
    }
    $runtimePath = $runtimeCandidates |
        Where-Object { $_ -notmatch '\\onecore\\' } |
        Sort-Object -Descending |
        Select-Object -First 1
    if ($null -eq $runtimePath) {
        throw "Install the Visual C++ $runtimeArch redistributable component in Visual Studio."
    }

    $assetsDir = Join-Path $workspaceRoot 'target\packager\windows-assets'
    New-Item -ItemType Directory -Path $assetsDir -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $iconBuild.out_dir 'localcraft.ico') -Destination (Join-Path $assetsDir 'localcraft.ico')
    Copy-Item -LiteralPath $runtimePath -Destination (Join-Path $assetsDir 'vcruntime140.dll')
    Write-Host "Prepared the Windows installer icon and $runtimeArch Visual C++ runtime."
}
finally {
    Pop-Location
}
