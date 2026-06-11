@echo off
cd /d d:\Downloads\PasteBridge\crates\core
cargo check --color=never 1>d:\Downloads\PasteBridge\check_out.txt 2>&1
echo Done exit=%ERRORLEVEL% >> d:\Downloads\PasteBridge\check_out.txt