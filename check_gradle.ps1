$g = Get-Command gradle -ErrorAction SilentlyContinue
if ($g) {
  Write-Output "global gradle: $($g.Source)"
  & gradle --version 2>&1 | Select-Object -First 8
} else {
  Write-Output "no global gradle"
}
Write-Output '---'
# Try common locations
$candidates = @(
  "C:\Program Files\Gradle\gradle-*\bin\gradle.exe",
  "C:\gradle\gradle-*\bin\gradle.exe",
  "$env:USERPROFILE\.gradle\gradle-*\bin\gradle.exe",
  "$env:LOCALAPPDATA\Programs\gradle\gradle-*\bin\gradle.exe"
)
foreach ($p in $candidates) {
  $found = Get-Item $p -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($found) { Write-Output "candidate: $($found.FullName)" }
}