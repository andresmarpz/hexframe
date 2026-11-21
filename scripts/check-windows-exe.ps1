param([Parameter(Mandatory)][string]$Dir)

$ErrorActionPreference = 'Stop'
$vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
$dumpbin = Get-ChildItem "$vs\VC\Tools\MSVC" -Recurse -Filter dumpbin.exe | Where-Object { $_.FullName -like '*Hostx64\x64*' } | Select-Object -First 1 -ExpandProperty FullName
if (-not $dumpbin) { throw 'dumpbin not found' }
foreach ($exe in 'greyscreen.exe', 'greyscreen-cli.exe') {
    $deps = (& $dumpbin /dependents "$Dir/$exe") -join "`n"
    Write-Host $deps
    if ($deps -match 'VCRUNTIME|MSVCP|api-ms-win-crt') { throw "$exe imports the C runtime dynamically" }
}
$gui = (& $dumpbin /headers "$Dir/greyscreen.exe") -join "`n"
if ($gui -notmatch 'subsystem \(Windows GUI\)') { throw 'greyscreen.exe is not a GUI subsystem exe' }
$cui = (& $dumpbin /headers "$Dir/greyscreen-cli.exe") -join "`n"
if ($cui -notmatch 'subsystem \(Windows CUI\)') { throw 'greyscreen-cli.exe is not a console subsystem exe' }
