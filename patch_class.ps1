$lines = [System.IO.File]::ReadAllLines(".\crates\mobile\shared\src\androidMain\kotlin\uniffi\paste_bridge_core\paste_bridge_core.kt")
$lines[2086] = '    ) : PasteBridgeException() {'
$lines[2087] = '        // 跳过 override val message 避免与字段名同名冲突'
$lines[2088] = '        val messageAsString: kotlin.String get() = "message=" + message'
$lines[2089] = '    }'
[System.IO.File]::WriteAllLines(".\crates\mobile\shared\src\androidMain\kotlin\uniffi\paste_bridge_core\paste_bridge_core.kt", $lines)
for ($i=2082; $i -le 2095; $i++) { Write-Host ($i+1).ToString() + ': ' + $lines[$i] }
