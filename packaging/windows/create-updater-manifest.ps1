[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Version,

    [Parameter(Mandatory = $true)]
    [string]$InstallerPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$signaturePath = "$InstallerPath.sig"
if (-not (Test-Path -LiteralPath $signaturePath -PathType Leaf)) {
    throw 'The signed updater installer was not found.'
}

$releaseTag = "v$Version"
$directory = [System.IO.Path]::GetDirectoryName([System.IO.Path]::GetFullPath($InstallerPath))
& node (Join-Path $PSScriptRoot '..\updater\manifest.mjs') fragment `
    --platform windows `
    --tag $releaseTag `
    --version $Version `
    --dir $directory `
    --out $OutputPath
if ($LASTEXITCODE -ne 0) {
    throw 'Could not create the Windows updater manifest fragment.'
}
