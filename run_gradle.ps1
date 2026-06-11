Set-Location 'd:\Downloads\PasteBridge\crates\mobile'
$env:JAVA_HOME = $env:JAVA_HOME
$proc = Start-Process -FilePath '.\gradlew.bat' -ArgumentList @(':shared:compileDebugKotlinAndroid','--info','--stacktrace') -RedirectStandardOutput 'd:\Downloads\PasteBridge\gradle_out.txt' -RedirectStandardError 'd:\Downloads\PasteBridge\gradle_err.txt' -PassThru -NoNewWindow -WorkingDirectory 'd:\Downloads\PasteBridge\crates\mobile'
Write-Output "Started gradle pid=$($proc.Id), waiting..."
$exited = $proc.WaitForExit(600000) # 10 min
if (-not $exited) {
    Write-Output 'Gradle still running after 10min, killing'
    Stop-Process -Id $proc.Id -Force
}
Write-Output "Gradle exited. Code=$($proc.ExitCode)"