import 'dart:math' show max, pow;
import 'dart:typed_data';
import 'dart:ui'
    as ui
    show Image, FragmentProgram, ImmutableBuffer, ImageDescriptor, PixelFormat;

import 'package:flutter/material.dart';

/// 热力图缩略图组件
///
/// PRD V6.0 4.2 GPU 着色器缩略图
/// 使用 GLSL Fragment Shader 实现纳秒级热力图渲染
///
/// 核心特性:
/// - 将 Rust 端 density_map (Uint8List) 直接传递给 GPU
/// - 通过 FragmentProgram 在 GPU 端完成颜色计算
/// - 彻底释放主 Isolate CPU
class HeatmapMinimap extends StatefulWidget {
  /// 密度数据 (从 Rust 端传入的 Uint8List)
  ///
  /// 每个字节代表一行日志的命中密度 (0-255)
  final Uint8List? densityMap;

  /// 最大密度值 (用于归一化)
  final int maxDensity;

  /// 组件宽度
  final double width;

  /// 组件高度
  final double height;

  /// 点击回调 (返回点击位置对应的日志行索引)
  final void Function(int lineIndex)? onTap;

  const HeatmapMinimap({
    super.key,
    this.densityMap,
    this.maxDensity = 255,
    this.width = 20.0,
    this.height = 200.0,
    this.onTap,
  });

  @override
  State<HeatmapMinimap> createState() => _HeatmapMinimapState();
}

class _HeatmapMinimapState extends State<HeatmapMinimap> {
  /// FragmentProgram (编译后的着色器程序)
  ui.FragmentProgram? _fragmentProgram;

  /// 着色器加载状态
  bool _shaderLoaded = false;

  /// 密度纹理
  ui.Image? _densityTexture;

  @override
  void initState() {
    super.initState();
    _loadShader();
  }

  @override
  void didUpdateWidget(HeatmapMinimap oldWidget) {
    super.didUpdateWidget(oldWidget);
    // 当密度数据变化时，重新创建纹理
    if (widget.densityMap != oldWidget.densityMap) {
      _updateDensityTexture();
    }
  }

  @override
  void dispose() {
    _densityTexture?.dispose();
    super.dispose();
  }

  /// 加载 GLSL 着色器
  ///
  /// 使用 Flutter 的 FragmentProgram.fromAsset 加载预编译的着色器
  Future<void> _loadShader() async {
    try {
      // 尝试加载 GPU 着色器
      // 注意: FragmentProgram 需要 Flutter 3.0+ 和 Skia/Impeller 后端支持
      final program = await ui.FragmentProgram.fromAsset(
        'shaders/heatmap.frag',
      );
      setState(() {
        _fragmentProgram = program;
        _shaderLoaded = true;
      });

      // 初始化纹理
      _updateDensityTexture();
    } catch (e) {
      // 着色器加载失败，使用 CPU 回退渲染
      debugPrint('热力图着色器加载失败，使用 CPU 回退: $e');
      setState(() {
        _shaderLoaded = false;
      });
    }
  }

  /// 更新密度纹理
  ///
  /// 将 Uint8List densityMap 转换为 GPU 纹理
  Future<void> _updateDensityTexture() async {
    final densityMap = widget.densityMap;
    if (densityMap == null || densityMap.isEmpty) {
      _densityTexture?.dispose();
      _densityTexture = null;
      return;
    }

    // 创建 1xN 的纹理 (N = 密度数据长度)
    const width = 1;
    final height = densityMap.length;

    // 使用 RGBA 格式 (每个像素 4 字节)
    final pixels = Uint8List(width * height * 4);

    for (int i = 0; i < height; i++) {
      final density = densityMap[i];
      final offset = i * 4;
      // RGBA: 密度值放入 R 通道
      pixels[offset] = density; // R
      pixels[offset + 1] = density; // G (备用)
      pixels[offset + 2] = density; // B (备用)
      pixels[offset + 3] = 255; // A (完全不透明)
    }

    // 创建图像描述符
    final descriptor = ui.ImageDescriptor.raw(
      await ui.ImmutableBuffer.fromUint8List(pixels),
      width: width,
      height: height,
      pixelFormat: ui.PixelFormat.rgba8888,
    );

    // 编码图像
    final codec = await descriptor.instantiateCodec();
    final frame = await codec.getNextFrame();

    _densityTexture?.dispose();
    _densityTexture = frame.image;

    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final densityMap = widget.densityMap;

    // 无数据时显示占位符
    if (densityMap == null || densityMap.isEmpty) {
      return _buildPlaceholder();
    }

    // 优先使用 GPU 着色器渲染
    if (_shaderLoaded && _fragmentProgram != null && _densityTexture != null) {
      return _buildGpuHeatmap();
    }

    // CPU 回退渲染
    return _buildCpuHeatmap();
  }

  /// 构建占位符
  Widget _buildPlaceholder() {
    return Container(
      width: widget.width,
      height: widget.height,
      decoration: BoxDecoration(
        color: Colors.grey.withOpacity(0.1),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: Colors.grey.withOpacity(0.3), width: 1),
      ),
      child: const Center(
        child: Icon(Icons.minimize, size: 16, color: Colors.grey),
      ),
    );
  }

  /// 构建 GPU 热力图
  ///
  /// 使用 FragmentShader 进行 GPU 加速渲染
  Widget _buildGpuHeatmap() {
    return GestureDetector(
      onTapUp: _handleTap,
      child: CustomPaint(
        size: Size(widget.width, widget.height),
        painter: _GpuHeatmapPainter(
          fragmentProgram: _fragmentProgram!,
          densityTexture: _densityTexture!,
          maxDensity: widget.maxDensity.toDouble(),
        ),
      ),
    );
  }

  /// 构建 CPU 回退热力图
  ///
  /// 当 GPU 着色器不可用时使用
  Widget _buildCpuHeatmap() {
    return GestureDetector(
      onTapUp: _handleTap,
      child: CustomPaint(
        size: Size(widget.width, widget.height),
        painter: _CpuHeatmapPainter(
          densityMap: widget.densityMap!,
          maxDensity: widget.maxDensity,
        ),
      ),
    );
  }

  /// 处理点击事件
  void _handleTap(TapUpDetails details) {
    if (widget.onTap == null || widget.densityMap == null) return;

    // 计算点击位置对应的日志行索引
    final relativeY = details.localPosition.dy / widget.height;
    final lineIndex = (relativeY * widget.densityMap!.length).floor();

    widget.onTap!(lineIndex.clamp(0, widget.densityMap!.length - 1));
  }
}

/// GPU 热力图绘制器
///
/// 使用 FragmentShader 在 GPU 端渲染热力图
class _GpuHeatmapPainter extends CustomPainter {
  final ui.FragmentProgram fragmentProgram;
  final ui.Image densityTexture;
  final double maxDensity;

  _GpuHeatmapPainter({
    required this.fragmentProgram,
    required this.densityTexture,
    required this.maxDensity,
  });

  @override
  void paint(Canvas canvas, Size size) {
    // 创建 FragmentShader
    final shader = fragmentProgram.fragmentShader();

    // 设置 uniform 变量
    // 注意: 索引顺序对应 GLSL 中的布局
    shader.setImageSampler(0, densityTexture); // u_density_texture
    shader.setFloat(0, size.width); // u_resolution.x
    shader.setFloat(1, size.height); // u_resolution.y
    shader.setFloat(2, maxDensity); // u_max_density

    // 创建 Paint
    final paint = Paint()..shader = shader;

    // 绘制
    canvas.drawRect(Offset.zero & size, paint);
  }

  @override
  bool shouldRepaint(covariant _GpuHeatmapPainter oldDelegate) {
    return oldDelegate.densityTexture != densityTexture ||
        oldDelegate.maxDensity != maxDensity;
  }
}

/// CPU 回退热力图绘制器
///
/// 当 GPU 着色器不可用时使用 CPU 渲染
class _CpuHeatmapPainter extends CustomPainter {
  final Uint8List densityMap;
  final int maxDensity;

  _CpuHeatmapPainter({required this.densityMap, required this.maxDensity});

  /// 热力图颜色映射
  ///
  /// 将归一化的密度值 (0.0-1.0) 映射到热力图颜色
  Color _heatmapColor(double t) {
    // 五点渐变: 蓝 -> 青 -> 绿 -> 黄 -> 红
    const blue = Color(0xFF0000FF);
    const cyan = Color(0xFF00FFFF);
    const green = Color(0xFF00FF00);
    const yellow = Color(0xFFFFFF00);
    const red = Color(0xFFFF0000);

    Color color;

    if (t < 0.25) {
      color = Color.lerp(blue, cyan, t * 4)!;
    } else if (t < 0.5) {
      color = Color.lerp(cyan, green, (t - 0.25) * 4)!;
    } else if (t < 0.75) {
      color = Color.lerp(green, yellow, (t - 0.5) * 4)!;
    } else {
      color = Color.lerp(yellow, red, (t - 0.75) * 4)!;
    }

    return color;
  }

  @override
  void paint(Canvas canvas, Size size) {
    final densityCount = densityMap.length;
    final pixelsPerDensity = size.height / densityCount;

    for (int i = 0; i < densityCount; i++) {
      final density = densityMap[i];
      final normalizedDensity = density / max(1, maxDensity);

      // 应用伽马校正
      final gammaCorrected = pow(normalizedDensity, 0.7).toDouble();

      // 获取热力图颜色
      final color = _heatmapColor(gammaCorrected);

      // 计算绘制区域
      final top = i * pixelsPerDensity;
      final rect = Rect.fromLTWH(0, top, size.width, pixelsPerDensity + 1);

      // 绘制
      canvas.drawRect(
        rect,
        Paint()..color = color.withOpacity(0.3 + normalizedDensity * 0.7),
      );
    }
  }

  @override
  bool shouldRepaint(covariant _CpuHeatmapPainter oldDelegate) {
    return oldDelegate.densityMap != densityMap ||
        oldDelegate.maxDensity != maxDensity;
  }
}
