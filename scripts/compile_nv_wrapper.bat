@echo off
REM Compile nv_wrapper.cpp separately to see compilation errors

setlocal

REM Get the script directory and project root (one level up)
set SCRIPT_DIR=%~dp0
set WORKSPACE=%SCRIPT_DIR%..
cd /d "%WORKSPACE%"

set NVSDK_PATH=%WORKSPACE%\externals\Video_Codec_SDK_12.1.14
set NVCODEC_HEADERS=%WORKSPACE%\externals\nv-codec-headers_n12.1.14.0\include\ffnvcodec
set CPP_DIR=%WORKSPACE%\cpp
set COMMON_DIR=%CPP_DIR%\common
set WIN_DIR=%COMMON_DIR%\platform\win
set NV_DIR=%CPP_DIR%\nv

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
set FLAGS=/nologo /MT /Z7 /EHsc /W4 /utf-8 /DNOMINMAX /D_CRT_SECURE_NO_WARNINGS
set INCLUDES=/I"%COMMON_DIR%" /I"%WIN_DIR%" /I"%NV_DIR%" /I"%NVCODEC_HEADERS%" /I"%NVSDK_PATH%" /I"%NVSDK_PATH%\Interface" /I"%NVSDK_PATH%\Samples\Utils" /I"%NVSDK_PATH%\Samples\NvCodec" /I"%NVSDK_PATH%\Samples\NvCodec\NVEncoder" /I"%NVSDK_PATH%\Samples\NvCodec\NVDecoder"
set SOURCE=%NV_DIR%\nv_wrapper.cpp
set OUTPUT=nv_wrapper.obj

echo ========================================
echo Compiling nv_wrapper.cpp
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
echo   - %NV_DIR%
echo   - %NVCODEC_HEADERS%
echo   - %NVSDK_PATH%
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
