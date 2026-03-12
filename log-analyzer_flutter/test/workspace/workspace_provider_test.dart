import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:log_analyzer_flutter/shared/providers/workspace_provider.dart';
import 'package:log_analyzer_flutter/shared/models/common.dart';

void main() {
  group('WorkspaceProvider Tests', () {
    late ProviderContainer container;

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() {
      container.dispose();
    });

    test(
      'workspaceStateProvider should initialize with empty workspace list',
      () {
        final state = container.read(workspaceStateProvider);
        // workspaceStateProvider 返回 List<Workspace>
        expect(state, isEmpty);
      },
    );

    test('workspaceStateProvider should be a List<Workspace>', () {
      final state = container.read(workspaceStateProvider);
      // 验证返回类型是 List<Workspace>
      expect(state, isA<List<Workspace>>());
    });
  });
}
