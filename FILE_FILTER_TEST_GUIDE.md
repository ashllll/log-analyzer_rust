# 文件类型过滤功能 - 端到端测试指南

## 功能概述

本功能实现了三层检测策略的文件类型过滤系统，用于在导入日志时自动过滤非日志文件。

### 三层检测策略

1. **第1层：二进制文件检测**（默认启用）
   - 仅读取文件前1KB进行魔数检测
   - 自动识别：图片（JPEG、PNG、GIF等）、视频（MP4、MKV等）、音频（MP3、WAV等）、可执行文件（EXE、ELF等）
   - 性能影响：<1ms/文件

2. **第2层：智能过滤规则**（默认禁用，可选启用）
   - **文件名 Glob 模式**：支持无后缀日志文件（如 `syslog`、`messages`、`stdout`）
   - **扩展名过滤**：白名单/黑名单双模式

## 测试前准备

### 1. 启动应用

```bash
cd F:\github\log-analyzer_rust\log-analyzer
npm run tauri dev
```

### 2. 创建测试数据

创建以下测试文件结构：

```
test_data/
├── logs/                          # 应该被导入
│   ├── app.log                    # ✓ 匹配 *log* 模式
│   ├── syslog                     # ✓ 匹配 syslog 模式（无后缀）
│   ├── messages                   # ✓ 匹配 messages 模式（无后缀）
│   ├── error.log                  # ✓ 匹配 *log* 模式
│   └── access.2024-01-01          # ✓ 匹配 *.20* 模式
├── binary_files/                  # 应该被第1层拒绝
│   ├── image.png                  # ✗ 二进制文件（PNG魔数）
│   ├── photo.jpg                  # ✗ 二进制文件（JPEG魔数）
│   └── program.exe                # ✗ 二进制文件（EXE魔数）
├── text_files/                    # 根据配置决定
│   ├── data.csv                   # ? 如果启用白名单且csv不在列表则拒绝
│   ├── config.json                # ? 如果启用白名单且json在列表则允许
│   └── readme.txt                 # ? 如果启用白名单且txt在列表则允许
└── archives/                      # 递归解压后应用过滤
    ├── logs.zip                   # 解压后内容会被过滤
    └── mixed.tar.gz               # 解压后内容会被过滤
```

### 3. 生成测试文件脚本

**Python 脚本**（generate_test_data.py）：

```python
import os
from pathlib import Path

# 创建测试目录
base_dir = Path("test_data")
base_dir.mkdir(exist_ok=True)

# 1. 创建日志文件（应该被导入）
logs_dir = base_dir / "logs"
logs_dir.mkdir(exist_ok=True)

(logs_dir / "app.log").write_text("""2024-01-01 12:00:00 INFO Application started
2024-01-01 12:00:01 ERROR Database connection failed
2024-01-01 12:00:02 WARN Retrying connection
""")

(logs_dir / "syslog").write_text("""Jan  1 12:00:00 server app[123]: Started
Jan  1 12:00:01 server app[123]: Error: Connection failed
""")

(logs_dir / "messages").write_text("""Jan  1 12:00:00 kernel: [    0.000000] Linux version 6.1.0
Jan  1 12:00:01 server sshd[1234]: Accepted password for user
""")

(logs_dir / "error.log").write_text("""[ERROR] 2024-01-01 Failed to connect
[ERROR] 2024-01-01 Timeout occurred
""')

(logs_dir / "access.2024-01-01").write_text("""127.0.0.1 - - [01/Jan/2024:12:00:00] "GET /api HTTP/1.1" 200
127.0.0.1 - - [01/Jan/2024:12:00:01] "POST /api/data HTTP/1.1" 201
""")

# 2. 创建二进制文件（应该被第1层拒绝）
binary_dir = base_dir / "binary_files"
binary_dir.mkdir(exist_ok=True)

# PNG文件（8字节PNG魔数）
(binary_dir / "image.png").write_bytes(b'\x89PNG\r\n\x1a\n\x00\x00\x00')

# JPEG文件（3字节JPEG魔数）
(binary_dir / "photo.jpg").write_bytes(b'\xff\xd8\xff\xe0\x00\x10JFIF')

# EXE文件（2字节EXE魔数）
(binary_dir / "program.exe").write_bytes(b'MZ\x90\x00\x03\x00\x00\x00')

# 3. 创建文本文件（根据配置决定）
text_dir = base_dir / "text_files"
text_dir.mkdir(exist_ok=True)

(text_dir / "data.csv").write_text("""id,name,value
1,Item1,100
2,Item2,200
""")

(text_dir / "config.json").write_text("""{"app": "LogAnalyzer", "version": "1.0.0"}
""")

(text_dir / "readme.txt").write_text("""This is a readme file.
It contains plain text.
""")

print(f"Test data created in {base_dir.absolute()}")
print("\nFile structure:")
for root, dirs, files in os.walk(base_dir):
    level = root.replace(str(base_dir), '').count(os.sep)
    indent = ' ' * 2 * level
    print(f'{indent}{os.path.basename(root)}/')
    subindent = ' ' * 2 * (level + 1)
    for file in files:
        print(f'{subindent}{file}')
```

**运行脚本**：
```bash
python generate_test_data.py
```

## 测试场景

### 场景1：默认配置（仅第1层二进制检测）

**目标**：验证二进制文件被自动过滤

**步骤**：
1. 启动应用
2. 进入 Workspaces 页面
3. 点击"Import Folder"，选择 `test_data` 目录
4. 观察导入结果

**预期结果**：
- ✅ 允许：`logs/` 下的所有日志文件
- ✅ 允许：`text_files/` 下的所有文本文件
- ✅ 拒绝：`binary_files/` 下的二进制文件（PNG、JPEG、EXE）
- ✅ 日志输出：`File skipped by filter configuration`（针对二进制文件）

**验证日志**：
```bash
# 查看后端日志
# macOS
tail -f ~/Library/Logs/com.joeash.log-analyzer/

# Linux
tail -f ~/.local/share/com.joeash.log-analyzer/logs/

# Windows
# 查看 %APPDATA%\com.joeash.log-analyzer\logs\
```

**应该看到类似日志**：
```
[INFO] File skipped by filter configuration file=image.png path=...test_data/binary_files/image.png
[INFO] file=photo.jpg Detected binary file by magic number file_type=JPEG
[INFO] file=program.exe Detected binary file by magic number file_type=EXE
```

---

### 场景2：启用第2层白名单模式

**目标**：验证白名单模式只导入指定扩展名文件

**步骤**：
1. 启动应用
2. 进入 Workspaces 页面
3. 点击"文件过滤设置"按钮
4. 配置如下：
   - ✅ 启用二进制文件检测
   - ✅ 启用智能过滤规则
   - 选择"白名单模式"
   - 文件名模式：`*log*`, `syslog`, `messages`
   - 扩展名白名单：`log`, `txt`
5. 保存配置
6. 导入 `test_data` 目录

**预期结果**：
- ✅ 允许：`logs/app.log`（匹配 *log* 模式）
- ✅ 允许：`logs/syslog`（匹配 syslog 模式）
- ✅ 允许：`logs/messages`（匹配 messages 模式）
- ✅ 允许：`text_files/readme.txt`（txt 在白名单中）
- ✅ 拒绝：`text_files/data.csv`（csv 不在白名单中）
- ✅ 拒绝：`text_files/config.json`（json 不在白名单中）
- ✅ 拒绝：`logs/access.2024-01-01`（不匹配任何模式且无扩展名）

---

### 场景3：启用第2层黑名单模式

**目标**：验证黑名单模式只拒绝指定扩展名文件

**步骤**：
1. 打开文件过滤设置
2. 配置如下：
   - ✅ 启用二进制文件检测
   - ✅ 启用智能过滤规则
   - 选择"黑名单模式"
   - 扩展名黑名单：`exe`, `bat`, `sh`
3. 保存配置
4. 导入 `test_data` 目录

**预期结果**：
- ✅ 允许：所有日志文件和文本文件（除了二进制文件）
- ✅ 拒绝：`binary_files/program.exe`（exe 在黑名单中）

---

### 场景4：禁用所有过滤（向后兼容）

**目标**：验证禁用过滤后允许所有文件

**步骤**：
1. 打开文件过滤设置
2. 配置如下：
   - ❌ 禁用二进制文件检测
   - ❌ 禁用智能过滤规则
3. 保存配置
4. 导入 `test_data` 目录

**预期结果**：
- ✅ 允许：所有文件（包括二进制文件）
- ✅ 行为与原始版本完全一致

---

### 场景5：压缩包递归过滤

**目标**：验证压缩包内的文件也应用过滤规则

**步骤**：
1. 创建包含混合文件的 ZIP 压缩包：
   ```
   mixed.zip
   ├── app.log        # 应该被导入
   ├── image.png      # 应该被拒绝
   └── data.csv       # 根据配置决定
   ```

2. 使用白名单模式配置（仅允许 .log 文件）
3. 导入 `mixed.zip`

**预期结果**：
- ✅ 压缩包自动解压
- ✅ `app.log` 被导入
- ✅ `image.png` 被拒绝（二进制检测）
- ✅ `data.csv` 被拒绝（不在白名单）

---

## 防御性设计验证

### 测试1：配置加载失败

**目标**：验证配置加载失败时自动允许所有文件

**步骤**：
1. 手动损坏配置文件（添加无效 JSON）：
   ```bash
   # macOS/Linux
   echo '{"invalid": json}' > ~/.local/share/com.joeash.log-analyzer/config.json

   # Windows
   echo {"invalid": json} > %APPDATA%\com.joeash.log-analyzer\config.json
   ```

2. 重启应用
3. 尝试导入文件

**预期结果**：
- ✅ 应用正常启动
- ✅ 所有文件都能导入（降级到默认行为）
- ✅ 后端日志显示：`Failed to load file filter config, allowing all files (fail-safe)`

---

### 测试2：过滤逻辑异常

**目标**：验证过滤逻辑异常时不影响导入

**步骤**：
1. 添加一个特殊的文件名模式（可能导致正则表达式编译失败）
2. 保存配置
3. 尝试导入文件

**预期结果**：
- ✅ 导入正常进行
- ✅ 其他文件的过滤仍然有效
- ✅ 后端日志记录警告但不中断流程

---

## 性能测试

### 测试大量文件的导入性能

**目标**：验证过滤功能对性能的影响

**步骤**：
1. 创建包含 10,000 个文件的测试目录：
   - 5,000 个 .log 文件（应该允许）
   - 5,000 个 .png 文件（应该拒绝）

2. 配置：仅启用二进制检测（默认配置）
3. 导入目录并记录时间

**预期结果**：
- ✅ 导入时间增加 <5%（二进制检测仅读1KB）
- ✅ 所有 PNG 文件被快速拒绝（魔数检测）
- ✅ 所有 LOG 文件正常导入

---

## 验证清单

完成以下检查项：

- [ ] **默认配置**：二进制文件自动被过滤
- [ ] **白名单模式**：只导入匹配规则文件
- [ ] **黑名单模式**：跳过黑名单文件
- [ ] **无后缀文件**：`syslog`、`messages` 等被正确识别
- [ ] **压缩包递归**：压缩包内文件也应用过滤
- [ ] **配置持久化**：重启后配置保留
- [ ] **防御性设计**：配置损坏时不影响导入
- [ ] **日志记录**：每个决策都有日志输出
- [ ] **性能影响**：导入时间增加 <5%
- [ ] **向后兼容**：禁用过滤后行为一致

---

## 常见问题排查

### 问题1：文件应该被拒绝但被导入了

**检查步骤**：
1. 确认过滤配置已启用
2. 检查后端日志，查看过滤决策
3. 验证文件名模式或扩展名是否匹配

### 问题2：文件应该被导入但被拒绝了

**检查步骤**：
1. 检查是否误启用白名单模式
2. 确认文件名模式或扩展名在白名单中
3. 查看后端日志，确认拒绝原因

### 问题3：配置保存后没有生效

**检查步骤**：
1. 刷新工作区或重新导入
2. 检查配置文件是否正确写入：
   ```bash
   # 查看配置文件内容
   cat ~/.local/share/com.joeash.log-analyzer/config.json
   ```
3. 重启应用后重新尝试

---

## 测试完成标准

所有测试场景通过后，功能即可视为验收合格：

1. ✅ 默认配置下二进制文件被自动过滤
2. ✅ 白名单/黑名单模式正确工作
3. ✅ 无后缀日志文件被正确识别
4. ✅ 压缩包递归过滤正常
5. ✅ 防御性设计有效（配置失败不影响导入）
6. ✅ 性能影响可接受（<5%）
7. ✅ 向后兼容（禁用后行为一致）
8. ✅ 日志详细可追踪

---

## 相关文件清单

### 后端文件
- `src-tauri/src/services/file_type_filter.rs` - 核心过滤逻辑
- `src-tauri/src/models/config.rs` - 配置模型
- `src-tauri/src/archive/processor.rs` - 导入流程集成
- `src-tauri/src/commands/config.rs` - Tauri 命令

### 前端文件
- `src/components/modals/FileFilterSettings.tsx` - 设置 UI 组件
- `src/pages/WorkspacesPage.tsx` - 集成设置按钮
- `src/types/common.ts` - TypeScript 类型定义

### 测试文件
- 本文档（测试指南）
- `generate_test_data.py` - 测试数据生成脚本

---

*测试指南 v1.0 - 2025-01-03*
