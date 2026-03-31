# 运行手册

本手册面向维护者，覆盖本地构建、故障排查和回滚。

## 构建与启动

开发模式：

```bash
cd log-analyzer
npm install
npm run tauri dev
```

生产构建：

```bash
cd log-analyzer
npm run tauri build
```

Rust 全量检查：

```bash
cd log-analyzer/src-tauri
cargo test -q
```

## 常见运行路径

关键目录：

- 前端代码：`log-analyzer/src/`
- 后端命令层：`log-analyzer/src-tauri/src/commands/`
- 存储与搜索：`log-analyzer/src-tauri/src/storage/`、`log-analyzer/src-tauri/src/search_engine/`
- Workspace crates：`log-analyzer/src-tauri/crates/`

## 搜索相关排障

### 搜索无结果

优先检查：

1. 工作区是否已成功导入
2. `search_logs` 是否返回有效 `search_id`
3. 前端是否继续调用 `fetch_search_page`
4. 过滤条件是否过严

特别注意：

- 当前时间过滤使用项目自己的时间解析规则，不是纯 RFC3339
- 文件过滤支持通配符与子串两种模式
- 无法解析时间戳的日志行，在启用时间范围过滤时会被排除

### 搜索结果分页异常

检查：

1. 后端 `fetch_search_page` 是否能读到磁盘会话
2. `search_id` 是否已过期或被清理
3. 是否误删临时搜索结果文件

### 工作区导入后无法搜索

检查：

1. SQLite 元数据是否正常创建
2. CAS 对象是否写入 `objects/`
3. `MetadataStore::get_all_files()` 是否能返回文件

## 常见构建问题

### Tauri 前置依赖缺失

现象：

- `npm run tauri dev` 无法启动
- 平台构建脚本报缺依赖

处理：

- 按 Tauri 2 官方前置依赖逐项安装
- 重新执行 `npm run tauri dev`

### Rust 测试通过但文档测试失败

现象：

- `cargo test -q` 中 doctest 失败

处理：

- 检查 `rustdoc` 代码块是否应标记为 `text` / `ignore` / `no_run`
- 不要在文档里放会被误执行的 ASCII 图和需要 runtime 的示例

## 回滚建议

回滚时优先使用：

1. `git log --oneline` 找到最近稳定提交
2. 建立修复分支再处理
3. 对工作区数据问题，优先重新导入，而不是直接操作 CAS 对象目录

不建议：

- 直接删除用户工作区目录
- 在未核对元数据状态前强行清空搜索缓存

## 发布前核对

至少执行：

```bash
cd log-analyzer
npm run lint
npm run type-check
npm test

cd src-tauri
cargo fmt -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test -q
```

然后核对：

- `README.md`
- `docs/README.md`
- `CHANGELOG.md`
- `RELEASE_PROCESS.md`
