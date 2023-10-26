@echo off
cargo build
pause

:: 请求管理员权限并运行 .exe 文件
powershell -Command "Start-Process 'target/debug/click_helper.exe' -Verb RunAs"
