@echo off
set fdir=release
if exist %fdir% (
	echo.
	echo ### Delete old release files
	rd %fdir% /s/q
)
if not exist %fdir% (
	echo.
	echo ### Making Directory release/
	md %fdir%
)
echo.
echo.
echo ### Cargo building from source code...
cargo build --release
echo.
echo.

xcopy "target\release\click_helper.exe" %fdir% /f/c
xcopy "configs.json" %fdir% /f/c
xcopy "res\*.*" "%fdir%\res\" /s/h/e/k/f/c

echo @echo off> release\run.bat
echo :: 请求管理员权限并运行 .exe 文件>> release\run.bat
echo powershell -Command "Start-Process 'click_helper.exe' -Verb RunAs">> release\run.bat

pause
