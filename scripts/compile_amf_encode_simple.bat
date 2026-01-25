@echo off
REM Simple script to compile amf_encode.cpp

REM Get the script directory and project root (one level up)
set SCRIPT_DIR=%~dp0
set WORKSPACE=%SCRIPT_DIR%..
cd /d "%WORKSPACE%"

REM Set up MSVC environment
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

REM Compile with all necessary includes and flags
cl.exe -nologo -MT -Z7 -Brepro -EHsc ^
    -I "%WORKSPACE%\cpp\common" ^
    -I "%WORKSPACE%\cpp\common\platform\win" ^
    -I "%WORKSPACE%\externals\AMF_v1.4.35/amf/public/common" ^
    -I "%WORKSPACE%\externals\AMF_v1.4.35\amf" ^
    -W4 ^
    -DNOMINMAX ^
    -D_CRT_SECURE_NO_WARNINGS ^
    -Fo"amf_encode.obj" ^
    -c "%WORKSPACE%\cpp\amf\amf_encode.cpp"

if %ERRORLEVEL% EQU 0 (
    echo Compilation successful!
) else (
    echo Compilation failed with error code %ERRORLEVEL%
)

pause
