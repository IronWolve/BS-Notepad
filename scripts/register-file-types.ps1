<#
    Associates markdown files with this app for the current user, so
    double-clicking one opens it.

    Run it yourself from Windows PowerShell:
        powershell -ExecutionPolicy Bypass -File register-file-types.ps1

    Writes only under HKCU. Nothing system wide, no administrator rights.
    Pass -Remove to undo it.
#>
param(
    [string]$Exe = (Join-Path $PSScriptRoot "notepad.exe"),
    [string[]]$Extensions = @(".md", ".markdown", ".mdown", ".mkd", ".mkdn"),
    [switch]$Remove
)

$progId = "Notepad.Markdown"
$classes = "HKCU:\Software\Classes"

if ($Remove) {
    Remove-Item -Path "$classes\$progId" -Recurse -ErrorAction SilentlyContinue
    foreach ($ext in $Extensions) {
        Remove-Item -Path "$classes\$ext\OpenWithProgids" -ErrorAction SilentlyContinue
    }
    Write-Host "Associations removed." -ForegroundColor Green
    return
}

if (-not (Test-Path $Exe)) {
    Write-Host "Cannot find the program at $Exe" -ForegroundColor Red
    Write-Host "Pass -Exe with the full path." -ForegroundColor Yellow
    exit 1
}
$Exe = (Resolve-Path $Exe).Path

New-Item -Path "$classes\$progId\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path "$classes\$progId" -Name "(default)" -Value "Markdown document"
Set-ItemProperty -Path "$classes\$progId\shell\open\command" -Name "(default)" -Value "`"$Exe`" `"%1`""
New-Item -Path "$classes\$progId\DefaultIcon" -Force | Out-Null
Set-ItemProperty -Path "$classes\$progId\DefaultIcon" -Name "(default)" -Value "`"$Exe`",0"

foreach ($ext in $Extensions) {
    New-Item -Path "$classes\$ext\OpenWithProgids" -Force | Out-Null
    Set-ItemProperty -Path "$classes\$ext\OpenWithProgids" -Name $progId -Value ([byte[]]@()) -Type Binary
    Write-Host ("  {0,-12} -> {1}" -f $ext, $progId) -ForegroundColor Cyan
}

Write-Host ""
Write-Host "Registered for the current user." -ForegroundColor Green
Write-Host "Right-click a markdown file, Open with, Choose another app, and pick it once." -ForegroundColor Gray
