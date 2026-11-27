param([Parameter(Mandatory)][string]$Token)

$ErrorActionPreference = 'Stop'
$version = '2.338.2'
$dir = "$HOME\actions-runner-greyscreen"

foreach ($id in 'Git.Git', 'GitHub.cli', 'Gyan.FFmpeg') {
    winget list --id $id --exact --accept-source-agreements | Out-Null
    if ($LASTEXITCODE -ne 0) { winget install --id $id --exact --silent --accept-package-agreements --accept-source-agreements }
}

$git = "$env:ProgramFiles\Git"
$machine = [Environment]::GetEnvironmentVariable('Path', 'Machine')
$user = [Environment]::GetEnvironmentVariable('Path', 'User')
$env:Path = "$git\bin;$git\cmd;$HOME\.cargo\bin;$machine;$user"
if (-not (Get-Command cargo-nextest -ErrorAction SilentlyContinue)) { cargo install cargo-nextest --locked }
foreach ($tool in 'bash', 'git', 'gh', 'cargo', 'ffmpeg', 'ffprobe') {
    $found = (Get-Command $tool -ErrorAction Stop).Source
    Write-Host "$tool -> $found"
}
if ((Get-Command bash).Source -notlike "$git*") { throw 'bash does not resolve to Git Bash' }

New-Item -ItemType Directory -Force $dir | Out-Null
Set-Location $dir
if (-not (Test-Path .\config.cmd)) {
    $zip = "actions-runner-win-x64-$version.zip"
    Invoke-WebRequest "https://github.com/actions/runner/releases/download/v$version/$zip" -OutFile $zip
    Expand-Archive $zip -DestinationPath . -Force
    Remove-Item $zip
}
.\config.cmd --unattended --replace --url https://github.com/andresmarpz/greyscreen --token $Token --name greyscreen-pc --labels greyscreen,windows --work _work
Set-Content -Path .\.path -Value $env:Path -NoNewline

$action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -WindowStyle Hidden -Command `"Set-Location '$dir'; .\run.cmd`""
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit 0 -RestartCount 99 -RestartInterval (New-TimeSpan -Minutes 1) -Priority 7
Register-ScheduledTask -TaskName 'Greyscreen runner' -Action $action -Trigger $trigger -Settings $settings -Force | Out-Null
Start-ScheduledTask -TaskName 'Greyscreen runner'
Write-Host 'Runner registered and started. It starts again at every logon.'
