import 'package:flutter/material.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// 目录图标常量
const IconData directoryIcon = LucideIcons.folder;
const IconData directoryOpenIcon = LucideIcons.folderOpen;

/// 常用文件类型图标映射
///
/// 根据文件扩展名返回对应的图标
IconData getFileIcon(String fileName) {
  final ext = fileName.split('.').last.toLowerCase();

  switch (ext) {
    // 日志和文本文件
    case 'log':
    case 'txt':
    case 'text':
      return LucideIcons.fileText;

    // JSON 相关
    case 'json':
    case 'jsonc':
      return LucideIcons.braces;

    // XML 相关
    case 'xml':
    case 'html':
    case 'htm':
      return LucideIcons.code;

    // YAML 相关
    case 'yaml':
    case 'yml':
      return LucideIcons.fileJson;

    // Markdown
    case 'md':
    case 'markdown':
      return LucideIcons.fileText;

    // CSV 和数据文件
    case 'csv':
    case 'tsv':
      return LucideIcons.table;

    // 代码文件
    case 'js':
    case 'jsx':
      return LucideIcons.fileJson;
    case 'ts':
    case 'tsx':
      return LucideIcons.fileJson;
    case 'py':
      return LucideIcons.fileJson;
    case 'rs':
      return LucideIcons.fileJson;
    case 'go':
      return LucideIcons.fileJson;
    case 'java':
      return LucideIcons.fileJson;
    case 'c':
    case 'cpp':
    case 'h':
    case 'hpp':
      return LucideIcons.fileJson;

    // 配置文件
    case 'toml':
    case 'ini':
    case 'conf':
    case 'config':
    case 'properties':
      return LucideIcons.settings;

    // 压缩文件
    case 'zip':
      return LucideIcons.archive;
    case 'tar':
      return LucideIcons.archive;
    case 'gz':
    case 'gzip':
      return LucideIcons.archive;
    case 'rar':
      return LucideIcons.archive;
    case '7z':
      return LucideIcons.archive;
    case 'bz2':
      return LucideIcons.archive;

    // 图片文件
    case 'jpg':
    case 'jpeg':
    case 'png':
    case 'gif':
    case 'bmp':
    case 'svg':
    case 'webp':
      return LucideIcons.image;

    // 文档文件
    case 'pdf':
      return LucideIcons.fileText;
    case 'doc':
    case 'docx':
      return LucideIcons.fileText;
    case 'xls':
    case 'xlsx':
      return LucideIcons.table;
    case 'ppt':
    case 'pptx':
      return LucideIcons.presentation;

    // 音频/视频
    case 'mp3':
    case 'wav':
    case 'flac':
    case 'ogg':
      return LucideIcons.music;
    case 'mp4':
    case 'avi':
    case 'mkv':
    case 'mov':
      return LucideIcons.video;

    // 可执行文件
    case 'exe':
    case 'msi':
    case 'dmg':
    case 'app':
    case 'deb':
    case 'rpm':
      return LucideIcons.appWindow;

    // Shell 脚本
    case 'sh':
    case 'bash':
    case 'zsh':
    case 'bat':
    case 'ps1':
      return LucideIcons.terminal;

    // 证书和密钥
    case 'pem':
    case 'key':
    case 'crt':
    case 'cer':
    case 'p12':
    case 'pfx':
      return LucideIcons.key;

    // 默认文件图标
    default:
      return LucideIcons.file;
  }
}

/// 获取文件图标的颜色
///
/// 根据文件类型返回不同的颜色
Color getFileIconColor(String fileName, {bool isDirectory = false}) {
  if (isDirectory) {
    return const Color(0xFFFFB74D); // 琥珀色
  }

  final ext = fileName.split('.').last.toLowerCase();

  switch (ext) {
    // 日志和文本 - 蓝色
    case 'log':
    case 'txt':
    case 'text':
    case 'md':
    case 'markdown':
      return const Color(0xFF42A5F5);

    // JSON/YAML - 紫色
    case 'json':
    case 'jsonc':
    case 'yaml':
    case 'yml':
      return const Color(0xFFAB47BC);

    // 代码文件 - 绿色
    case 'js':
    case 'jsx':
    case 'ts':
    case 'tsx':
    case 'py':
    case 'rs':
    case 'go':
    case 'java':
    case 'c':
    case 'cpp':
    case 'h':
    case 'hpp':
      return const Color(0xFF66BB6A);

    // 压缩文件 - 橙色
    case 'zip':
    case 'tar':
    case 'gz':
    case 'gzip':
    case 'rar':
    case '7z':
    case 'bz2':
      return const Color(0xFFFF7043);

    // 图片 - 粉色
    case 'jpg':
    case 'jpeg':
    case 'png':
    case 'gif':
    case 'bmp':
    case 'svg':
    case 'webp':
      return const Color(0xFFEC407A);

    // 文档 - 红色
    case 'pdf':
    case 'doc':
    case 'docx':
      return const Color(0xFFEF5350);

    // 默认 - 灰色
    default:
      return const Color(0xFF9E9E9E);
  }
}
