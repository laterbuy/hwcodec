@echo off
REM Compile mfx_wrapper.cpp separately to see compilation errors

setlocal

REM Get the script directory and project root (one level up)
set SCRIPT_DIR=%~dp0
set WORKSPACE=%SCRIPT_DIR%..
cd /d "%WORKSPACE%"

set MEDIASDK_PATH=%WORKSPACE%\externals\MediaSDK_22.5.4
set CPP_DIR=%WORKSPACE%\cpp
set COMMON_DIR=%CPP_DIR%\common
set WIN_DIR=%COMMON_DIR%\platform\win
set MFX_DIR=%CPP_DIR%\mfx

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
set FLAGS=/nologo /MT /Z7 /EHsc /W4 /utf-8 /DNOMINMAX /DMFX_DEPRECATED_OFF /DMFX_D3D11_SUPPORT /D_CRT_SECURE_NO_WARNINGS
set INCLUDES=/I"%COMMON_DIR%" /I"%WIN_DIR%" /I"%MFX_DIR%" /I"%MEDIASDK_PATH%\api\include" /I"%MEDIASDK_PATH%\samples\sample_common\include"
set SOURCE=%MFX_DIR%\mfx_wrapper.cpp
set OUTPUT=mfx_wrapper.obj

echo ========================================
echo Compiling mfx_wrapper.cpp
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
echo   - %MFX_DIR%
echo   - %MEDIASDK_PATH%\api\include
echo   - %MEDIASDK_PATH%\samples\sample_common\include
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
