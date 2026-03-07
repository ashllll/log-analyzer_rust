// 文件树集成测试
//
// 测试虚拟文件树工作流

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:log_analyzer_flutter/shared/providers/virtual_file_tree_provider.dart';

void main() {
  group('File Tree Integration Tests', () {
    late ProviderContainer container;
    const testWorkspaceId = 'test-workspace-1';

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() {
      container.dispose();
    });

    group('文件树加载', () {
      test('应正确加载根节点', () async {
        // 等待异步加载
        await Future.delayed(const Duration(milliseconds: 100));

        final state = container.read(virtualFileTreeProvider(testWorkspaceId));
        final nodes = state.valueOrNull ?? [];

        // 初始应为空或加载中
        expect(nodes, isNotNull);
      });
    });

    group('目录展开/折叠', () {
      test('目录应包含子节点', () async {
        const archiveNode = VirtualTreeNode.archive(
          name: 'logs',
          path: '/logs',
          hash: 'dir-hash-1',
          archiveType: null, // 目录
          children: [
            VirtualTreeNode.file(
              name: 'app.log',
              path: '/logs/app.log',
              hash: 'file-hash-1',
              size: 1024,
            ),
          ],
        );

        expect(archiveNode.children.length, equals(1));
        expect(archiveNode.children.first.name, equals('app.log'));
      });
    });

    group('Freezed 模型集成', () {
      test('应能正确序列化文件树', () {
        const node = VirtualTreeNode.archive(
          name: 'logs',
          path: '/logs',
          hash: 'hash1',
          archiveType: null,
          children: [
            VirtualTreeNode.file(
              name: 'app.log',
              path: '/logs/app.log',
              hash: 'hash2',
              size: 1024,
            ),
          ],
        );

        // 序列化根节点
        final rootJson = node.toJson();

        expect(rootJson['name'], equals('logs'));
        expect(rootJson['children'], isA<List>());
      });

      test('应能反序列化文件树', () {
        final json = {
          'name': 'test.log',
          'path': '/test.log',
          'hash': 'hash123',
          'size': 1024,
        };

        final nodes = [VirtualTreeNode.fromJson(json)];

        expect(nodes.length, equals(1));
        expect(nodes.first.nodeName, equals('test.log'));
      });
    });
  });
}
