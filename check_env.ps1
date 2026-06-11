java -version 2>&1 | Select-Object -First 3
Write-Output '---'
$gradlePath = Get-Command gradle -ErrorAction SilentlyContinue
if ($gradlePath) { Write-Output "gradle: $($gradlePath.Source)" } else { Write-Output "gradle not in PATH" }
Write-Output '---'
$exists = Test-Path 'd:\Downloads\PasteBridge\crates\mobile\gradlew.bat'
Write-Output "gradlew.bat: $exists"