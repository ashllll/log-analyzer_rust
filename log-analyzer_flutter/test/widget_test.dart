// 基础 Widget 测试
//
// 验证应用启动和基本 UI 渲染

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:log_analyzer_flutter/main.dart';

void main() {
  testWidgets('应用应能正常启动', (WidgetTester tester) async {
    // 构建应用并触发一帧
    await tester.pumpWidget(const LogAnalyzerApp());

    // 验证应用已渲染（检查是否有 Material 或 Scaffold 等组件）
    expect(find.byType(MaterialApp), findsOneWidget);
  });
}
