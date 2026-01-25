@echo off
REM Compile all C++ files for hwcodec project

REM Get the script directory and project root (one level up)
set SCRIPT_DIR=%~dp0
set WORKSPACE=%SCRIPT_DIR%..
cd /d "%WORKSPACE%"

REM Set up MSVC environment
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

REM Set base directories
set COMMON_DIR=%WORKSPACE%\cpp\common
set WIN_DIR=%COMMON_DIR%\platform\win
set EXTERNALS_DIR=%WORKSPACE%\externals
set AMF_DIR=%EXTERNALS_DIR%\AMF_v1.4.35
set NV_DIR=%WORKSPACE%\cpp\nv
set MFX_DIR=%WORKSPACE%\cpp\mfx
set NV_HEADERS=%EXTERNALS_DIR%\nv-codec-headers_n12.1.14.0\include\ffnvcodec
set NV_SDK=%EXTERNALS_DIR%\Video_Codec_SDK_12.1.14
set MFX_SDK=%EXTERNALS_DIR%\MediaSDK_22.5.4

REM Common compiler flags
set COMMON_FLAGS=-nologo -MT -Z7 -Brepro -EHsc -W4 -DNOMINMAX -D_CRT_SECURE_NO_WARNINGS

REM Common include paths
set COMMON_INCLUDES=^
    -I "%COMMON_DIR%" ^
    -I "%WIN_DIR%"

echo Compiling common files...
cl.exe %COMMON_FLAGS% %COMMON_INCLUDES% -Fo"log.obj" -c "%COMMON_DIR%\log.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %COMMON_FLAGS% %COMMON_INCLUDES% -Fo"util.obj" -c "%COMMON_DIR%\util.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %COMMON_FLAGS% %COMMON_INCLUDES% -Fo"win.obj" -c "%WIN_DIR%\win.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

echo Compiling AMF files...
set AMF_INCLUDES=%COMMON_INCLUDES% ^
    -I "%AMF_DIR%/amf/public/common" ^
    -I "%AMF_DIR%\amf"

cl.exe %COMMON_FLAGS% %AMF_INCLUDES% -Fo"amf_encode.obj" -c "%WORKSPACE%\cpp\amf\amf_encode.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %COMMON_FLAGS% %AMF_INCLUDES% -Fo"amf_decode.obj" -c "%WORKSPACE%\cpp\amf\amf_decode.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %COMMON_FLAGS% %AMF_INCLUDES% -Fo"amf_common.obj" -c "%WORKSPACE%\cpp\amf\amf_common.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

echo Compiling NV files...
set NV_INCLUDES=%COMMON_INCLUDES% ^
    -I "%NV_HEADERS%" ^
    -I "%NV_SDK%" ^
    -I "%NV_SDK%\Interface" ^
    -I "%NV_SDK%\Samples\Utils" ^
    -I "%NV_SDK%\Samples\NvCodec" ^
    -I "%NV_SDK%\Samples\NvCodec\NVEncoder" ^
    -I "%NV_SDK%\Samples\NvCodec\NVDecoder"

cl.exe %COMMON_FLAGS% %NV_INCLUDES% -Fo"nv_encode.obj" -c "%NV_DIR%\nv_encode.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %COMMON_FLAGS% %NV_INCLUDES% -Fo"nv_decode.obj" -c "%NV_DIR%\nv_decode.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

echo Compiling MFX files...
set MFX_INCLUDES=%COMMON_INCLUDES% ^
    -I "%MFX_SDK%\api\mfx_dispatch\windows\include" ^
    -I "%MFX_SDK%\api\include" ^
    -I "%MFX_SDK%\samples\sample_common\include"

set MFX_FLAGS=%COMMON_FLAGS% -DMFX_DEPRECATED_OFF -DMFX_D3D11_SUPPORT -D_CRT_SECURE_NO_WARNINGS

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"mfx_encode.obj" -c "%MFX_DIR%\mfx_encode.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"mfx_decode.obj" -c "%MFX_DIR%\mfx_decode.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

echo Compiling MFX SDK sample files...
set SAMPLE_PATH=%MFX_SDK%\samples\sample_common\src
set VM_PATH=%SAMPLE_PATH%\vm

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"sample_utils.obj" -c "%SAMPLE_PATH%\sample_utils.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"base_allocator.obj" -c "%SAMPLE_PATH%\base_allocator.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"d3d11_allocator.obj" -c "%SAMPLE_PATH%\d3d11_allocator.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"avc_bitstream.obj" -c "%SAMPLE_PATH%\avc_bitstream.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"avc_spl.obj" -c "%SAMPLE_PATH%\avc_spl.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"avc_nal_spl.obj" -c "%SAMPLE_PATH%\avc_nal_spl.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"time.obj" -c "%VM_PATH%\time.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"atomic.obj" -c "%VM_PATH%\atomic.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"shared_object.obj" -c "%VM_PATH%\shared_object.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %MFX_FLAGS% %MFX_INCLUDES% -Fo"thread_windows.obj" -c "%VM_PATH%\thread_windows.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

echo Compiling NV SDK files...
set NV_SDK_SRC=%NV_SDK%\Samples\NvCodec\NvEncoder

cl.exe %COMMON_FLAGS% %NV_INCLUDES% -Fo"NvEncoder.obj" -c "%NV_SDK_SRC%\NvEncoder.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

cl.exe %COMMON_FLAGS% %NV_INCLUDES% -Fo"NvEncoderD3D11.obj" -c "%NV_SDK_SRC%\NvEncoderD3D11.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

set NV_SDK_SRC=%NV_SDK%\Samples\NvCodec\NvDecoder
cl.exe %COMMON_FLAGS% %NV_INCLUDES% -Fo"NvDecoder.obj" -c "%NV_SDK_SRC%\NvDecoder.cpp"
if %ERRORLEVEL% NEQ 0 goto :error

echo.
echo ========================================
echo All files compiled successfully!
echo ========================================
goto :end

:error
echo.
echo ========================================
echo Compilation failed!
echo ========================================
exit /b %ERRORLEVEL%

:end
