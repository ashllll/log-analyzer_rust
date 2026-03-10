
/// 错误码分类
class ErrorCodes {
  // 通用错误 (0-999)
  static const int unknown = 0;
  static const int networkError = 1;
  static const int timeout = 2;
  static const int invalidParams = 3;
  static const int notFound = 4;
  static const int unauthorized = 5;
  static const int ffiNotInitialized = 10;
  static const int ffiLoadFailed = 11;

  // 模块特定错误 (1000+)
  // 搜索模块 (1000-1099)
  static const int searchFailed = 1000;
  static const int searchCancelled = 1001;
  static const int searchTimeout = 1002;

  // 工作区模块 (1100-1199)
  static const int workspaceNotFound = 1100;
  static const int workspaceCreateFailed = 1101;
  static const int workspaceDeleteFailed = 1102;

  // 导入模块 (1200-1299)
  static const int importFailed = 1200;
  static const int importCancelled = 1201;

  // 文件监听模块 (1300-1399)
  static const int watchFailed = 1300;
}

/// 应用异常
class AppException implements Exception {
  final int code;
  final String message;
  final String? help;
  final dynamic originalError;

  const AppException({
    required this.code,
    required this.message,
    this.help,
    this.originalError,
  });

  String get displayMessage => '[E$code] $message';

  String? get solution => _getSolution(code);

  static String? _getSolution(int code) {
    switch (code) {
      case ErrorCodes.ffiNotInitialized:
        return '请重启应用程序';
      case ErrorCodes.ffiLoadFailed:
        return '请确保 Rust 后端已正确安装';
      case ErrorCodes.networkError:
        return '请检查网络连接';
      case ErrorCodes.timeout:
        return '操作超时，请重试';
      case ErrorCodes.workspaceNotFound:
        return '工作区不存在，请刷新列表';
      default:
        return null;
    }
  }
}

/// FFI 初始化异常
class FfiInitializationException implements Exception {
  final String message;

  FfiInitializationException(this.message);

  @override
  String toString() => 'FFI InitializationException: $message';
}
