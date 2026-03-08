import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:log_analyzer_flutter/shared/providers/workspace_provider.dart';

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
        expect(state.workspaces, isEmpty);
      },
    );

    test('workspaceStateProvider should have initial status', () {
      final state = container.read(workspaceStateProvider);
      expect(state.status, equals(WorkspaceStatus.initial));
    });
  });
}
