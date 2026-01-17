@echo off
REM Compile amf_encode.cpp using MSVC

REM Set up MSVC environment
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

REM Set variables
set SOURCE_FILE=D:\workspace\rust\hwcodec\cpp\amf\amf_encode.cpp
set OUTPUT_FILE=amf_encode.obj
set COMMON_DIR=D:\workspace\rust\hwcodec\cpp\common
set WIN_DIR=%COMMON_DIR%\platform\win
set EXTERNALS_DIR=D:\workspace\rust\hwcodec\externals
set AMF_DIR=%EXTERNALS_DIR%\AMF_v1.4.35

REM Compile
cl.exe -nologo -MT -Z7 -Brepro ^
    -I "%COMMON_DIR%" ^
    -I "%WIN_DIR%" ^
    -I "%AMF_DIR%/amf/public/common" ^
    -I "%AMF_DIR%\amf" ^
    -W4 ^
    -DNOMINMAX ^
    -Fo"%OUTPUT_FILE%" ^
    -c "%SOURCE_FILE%"

if %ERRORLEVEL% EQU 0 (
    echo Compilation successful!
) else (
    echo Compilation failed with error code %ERRORLEVEL%
    exit /b %ERRORLEVEL%
)
