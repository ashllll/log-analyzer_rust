import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:log_analyzer_flutter/shared/providers/import_progress_provider.dart';
import 'package:log_analyzer_flutter/shared/widgets/import_progress_dialog.dart';

void main() {
  group('ImportProgressDialog Tests', () {
    testWidgets('should display progress when importing', (tester) async {
      // 创建一个测试用的状态
      const testState = ImportProgressState(
        status: ImportStatus.importing,
        totalFiles: 100,
        processedFiles: 50,
        currentFile: 'test.log',
        progressPercent: 0.5,
        errors: [],
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            importProgressProvider.overrideWithValue(testState),
          ],
          child: const MaterialApp(
            home: Scaffold(
              body: ImportProgressDialog(),
            ),
          ),
        ),
      );

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(find.text('50 / 100'), findsOneWidget);
      expect(find.text('test.log'), findsOneWidget);
    });

    testWidgets('should display cancel button when importing', (tester) async {
      const testState = ImportProgressState(
        status: ImportStatus.importing,
        totalFiles: 100,
        processedFiles: 50,
        currentFile: 'test.log',
        progressPercent: 0.5,
        errors: [],
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            importProgressProvider.overrideWithValue(testState),
          ],
          child: const MaterialApp(
            home: Scaffold(
              body: ImportProgressDialog(),
            ),
          ),
        ),
      );

      expect(find.text('取消'), findsOneWidget);
    });

    testWidgets('should display completed state', (tester) async {
      const testState = ImportProgressState(
        status: ImportStatus.completed,
        totalFiles: 100,
        processedFiles: 100,
        currentFile: '',
        progressPercent: 1.0,
        errors: [],
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            importProgressProvider.overrideWithValue(testState),
          ],
          child: const MaterialApp(
            home: Scaffold(
              body: ImportProgressDialog(),
            ),
          ),
        ),
      );

      expect(find.text('导入完成'), findsOneWidget);
    });
  });
}
