@echo off
REM Flutter Rust Bridge 代码生成脚本
REM 用于从 Rust 端生成 Dart 绑定代码

echo ========================================
echo Flutter Rust Bridge 代码生成
echo ========================================
echo.

REM 检查是否安装了 flutter_rust_bridge_codegen
where frb_codegen >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo [错误] 未找到 frb_codegen 工具
    echo.
    echo 请先安装 flutter_rust_bridge_codegen:
    echo   cargo install flutter_rust_bridge_codegen
    echo.
    pause
    exit /b 1
)

echo [1/3] 检查 Rust 端 FFI 模块...
if not exist "..\log-analyzer\src-tauri\src\ffi\bridge.rs" (
    echo [错误] 未找到 Rust FFI 模块
    pause
    exit /b 1
)
echo [OK] Rust FFI 模块存在
echo.

echo [2/3] 创建输出目录...
if not exist "lib\shared\services\generated" mkdir "lib\shared\services\generated"
echo [OK] 输出目录已准备
echo.

echo [3/3] 运行代码生成...
echo 执行命令: frb_codegen
echo.

REM 运行代码生成
flutter_rust_bridge_codegen generate ^
    --rust-input ../log-analyzer/src-tauri/src/ffi/bridge.rs ^
    --dart-output lib/shared/services/generated ^
    --dart-entrypoint-class-name LogAnalyzerBridge

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [错误] 代码生成失败，错误代码: %ERRORLEVEL%
    pause
    exit /b 1
)

echo.
echo ========================================
echo [成功] 代码生成完成！
echo ========================================
echo.
echo 生成的文件位于: lib\shared\services\generated\
echo.
echo 下一步:
echo   1. 运行 build_runner 生成 Freezed/Riverpod 代码:
echo      dart run build_runner build --delete-conflicting-outputs
echo   2. 运行 Flutter 应用测试 FFI 连接
echo.

pause
