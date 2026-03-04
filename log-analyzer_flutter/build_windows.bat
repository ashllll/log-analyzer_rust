@echo off
REM Flutter Desktop Build Script for Windows
REM 自动化 Windows 版本打包流程

setlocal enableDelayedExpansion

REM 颜色定义
set RED=[31m
set GREEN=[32m
set YELLOW=[33m
set NC=[0m

echo ========================================
echo  Flutter Desktop Build Script
echo ========================================
echo.
echo  应用: Log Analyzer
echo.
echo ========================================
echo.

REM 检测 Flutter
where flutter >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo %RED%Flutter 未安装或不在 PATH 中%NC%
    echo.
    echo 请安装 Flutter SDK: https://flutter.dev/docs/get-started/install/windows
    pause
    exit /b 1
)

echo %GREEN%Flutter 环境检测通过%NC%

REM 进入项目目录（脚本已位于 Flutter 项目根目录）
cd /d "%~dp0"

echo %YELLOW%当前目录: %CD%%NC%
echo.

REM 显示菜单
:menu
echo ========================================
echo  选择构建选项:
echo ========================================
echo.
echo   1. 清理构建目录
echo   2. 生成代码
echo   3. 构建 Windows Release
echo   4. 完整构建^代码 + Release）
echo   5. 运行应用
echo   6. 退出
echo.
set /p choice="请输入选项 [1-6]: "

if "%choice%"=="" goto menu_done

if %choice%==1 goto clean
if %choice%==2 goto generate
if %choice%==3 goto build_release
if %choice%==4 goto full_build
if %choice%==5 goto run_app
if %choice%==6 goto exit_script

echo %RED%无效选项: %choice%%NC%
pause
goto menu

REM 清理构建目录
:clean
echo.
echo %YELLOW%清理构建目录...%NC%

if exist "build\windows" (
    rmdir /s /q "build\windows"
    echo %GREEN%✓ 构建目录已清理%NC%
)

echo.
pause
goto menu

REM 代码生成
:generate
echo.
echo %YELLOW%运行代码生成...%NC%

call dart run build_runner build --delete-conflicting-outputs

if %ERRORLEVEL% NEQ 0 (
    echo %RED%✗ 代码生成失败%NC%
) else (
    echo %GREEN%✓ 代码生成完成%NC%
)

echo.
pause
goto menu

REM 构建 Release
:build_release
echo.
echo %YELLOW%构建 Windows Release...%NC%

flutter build windows --release

if %ERRORLEVEL% EQU 0 (
    echo %GREEN%✓ Windows Release 构建成功%NC%
    echo.
    echo %YELLOW%发布包位置:%NC%
    echo %GREEN%build\windows\runner\Release\%NC%

    REM 检查是否存在可执行文件
    if exist "build\windows\runner\Release\log_analyzer_flutter.exe" (
        echo %GREEN%可执行文件: log_analyzer_flutter.exe%NC%
        echo.
    )
    else (
        echo %RED%未找到可执行文件%NC%
    )
) else (
    echo %RED%✗ Windows Release 构建失败%NC%
)

echo.
pause
goto menu

REM 完整构建
:full_build
echo.
echo %YELLOW%执行完整构建流程...%NC%
echo.

REM 1. 清理
if exist "build\windows" (
    rmdir /s /q "build\windows"
)

REM 2. 代码生成
call dart run build_runner build --delete-conflicting-outputs
if %ERRORLEVEL% NEQ 0 (
    echo %RED%✗ 代码生成失败，中止%NC%
    pause
    goto menu
)

REM 3. 构建
flutter build windows --release

if %ERRORLEVEL% EQU 0 (
    echo %GREEN%✓ 完整构建成功%NC%
) else (
    echo %RED%✗ 构建失败%NC%
)

echo.
pause
goto menu

REM 运行应用
:run_app
echo.
echo %YELLOW%构建并运行 Debug 版本...%NC%

flutter build windows --debug && build\windows\runner\Debug\log_analyzer_flutter.exe

if %ERRORLEVEL% EQU 0 (
    echo %GREEN%应用已启动%NC%
) else (
    echo %RED%启动失败%NC%
)

goto menu_done

REM 退出
:exit_script
echo.
echo %YELLOW%退出构建脚本%NC%
exit /b 0

:menu_done
echo.
echo %GREEN%构建脚本结束%NC%
timeout /t 3 >nul
