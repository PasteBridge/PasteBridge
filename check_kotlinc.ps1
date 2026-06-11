$kotlinc = Get-Command kotlinc -ErrorAction SilentlyContinue
if ($kotlinc) { Write-Output "kotlinc: $($kotlinc.Source)" } else { Write-Output "kotlinc not in PATH" }
$java = Get-Command java -ErrorAction SilentlyContinue
if ($java) { Write-Output "java: $($java.Source)" } else { Write-Output "java not in PATH" }