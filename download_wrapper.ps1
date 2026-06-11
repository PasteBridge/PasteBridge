$wrapperDir = 'd:\Downloads\PasteBridge\crates\mobile\gradle\wrapper'
$wrapperJar = Join-Path $wrapperDir 'gradle-wrapper.jar'
$wrapperProps = Get-Content (Join-Path $wrapperDir 'gradle-wrapper.properties')
$distributionUrl = ($wrapperProps | Select-String 'distributionUrl').ToString().Split('=', 2)[1].Trim()
Write-Output "distributionUrl = $distributionUrl"

# Extract gradle version from URL
$gradleVersion = ($distributionUrl -split 'gradle-')[2] -split '-bin\.zip$'
$gradleVersion = $gradleVersion -replace '[\.\-]', ''
Write-Output "Looking for gradle version: $gradleVersion"

# Try downloading a recent gradle distribution
$candidates = @(
    'https://services.gradle.org/distributions/gradle-8.13-bin.zip',
    'https://services.gradle.org/distributions/gradle-8.10-bin.zip',
    'https://services.gradle.org/distributions/gradle-8.7-bin.zip'
)

$zipPath = "$env:TEMP\gradle-dl.zip"
foreach ($url in $candidates) {
    Write-Output "Trying $url ..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing -TimeoutSec 120 -ErrorAction Stop
        Write-Output "Downloaded $url"
        break
    } catch {
        Write-Output "Failed: $($_.Exception.Message)"
    }
}

if (-not (Test-Path $zipPath)) { Write-Error "all downloads failed"; exit 2 }

# Extract gradle-launcher jar and use it to bootstrap wrapper jar
Add-Type -AssemblyName System.IO.Compression.FileSystem
$extractDir = "$env:TEMP\gradle-extract"
if (Test-Path $extractDir) { Remove-Item $extractDir -Recurse -Force }
[System.IO.Compression.ZipFile]::ExtractToDirectory($zipPath, $extractDir)
$launcherJar = Get-ChildItem -Path $extractDir -Recurse -Filter 'gradle-wrapper-*.jar' -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $launcherJar) {
    $launcherJar = Get-ChildItem -Path $extractDir -Recurse -Filter 'gradle-launcher-*.jar' -ErrorAction SilentlyContinue | Select-Object -First 1
}
Write-Output "found jar: $($launcherJar.FullName)"

# Generate the wrapper using gradle's built-in wrapper task
$gradleBin = Get-ChildItem -Path $extractDir -Recurse -Filter 'gradle.bat' | Select-Object -First 1
Write-Output "gradle bin: $($gradleBin.FullName)"