$ttsCargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$ttsRoot = $PSScriptRoot
$ttsDist = Join-Path $ttsRoot 'dist'
$ttsMt = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Recurse -Filter 'mt.exe' |
    Sort-Object FullName -Descending |
    Select-Object -First 1 -ExpandProperty FullName

if (-not (Test-Path $ttsCargo)) { throw 'Rust toolchain not found.' }
if (-not $ttsMt) { throw 'mt.exe not found in Windows SDK.' }

& $ttsCargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Pick the largest exe in target/release (the real GUI binary).
$ttsExe = Get-ChildItem (Join-Path $ttsRoot 'target\release') -Filter '*.exe' |
    Sort-Object Length -Descending |
    Select-Object -First 1
if (-not $ttsExe) { throw 'Release exe not found.' }

# embed-resource does not reliably link RT_MANIFEST into the MSVC PE;
# inject app.manifest as resource ID 1 with mt.exe.
& $ttsMt '-manifest' (Join-Path $ttsRoot 'app.manifest') "-outputresource:$($ttsExe.FullName);#1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# dist/ is the single deliverable directory: wipe old versions, copy only the latest.
if (Test-Path $ttsDist) {
    Get-ChildItem $ttsDist -File | Remove-Item -Force
} else {
    New-Item -ItemType Directory -Path $ttsDist | Out-Null
}
Copy-Item $ttsExe.FullName (Join-Path $ttsDist $ttsExe.Name) -Force

Write-Host "Done: $(Join-Path $ttsDist $ttsExe.Name)"
