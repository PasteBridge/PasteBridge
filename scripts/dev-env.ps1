# PasteBridge dev env
$env:JAVA_HOME = "C:\Program Files\Java\jdk-22"
$env:Path = "$env:JAVA_HOME\bin;C:\Users\Administrator\.cargo\bin;C:\Users\Administrator\AppData\Local\Android\Sdk\platform-tools;$env:Path"
$env:GRADLE_USER_HOME = "$env:USERPROFILE\.gradle"
$env:GRADLE_HOME = "$env:GRADLE_USER_HOME\wrapper\dists\gradle-9.1.0-bin\9agqghryom9wkf8r80qlhnts3\gradle-9.1.0"

# gradlew wrapper jar 缺失,直接 java -jar launcher 跑
function global:gradle {
    $java = "$env:JAVA_HOME\bin\java.exe"
    $launcher = "$env:GRADLE_HOME\lib\gradle-launcher-9.1.0.jar"
    $agent = "$env:GRADLE_HOME\lib\agents\gradle-instrumentation-agent-9.1.0.jar"
    & $java -Xmx64m -Xms64m "-javaagent:$agent" -classpath "$launcher" org.gradle.launcher.GradleMain @args
}
