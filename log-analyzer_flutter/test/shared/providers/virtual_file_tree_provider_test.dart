// VirtualFileTreeProvider 测试
//
// 测试虚拟文件树状态管理功能

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:log_analyzer_flutter/shared/providers/virtual_file_tree_provider.dart';

void main() {
  group('VirtualFileTreeProvider Tests', () {
    late ProviderContainer container;
    const testWorkspaceId = 'test-workspace-1';

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() {
      container.dispose();
    });

    group('初始状态', () {
      test('应返回空列表', () async {
        // 等待异步初始化
        await Future.delayed(const Duration(milliseconds: 100));

        final state = container.read(virtualFileTreeProvider(testWorkspaceId));

        // 应该返回空列表或加载中
        expect(state.valueOrNull, isNotNull);
      });
    });

    group('节点类型判断', () {
      test('isArchive 应正确识别归档节点', () {
        const node = VirtualTreeNode.archive(
          name: 'test.zip',
          path: '/test.zip',
          hash: 'abc123',
          archiveType: 'zip',
        );

        expect(node.isArchive, isTrue);
        expect(node.isFile, isFalse);
      });

      test('isFile 应正确识别文件节点', () {
        const node = VirtualTreeNode.file(
          name: 'test.log',
          path: '/test.log',
          hash: 'def456',
          size: 1024,
        );

        expect(node.isFile, isTrue);
        expect(node.isArchive, isFalse);
      });
    });

    group('节点属性访问', () {
      test('nodeName 应返回节点名称', () {
        const fileNode = VirtualTreeNode.file(
          name: 'test.log',
          path: '/test.log',
          hash: 'abc123',
          size: 1024,
        );

        const archiveNode = VirtualTreeNode.archive(
          name: 'archive.zip',
          path: '/archive.zip',
          hash: 'def456',
          archiveType: 'zip',
        );

        expect(fileNode.nodeName, equals('test.log'));
        expect(archiveNode.nodeName, equals('archive.zip'));
      });

      test('nodePath 应返回节点路径', () {
        const fileNode = VirtualTreeNode.file(
          name: 'test.log',
          path: '/logs/test.log',
          hash: 'abc123',
          size: 1024,
        );

        expect(fileNode.nodePath, equals('/logs/test.log'));
      });

      test('nodeHash 应返回节点哈希', () {
        const fileNode = VirtualTreeNode.file(
          name: 'test.log',
          path: '/test.log',
          hash: 'sha256hash',
          size: 1024,
        );

        expect(fileNode.nodeHash, equals('sha256hash'));
      });

      test('nodeType 应返回正确的类型', () {
        const fileNode = VirtualTreeNode.file(
          name: 'test.log',
          path: '/test.log',
          hash: 'abc123',
          size: 1024,
        );

        const archiveNode = VirtualTreeNode.archive(
          name: 'archive.zip',
          path: '/archive.zip',
          hash: 'def456',
          archiveType: 'zip',
        );

        expect(fileNode.nodeType, equals(VirtualTreeNodeType.file));
        expect(archiveNode.nodeType, equals(VirtualTreeNodeType.archive));
      });
    });

    group('子节点管理', () {
      test('文件节点应返回空子节点列表', () {
        const fileNode = VirtualTreeNode.file(
          name: 'test.log',
          path: '/test.log',
          hash: 'abc123',
          size: 1024,
        );

        expect(fileNode.children, isEmpty);
        expect(fileNode.hasChildren, isFalse);
      });

      test('归档节点应返回子节点列表', () {
        const archiveNode = VirtualTreeNode.archive(
          name: 'archive.zip',
          path: '/archive.zip',
          hash: 'def456',
          archiveType: 'zip',
          children: [
            VirtualTreeNode.file(
              name: 'inner.log',
              path: '/archive.zip/inner.log',
              hash: 'inner123',
              size: 512,
            ),
          ],
        );

        expect(archiveNode.children.length, equals(1));
        expect(archiveNode.hasChildren, isTrue);
      });

      test('空归档节点应返回空子节点列表', () {
        const archiveNode = VirtualTreeNode.archive(
          name: 'empty.zip',
          path: '/empty.zip',
          hash: 'empty123',
          archiveType: 'zip',
          children: [],
        );

        expect(archiveNode.children, isEmpty);
        expect(archiveNode.hasChildren, isFalse);
      });

      test('needsLazyLoad 应正确识别需要懒加载的节点', () {
        // 有子节点的归档节点不需要懒加载
        const loadedArchive = VirtualTreeNode.archive(
          name: 'loaded.zip',
          path: '/loaded.zip',
          hash: 'loaded123',
          archiveType: 'zip',
          children: [
            VirtualTreeNode.file(
              name: 'file.log',
              path: '/loaded.zip/file.log',
              hash: 'file123',
              size: 512,
            ),
          ],
        );

        // 无子节点的归档节点需要懒加载
        const unloadedArchive = VirtualTreeNode.archive(
          name: 'unloaded.zip',
          path: '/unloaded.zip',
          hash: 'unloaded123',
          archiveType: 'zip',
          children: [],
        );

        const fileNode = VirtualTreeNode.file(
          name: 'test.log',
          path: '/test.log',
          hash: 'test123',
          size: 1024,
        );

        expect(loadedArchive.needsLazyLoad, isFalse);
        expect(unloadedArchive.needsLazyLoad, isTrue);
        expect(fileNode.needsLazyLoad, isFalse);
      });
    });

    group('Freezed 模型', () {
      test('归档节点应支持 JSON 序列化', () {
        const node = VirtualTreeNode.archive(
          name: 'test.zip',
          path: '/test.zip',
          hash: 'abc123',
          archiveType: 'zip',
          children: [
            VirtualTreeNode.file(
              name: 'inner.log',
              path: '/test.zip/inner.log',
              hash: 'inner123',
              size: 512,
            ),
          ],
        );

        final json = node.toJson();

        expect(json['name'], equals('test.zip'));
        expect(json['path'], equals('/test.zip'));
        expect(json['archiveType'], equals('zip'));
        expect(json['children'], isA<List>());
      });

      test('文件节点应支持 JSON 序列化', () {
        const node = VirtualTreeNode.file(
          name: 'test.log',
          path: '/test.log',
          hash: 'abc123',
          size: 1024,
          mimeType: 'text/plain',
        );

        final json = node.toJson();

        expect(json['name'], equals('test.log'));
        expect(json['path'], equals('/test.log'));
        expect(json['size'], equals(1024));
        expect(json['mimeType'], equals('text/plain'));
      });

      test('应支持从 JSON 反序列化', () {
        final archiveJson = {
          'name': 'test.zip',
          'path': '/test.zip',
          'hash': 'abc123',
          'archiveType': 'zip',
          'children': <Map<String, dynamic>>[],
        };

        final node = VirtualTreeNode.fromJson(archiveJson);

        expect(node, isA<VirtualTreeNodeArchive>());
      });

      test('应支持模式匹配', () {
        const fileNode = VirtualTreeNode.file(
          name: 'test.log',
          path: '/test.log',
          hash: 'abc123',
          size: 1024,
        );

        const archiveNode = VirtualTreeNode.archive(
          name: 'test.zip',
          path: '/test.zip',
          hash: 'def456',
          archiveType: 'zip',
        );

        String getNodeType(VirtualTreeNode node) {
          return switch (node) {
            VirtualTreeNodeFile() => 'file',
            VirtualTreeNodeArchive() => 'archive',
          };
        }

        expect(getNodeType(fileNode), equals('file'));
        expect(getNodeType(archiveNode), equals('archive'));
      });
    });
  });

  group('FileContentResponse Tests', () {
    test('应正确创建文件内容响应', () {
      const response = FileContentResponse(
        content: 'test content',
        hash: 'abc123',
        size: 1024,
      );

      expect(response.content, equals('test content'));
      expect(response.hash, equals('abc123'));
      expect(response.size, equals(1024));
    });

    test('应支持 JSON 序列化', () {
      const response = FileContentResponse(
        content: 'test content',
        hash: 'abc123',
        size: 1024,
      );

      final json = response.toJson();

      expect(json['content'], equals('test content'));
      expect(json['hash'], equals('abc123'));
      expect(json['size'], equals(1024));
    });

    test('应支持从 JSON 反序列化', () {
      final json = {'content': 'test content', 'hash': 'abc123', 'size': 1024};

      final response = FileContentResponse.fromJson(json);

      expect(response.content, equals('test content'));
      expect(response.hash, equals('abc123'));
      expect(response.size, equals(1024));
    });
  });

  group('VirtualTreeNodeType Tests', () {
    test('应正确识别枚举值', () {
      expect(VirtualTreeNodeType.file, equals(VirtualTreeNodeType.file));
      expect(VirtualTreeNodeType.archive, equals(VirtualTreeNodeType.archive));
    });
  });
}
