@echo off
REM Simple script to compile amf_encode.cpp

REM Set up MSVC environment
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

REM Compile with all necessary includes and flags
cl.exe -nologo -MT -Z7 -Brepro -EHsc ^
    -I "D:\workspace\rust\hwcodec\cpp\common" ^
    -I "D:\workspace\rust\hwcodec\cpp\common\platform\win" ^
    -I "D:\workspace\rust\hwcodec\externals\AMF_v1.4.35/amf/public/common" ^
    -I "D:\workspace\rust\hwcodec\externals\AMF_v1.4.35\amf" ^
    -W4 ^
    -DNOMINMAX ^
    -D_CRT_SECURE_NO_WARNINGS ^
    -Fo"amf_encode.obj" ^
    -c "D:\workspace\rust\hwcodec\cpp\amf\amf_encode.cpp"

if %ERRORLEVEL% EQU 0 (
    echo Compilation successful!
) else (
    echo Compilation failed with error code %ERRORLEVEL%
)

pause
