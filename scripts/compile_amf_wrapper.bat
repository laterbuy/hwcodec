@echo off
REM Compile amf_wrapper.cpp separately to see compilation errors

setlocal

REM Get the script directory and project root (one level up)
set SCRIPT_DIR=%~dp0
set WORKSPACE=%SCRIPT_DIR%..
cd /d "%WORKSPACE%"

set AMF_PATH=%WORKSPACE%\externals\AMF_v1.4.35
set CPP_DIR=%WORKSPACE%\cpp
set COMMON_DIR=%CPP_DIR%\common
set WIN_DIR=%COMMON_DIR%\platform\win
set AMF_DIR=%CPP_DIR%\amf

REM Setup Visual Studio environment
if exist "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" (
    call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
) else if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" (
    call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
) else if exist "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat" (
    call "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat"
) else (
    echo Error: Cannot find Visual Studio vcvars64.bat
    echo Please set up Visual Studio environment manually
    pause
    exit /b 1
)

REM Compiler options
set COMPILER=cl.exe
set FLAGS=/nologo /MT /Z7 /EHsc /W4 /DNOMINMAX /D_CRT_SECURE_NO_WARNINGS
set INCLUDES=/I"%COMMON_DIR%" /I"%WIN_DIR%" /I"%AMF_DIR%" /I"%AMF_PATH%/amf/public/common" /I"%AMF_PATH%\amf"
set SOURCE=%AMF_DIR%\amf_wrapper.cpp
set OUTPUT=amf_wrapper.obj

echo ========================================
echo Compiling amf_wrapper.cpp
echo ========================================
echo.
echo Workspace: %WORKSPACE%
echo Source: %SOURCE%
echo Output: %OUTPUT%
echo.
echo Compiler: %COMPILER%
echo Flags: %FLAGS%
echo.
echo Include paths:
echo   - %COMMON_DIR%
echo   - %WIN_DIR%
echo   - %AMF_DIR%
echo   - %AMF_PATH%/amf/public/common
echo   - %AMF_PATH%\amf
echo.

REM Compile
%COMPILER% %FLAGS% %INCLUDES% /c "%SOURCE%" /Fo"%OUTPUT%"

echo.
echo ========================================
if %ERRORLEVEL% EQU 0 (
    echo Compilation successful!
    echo Output: %OUTPUT%
) else (
    echo Compilation failed with error code %ERRORLEVEL%
    echo.
    echo Please check the error messages above.
)
echo ========================================
echo.

pause
