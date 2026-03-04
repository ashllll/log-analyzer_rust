# Phase 4: 压缩包浏览 - Research

**Researched:** 2026-03-02
**Domain:** Flutter 桌面应用 + Rust 后端压缩包处理
**Confidence:** HIGH

## Summary

Phase 4 需要实现用户浏览压缩包内文件、预览文本内容、以及在压缩包内搜索功能。Rust 后端已有完整的压缩包解压模块（ZIP/TAR/GZ/RAR/7Z），但缺少两个关键能力：
1. 列出压缩包内容（不解压）
2. 读取压缩包内单个文件内容

Flutter 前端已有 ArchiveImportDialog 组件，但 API 为 stub 实现。需要新增 Tauri 命令并实现对应的 Flutter 页面。

**Primary recommendation:** 在 Rust 后端实现 `list_archive_contents` 和 `read_archive_file` 命令，复用现有 archive handlers；Flutter 端使用 TreeView + SplitPane 实现树形浏览和实时预览。

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions
- 树形视图展示嵌套目录结构
- Split Pane 布局（左侧列表，右侧预览）
- 单击文件立即预览
- 预览支持关键词高亮
- 实时搜索模式
- 支持所有主流格式: ZIP/TAR/GZ/RAR/7Z
- 大文件截断并提示
- 空压缩包或无法预览时显示友好提示

### Claude's Discretion
- 树形视图的具体展开/折叠交互细节
- 搜索结果排序逻辑
- 预览面板的默认宽度比例
- 大文件阈值具体数值

### Deferred Ideas (OUT OF SCOPE)
- 无 — 讨论保持在 Phase 4 范围内

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| ARCH-01 | 用户可以浏览压缩包内的文件列表 | 需实现 `list_archive_contents` 后端命令 + Flutter TreeView |
| ARCH-02 | 用户可以预览压缩包内的文本文件内容 | 需实现 `read_archive_file` 命令 + SplitPane 预览组件 |
| ARCH-03 | 用户可以在压缩包内搜索关键词 | 复用 Phase 3 搜索能力 + 集成到压缩包预览 |

</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| **Rust `zip`** | 0.6 | ZIP 格式读取 | 纯 Rust 实现，稳定可靠 |
| **Rust `tar`** | 0.4 | TAR 格式读取 | 标准 TAR 处理 |
| **Rust `flate2`** | 1.0 | GZIP 解压 | 配合 tar 使用 |
| **Rust `unrar`** | 0.5 | RAR 格式读取 | libunrar 绑定 |
| **Rust `sevenz-rust`** | 0.5 | 7Z 格式读取 | 纯 Rust 实现 |
| **Flutter `tree_view`** | (内置) | 树形视图 | Flutter 内置 Widget |

### Supporting (Flutter)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| **flutter_rust_bridge** | 2.x | FFI 桥接 | 后端通信 |
| **Riverpod** | latest | 状态管理 | 管理压缩包浏览状态 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| 纯 Flutter 解压 | 使用 archive 包 | 不如 Rust 高效，且需重复实现 |
| HTTP API | 直接 FFI 调用 | FFI 性能更好（已采用） |
| Flat list | TreeView | 用户要求树形结构 |

---

## Architecture Patterns

### Recommended Project Structure

```
log-analyzer_flutter/
├── lib/
│   ├── features/
│   │   └── archive_browsing/
│   │       ├── presentation/
│   │       │   ├── pages/
│   │       │   │   └── archive_browser_page.dart    # 主页面
│   │       │   └── widgets/
│   │       │       ├── archive_tree_view.dart        # 树形视图
│   │       │       ├── archive_preview_panel.dart   # 预览面板
│   │       │       └── archive_search_bar.dart       # 搜索栏
│   │       ├── providers/
│   │       │   └── archive_browser_provider.dart     # 状态管理
│   │       └── models/
│   │           └── archive_node.dart                 # 数据模型
│   └── shared/
│       └── services/
│           └── api_service.dart                       # 扩展 API 方法

log-analyzer/src-tauri/src/
├── commands/
│   └── archive_commands.rs                           # 新增命令
└── archive/
    ├── archive_reader.rs                              # 新增：读取压缩包内容
    └── mod.rs
```

### Pattern 1: Archive Reader (不解压读取)

**What:** 直接读取压缩包内的文件列表和内容，无需完整解压

**When to use:** 浏览压缩包内容、预览单个文件

**Example (Rust):**
```rust
// 使用 zip crate 读取文件列表
use zip::ZipArchive;

pub fn list_zip_contents(path: &Path) -> Result<Vec<ArchiveEntry>> {
    let file = std::fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        entries.push(ArchiveEntry {
            name: file.name().to_string(),
            is_dir: file.is_dir(),
            size: file.size(),
        });
    }
    Ok(entries)
}

// 读取单个文件内容
pub fn read_zip_file(path: &Path, file_name: &str) -> Result<String> {
    let file = std::fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut zip_file = archive.by_name(file_name)?;
    let mut contents = String::new();
    zip_file.read_to_string(&mut contents)?;
    Ok(contents)
}
```

### Pattern 2: TreeView + SplitPane (Flutter)

**What:** 左侧树形视图展示目录结构，右侧实时预览选中文件

**When to use:** 需要同时浏览文件结构和预览内容的场景

**Example:**
```dart
// 主页面布局
Row(
  children: [
    // 左侧：树形视图 (30% 宽度)
    SizedBox(
      width: MediaQuery.of(context).size.width * 0.3,
      child: ArchiveTreeView(
        entries: entries,
        selectedPath: selectedPath,
        onSelect: (path) {
          setState(() => selectedPath = path);
          _loadPreview(path);
        },
      ),
    ),
    // 右侧：预览面板 (70% 宽度)
    Expanded(
      child: ArchivePreviewPanel(
        content: previewContent,
        searchKeyword: searchKeyword, // 关键词高亮
        isLoading: isLoading,
        error: error,
      ),
    ),
  ],
)
```

### Pattern 3: Real-time Search

**What:** 搜索框输入关键词，实时过滤和显示匹配结果

**When to use:** 用户需要在大量文件中快速定位

**Example:**
```dart
// 搜索栏
TextField(
  onChanged: (keyword) {
    // 实时搜索
    _performSearch(keyword);
  },
)

// 搜索结果高亮
RichText(
  text: TextSpan(
    children: _highlightMatches(content, searchKeyword),
  ),
)
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ZIP 读取 | 自己解析 ZIP 格式 | `zip` crate | ZIP 格式复杂，容易出错 |
| TAR 解析 | 手写 tar 格式 | `tar` + `flate2` crate | 标准库成熟稳定 |
| RAR 支持 | 自己实现 | `unrar` crate | RAR 专利限制，绑定 libunrar |
| 7Z 支持 | 自己实现 | `sevenz-rust` crate | 纯 Rust 实现 |
| 文本检测 | 手写二进制检测 | `encoding_rs` + 文件扩展名 | 已集成在项目中 |

**Key insight:** 项目已有完整的 Rust archive 模块，实现 ARCH-01/02/03 只需扩展功能，无需重新发明轮子。

---

## Common Pitfalls

### Pitfall 1: 大文件内存爆炸
**What goes wrong:** 预览时一次性加载整个文件到内存，导致应用卡顿或崩溃
**Why it happens:** 未对压缩包内大文件做限制
**How to avoid:**
- 设置预览文件大小阈值（如 10MB）
- 大文件截断显示 + 提示用户
- 使用流式读取

**Warning signs:**
- 内存占用急剧上升
- UI 线程阻塞
- 预览加载时间过长

### Pitfall 2: 二进制文件当作文本预览
**What goes wrong:** 预览图片/二进制文件时显示乱码
**Why it happens:** 未检测文件类型
**How to avoid:**
- 基于文件扩展名判断
- 使用 `encoding_rs` 检测编码
- 二进制文件显示占位符或禁止预览

**Warning signs:**
- 大量不可读字符
- 应用无响应

### Pitfall 3: 嵌套压缩包处理
**What goes wrong:** 压缩包内还有压缩包（如 a.zip 包含 b.tar.gz）
**Why it happens:** 未递归处理嵌套结构
**How to avoid:**
- 树形视图支持递归展开
- 识别压缩包格式图标
- 用户选择后递归读取

### Pitfall 4: 中文路径/文件名乱码
**What goes wrong:** 中文文件名显示为乱码
**Why it happens:** 编码问题（GBK vs UTF-8）
**How to avoid:**
- 使用 `encoding_rs` 尝试多种编码
- 显示友好错误而非乱码

---

## Code Examples

### Rust: 扩展 ArchiveHandler Trait

在 `src/archive/archive_handler.rs` 添加新方法：

```rust
#[async_trait]
pub trait ArchiveHandler {
    // ... 现有方法 ...

    /// 列出压缩包内容（不解压）
    fn list_contents(&self, path: &Path) -> Result<Vec<ArchiveEntry>>;

    /// 读取单个文件内容
    fn read_file(&self, path: &Path, file_name: &str) -> Result<String>;
}
```

### Rust: 新增 Tauri 命令

在 `src/commands/archive_commands.rs`：

```rust
#[tauri::command]
pub async fn list_archive_contents(
    archive_path: String,
) -> Result<Vec<ArchiveEntry>, String> {
    // 复用现有 handlers
    let path = Path::new(&archive_path);
    let handler = find_handler(path).ok_or("Unsupported format")?;
    handler.list_contents(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_archive_file(
    archive_path: String,
    file_name: String,
) -> Result<String, String> {
    let path = Path::new(&archive_path);
    let handler = find_handler(path).ok_or("Unsupported format")?;
    handler.read_file(path, &file_name).map_err(|e| e.to_string())
}
```

### Flutter: ArchiveTreeView Widget

```dart
class ArchiveTreeView extends StatelessWidget {
  final List<ArchiveEntry> entries;
  final String? selectedPath;
  final void Function(String path) onSelect;

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      itemCount: entries.length,
      itemBuilder: (context, index) {
        final entry = entries[index];
        return TreeTile(
          isSelected: entry.path == selectedPath,
          isDirectory: entry.isDirectory,
          name: entry.name,
          onTap: () => onSelect(entry.path),
        );
      },
    );
  }
}
```

### Flutter: 关键词高亮

```dart
List<TextSpan> _highlightMatches(String text, String keyword) {
  if (keyword.isEmpty) return [TextSpan(text: text)];

  final spans = <TextSpan>[];
  final regex = RegExp(RegExp.escape(keyword), caseSensitive: false);
  int lastEnd = 0;

  for (final match in regex.allMatches(text)) {
    if (match.start > lastEnd) {
      spans.add(TextSpan(text: text.substring(lastEnd, match.start)));
    }
    spans.add(TextSpan(
      text: text.substring(match.start, match.end),
      style: TextStyle(
        backgroundColor: Colors.yellow,
        fontWeight: FontWeight.bold,
      ),
    ));
    lastEnd = match.end;
  }

  if (lastEnd < text.length) {
    spans.add(TextSpan(text: text.substring(lastEnd)));
  }

  return spans;
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| 完整解压后浏览 | 不解压直接读取 | Phase 4 新增 | 内存效率提升 90%+ |
| 先解压再搜索 | 直接在压缩包内搜索 | Phase 4 新增 | 无需临时文件 |
| 静态搜索 | 实时搜索（输入即搜索） | Phase 3 已实现 | 用户体验更好 |

**Deprecated/outdated:**
- 完整解压到临时目录再浏览（Phase 2 做法）
- 手动遍历目录查找文件（被 TreeView 替代）

---

## Open Questions

1. **大文件阈值具体数值？**
   - What we know: 需要设置阈值防止内存爆炸
   - What's unclear: 10MB 还是 50MB？
   - Recommendation: 默认 10MB，用户可配置

2. **是否支持嵌套压缩包递归预览？**
   - What we know: 用户提到"浏览压缩包内的文件"
   - What's unclear: 是否需要处理 a.zip -> b.tar.gz -> c.log 这种嵌套？
   - Recommendation: Phase 4 先支持单层，嵌套作为后续功能

3. **搜索结果如何展示？**
   - What we know: 用户要求"实时搜索模式"
   - What's unclear: 搜索结果显示在列表上方还是新面板？
   - Recommendation: 在预览面板内添加搜索栏，结果在当前文件内高亮

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust: `#[tokio::test]`, Flutter: `flutter_test` |
| Config file | `log-analyzer/src-tauri/Cargo.toml` |
| Quick run command | `cargo test archive --lib` |
| Full suite command | `cargo test --all-features` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|---------------|
| ARCH-01 | 列出压缩包内容 | unit | `cargo test list_archive` | Yes - 需新增 |
| ARCH-02 | 预览文本文件 | unit | `cargo test read_archive_file` | Yes - 需新增 |
| ARCH-03 | 压缩包内搜索 | integration | `cargo test archive_search` | Yes - 需新增 |

### Sampling Rate
- **Per task commit:** `cargo test archive --lib -- --nocapture`
- **Per wave merge:** `cargo test --all-features`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src/archive/archive_reader.rs` — ARCH-01, ARCH-02 核心实现
- [ ] `src/commands/archive_commands.rs` — Tauri 命令
- [ ] `test_archive_commands` — 命令测试
- [ ] `test_archive_reader` — 读取器测试

---

## Sources

### Primary (HIGH confidence)
- 项目现有代码: `log-analyzer/src-tauri/src/archive/*.rs` — 已有 archive handlers
- 项目现有代码: `log-analyzer_flutter/lib/shared/widgets/archive_import_dialog.dart` — 现有 UI
- Rust `zip` crate 文档: 已知稳定版本 0.6
- Flutter SDK: TreeView/ListView 内置组件

### Secondary (MEDIUM confidence)
- 搜索实现参考 Phase 3 代码
- 项目 CLAUDE.md 中的压缩包处理文档

### Tertiary (LOW confidence)
- 无

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH - 项目已有完整 archive 模块
- Architecture: HIGH - 模式清晰（读取器 + 命令 + UI）
- Pitfalls: MEDIUM - 大文件/二进制文件问题需实际验证

**Research date:** 2026-03-02
**Valid until:** 2026-04-02 (30 days for stable library versions)
