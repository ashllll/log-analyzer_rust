# 运行手册

**当前版本：** 1.2.60

本手册面向维护者，覆盖本地构建、启动、故障排查和回滚操作。

---

## 构建与启动

### 开发模式

```bash
cd log-analyzer
npm install        # 仅首次或依赖变更后需要
npm run tauri dev  # 启动 Tauri 开发模式（前端热重载 + Rust 编译）
```

Rust 首次编译耗时较长（约 2-5 分钟），后续增量编译通常在 10-30 秒内完成。

### 生产构建

```bash
cd log-analyzer
npm run tauri build
```

产物位置：

| 平台 | 路径 |
|------|------|
| macOS | `src-tauri/target/release/bundle/dmg/*.dmg` |
| Windows | `src-tauri/target/release/bundle/msi/*.msi` |
| Linux | `src-tauri/target/release/bundle/deb/*.deb` 或 `appimage/*.AppImage` |

### Rust 全量检查

```bash
cd log-analyzer/src-tauri
cargo test -q                           # 所有 workspace crate
cargo test -q -p la-storage             # 只测 la-storage
cargo test -q segment_pruning           # 按测试名过滤
cargo test -q -- --nocapture            # 显示 println! 输出
```

---

## 关键目录速查

| 路径 | 说明 |
|------|------|
| `log-analyzer/src/` | React 前端代码 |
| `log-analyzer/src-tauri/src/commands/` | Tauri IPC 命令层 |
| `log-analyzer/src-tauri/src/services/` | 查询执行、文件监听等业务逻辑 |
| `log-analyzer/src-tauri/src/storage/` | 缓存管理适配层 |
| `log-analyzer/src-tauri/src/search_engine/` | SearchEngineManager 适配层 |
| `log-analyzer/src-tauri/crates/` | la-core / la-storage / la-search / la-archive |
| `{app_data_dir}/workspaces/` | 工作区数据（CAS + metadata.db） |
| `{config_dir}/config.json` | 应用全局配置 |

`{app_data_dir}` 平台路径：

- macOS: `~/Library/Application Support/io.github.ashllll.log-analyzer/`
- Windows: `%APPDATA%\io.github.ashllll.log-analyzer\`
- Linux: `~/.local/share/io.github.ashllll.log-analyzer/`

---

## 搜索相关排障

### 应用启动配置加载

应用启动时从 `{config_dir}/config.json` 加载配置：

- 缓存参数（max_capacity、ttl_seconds、tti_seconds）
- 任务管理器参数（max_workers、timeout）
- 搜索引擎参数（max_results、timeout_seconds）

配置文件不存在时使用内置默认值，不影响启动。

若需要重置配置：删除 `{config_dir}/config.json`，重启应用即可。

---

### 搜索无结果

**排查步骤：**

1. 确认工作区已成功导入（检查导入时是否有错误事件）
2. 检查 `search_logs` 是否返回有效 `search_id`（打开 Tauri devtools 查看 invoke 日志）
3. 确认前端是否在调用 `fetch_search_page`
4. 检查过滤条件是否过严：
   - 时间范围是否与日志实际时间段匹配
   - 日志级别过滤是否与日志内容一致
   - 文件模式是否与虚拟路径格式匹配（虚拟路径以 `/` 开头）

**时间过滤特别说明：**

- 前端 `datetime-local` 格式（`YYYY-MM-DDTHH:MM`）已受支持
- 开始时间晚于结束时间会直接返回验证错误
- 日志行无法解析时间戳时，若启用了时间范围过滤，该行会被排除
- 时间对比使用本地无时区时间（`NaiveDateTime`），不依赖时区转换

**文件模式特别说明：**

- 含 `*` 或 `?` 时按 glob 通配符编译为正则（如 `*.log` 匹配所有 `.log` 文件）
- 不含通配符时按子串匹配（如 `error.log` 匹配路径中包含 `error.log` 的文件）
- 虚拟路径格式为 `/logs/app.log` 或 `/archives/service.zip/service/2024-01-15.log`

---

### 搜索结果分页异常

**现象：** 前端滚动加载失败，或 `total_count` 与实际结果数不符。

**排查：**

1. 确认后端 `fetch_search_page` 能读到磁盘会话文件
2. 检查 `search_id` 是否已过期（默认 30 分钟）或被主动清理
3. 检查是否误删了 `{tmp_dir}` 下的搜索临时文件
4. 查看后端日志中 `DiskResultStore` 相关的 ERROR 日志

---

### 工作区重启后无法搜索

**原因：** 应用重启后内存中的工作区运行态（`workspace_dirs`、CAS 实例、MetadataStore）会清空。

**已有恢复机制：** `search_logs` 会在内存中找不到工作区时自动从磁盘按 `workspaces/{workspace_id}/` 路径懒恢复运行态。

若恢复失败，排查：

1. 确认 `{app_data_dir}/workspaces/{workspace_id}/` 目录存在
2. 确认 `metadata.db` 文件未损坏（可用 `sqlite3 metadata.db "PRAGMA integrity_check;"` 验证）
3. 确认 `objects/` 目录下有 CAS 对象文件
4. 重新执行 `load_workspace` 命令手动触发恢复

---

### 导入后无法搜索

**排查步骤：**

1. 确认导入没有提前终止（查看导入事件流，是否有 `import-error`）
2. 确认 SQLite 元数据已写入：

```bash
sqlite3 {workspace_dir}/metadata.db "SELECT COUNT(*) FROM files;"
```

3. 确认 CAS 对象已写入：

```bash
ls {workspace_dir}/objects/ | head -20
```

4. 若元数据有记录但 CAS 对象不存在：存储一致性问题，建议重新导入

---

## 归档提取配置排障

归档解压行为由 `src-tauri/config/extraction_policy.toml` 控制，修改后需重启应用。

| 问题现象 | 排查方向 |
|---------|---------|
| 大文件解压失败 | 检查 `max_file_size` 和 `max_total_size` 是否过小 |
| 嵌套归档解压不完整 | 检查 `max_depth` 是否足够（默认 10） |
| Windows 路径过长错误 | 启用 `use_enhanced_extraction` 并设置 `enable_long_paths = true` |
| zip 炸弹误报 | 调低 `compression_ratio_threshold`（默认 100.0） |
| RAR 文件无法解压 | 确认平台支持（运行 `check_rar_support` 命令） |

---

## 常见构建问题

### Tauri 前置依赖缺失

**现象：** `npm run tauri dev` 无法启动，报缺少系统库。

**处理：** 按平台安装 Tauri 前置依赖（详见 [CONTRIB.md](./CONTRIB.md#tauri-平台前置依赖)），重新执行 `npm run tauri dev`。

---

### Rust 编译错误

**处理：** 先运行 `cargo check` 快速检查，再针对具体错误处理：

```bash
cd log-analyzer/src-tauri
cargo check --all-features  # 快速检查，不生成产物
cargo build 2>&1 | head -50 # 只看前 50 行错误
```

---

### cargo clippy 报错

**现象：** CI 中 clippy 失败，本地通过。

**处理：** 确保本地 Rust 版本与 CI 一致：

```bash
rustup update stable
rustup show  # 查看当前版本
```

---

### Rust 测试通过但 doctest 失败

**现象：** `cargo test -q` 中 doctest 失败，报语法错误或运行时错误。

**处理：**

- 检查 `///` 注释中的代码块是否应标记为 `text`、`ignore` 或 `no_run`
- ASCII 图表和需要运行时的示例，加 ` ```text ` 或 ` ```ignore `
- 不要在文档注释里放不能编译的代码片段

---

### 前端类型检查失败

**处理：**

```bash
cd log-analyzer
npm run type-check 2>&1 | head -30
```

常见原因：

- 后端 Tauri 命令参数/返回值结构变更，但 `src/types/` 未同步更新
- `api.ts` 中调用的命令名与后端注册名不一致

---

## 性能问题排查

### 搜索耗时过长

**诊断：**

1. 检查 L1 缓存命中率：低命中率意味着大量磁盘 CAS 读取
2. 查看任务管理器中的搜索任务统计（耗时、结果数）
3. 检查系统资源占用（CPU、内存、磁盘 I/O）

**常见原因与处理：**

| 原因 | 处理 |
|------|------|
| 工作区文件过多（>10 万文件） | 拆分工作区，减小单次搜索范围 |
| 缓存容量不足 | 增大 `cache.max_capacity` 配置 |
| 正则过于复杂 | 避免使用回溯型正则（如 `(a+)+`） |
| 文件过滤条件未设置 | 设置 `file_pattern` 减少候选文件数 |
| 时间/级别过滤未生效 | 确认过滤条件格式正确，检查是否触发分段摘要优化 |

---

### 导入速度慢

**常见原因：**

- 大量小文件（`<1KB`）：文件元数据写入 SQLite 成为瓶颈
- 深度嵌套归档：递归解压层数过多，考虑限制 `max_depth`
- 磁盘 I/O 是瓶颈：CAS 写入速度受磁盘限制，SSD 比 HDD 快 5-10 倍

---

## 回滚建议

### 代码回滚

```bash
git log --oneline -20            # 找到最近稳定提交
git checkout -b hotfix/xxx       # 从稳定提交创建修复分支
git reset --hard <stable_commit> # （在修复分支上）
```

不建议直接在 `main` 上 `git reset --hard`，应通过修复分支合并。

### 工作区数据回滚

若工作区元数据损坏：

1. 优先重新导入（重新触发 `import_folder`），不要手动操作 CAS 目录
2. 若需要快速恢复，可通过 `sqlite3` 直接查询元数据库确认损坏范围
3. **不要**直接删除 CAS `objects/` 目录，会导致元数据引用悬空

若 SQLite WAL 损坏：

```bash
sqlite3 metadata.db "PRAGMA wal_checkpoint(TRUNCATE);"
sqlite3 metadata.db "PRAGMA integrity_check;"
```

若完全损坏：删除 `metadata.db` 后重新导入（CAS 对象可复用，只需重建元数据）。

---

## 发布前核对

至少执行以下全量校验：

```bash
cd log-analyzer
npm run lint
npm run type-check
npm test
npm run build   # 确保前端生产构建通过

cd src-tauri
cargo fmt -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test -q
```

然后核对：

- `README.md`：主要能力描述是否仍准确
- `docs/README.md`：文档链接是否仍有效
- `CHANGELOG.md`：本次版本变更是否已记录
- `RELEASE_PROCESS.md`：版本号是否已在三个文件中对齐

版本号需要在以下三处保持一致：

- `log-analyzer/package.json` 中的 `version`
- `log-analyzer/src-tauri/Cargo.toml` 中的 `version`
- `log-analyzer/src-tauri/tauri.conf.json` 中的 `version`
