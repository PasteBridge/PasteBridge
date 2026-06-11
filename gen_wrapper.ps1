$gradleBin = 'C:\Users\Administrator\AppData\Local\Temp\gradle-extract\gradle-8.13\bin\gradle.bat'
Set-Location 'd:\Downloads\PasteBridge\crates\mobile'
# Use the standalone gradle to generate the wrapper (this writes gradle-wrapper.jar)
$proc = Start-Process -FilePath $gradleBin -ArgumentList @('wrapper','--gradle-version=8.13','--distribution-type=bin') -RedirectStandardOutput 'd:\Downloads\PasteBridge\gen_wrapper_out.txt' -RedirectStandardError 'd:\Downloads\PasteBridge\gen_wrapper_err.txt' -PassThru -NoNewWindow
$exited = $proc.WaitForExit(300000) # 5 min
if (-not $exited) { Stop-Process -Id $proc.Id -Force; Write-Output 'TIMEOUT' }
Write-Output "Exit code: $($proc.ExitCode)"