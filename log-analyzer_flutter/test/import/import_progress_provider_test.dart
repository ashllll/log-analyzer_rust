import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:log_analyzer_flutter/shared/providers/import_progress_provider.dart';

void main() {
  group('ImportProgressProvider Tests', () {
    late ProviderContainer container;
    late ImportProgress notifier;

    setUp(() {
      container = ProviderContainer();
      notifier = container.read(importProgressProvider.notifier);
    });

    tearDown(() {
      container.dispose();
    });

    test('importProgressProvider should initialize with idle status', () {
      final state = container.read(importProgressProvider);
      expect(state.status, equals(ImportStatus.idle));
    });

    test('updateProgress should update state correctly', () {
      notifier.updateProgress(
        totalFiles: 10,
        processedFiles: 5,
        currentFile: 'test.log',
      );
      final state = container.read(importProgressProvider);
      expect(state.totalFiles, equals(10));
      expect(state.processedFiles, equals(5));
      expect(state.currentFile, equals('test.log'));
    });

    test('cancelImport should set status to cancelled', () async {
      notifier.cancelImport();
      final state = container.read(importProgressProvider);
      expect(state.status, equals(ImportStatus.cancelled));
    });

    test('reset should return to idle status', () {
      notifier.updateProgress(
        totalFiles: 10,
        processedFiles: 5,
        currentFile: 'test.log',
      );
      notifier.reset();
      final state = container.read(importProgressProvider);
      expect(state.status, equals(ImportStatus.idle));
      expect(state.processedFiles, equals(0));
    });
  });
}
