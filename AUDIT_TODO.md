# 代码审计修复跟踪清单

> 生成时间: 2026-05-14
> 总问题数: 161 (🔴Critical 16 / 🟠High 38 / 🟡Medium 65 / 🟢Low 42)
> 修复周期: 10小时
> 策略: 优先全部 Critical + High（54项），Medium/Low 视时间剩余处理

---

## 修复团队分工

| 代理 | 负责模块 | 问题数 | 优先级 |
|------|---------|--------|--------|
| Fix-Rust-Core | `src-tauri/src/commands/` + `src-tauri/src/services/` | 🔴3 + 🟠5 + 🟡10 + 🟢9 | P0/P1 |
| Fix-Rust-Infra | `src-tauri/src/utils/` + `storage/` + `state_sync/` + `task_manager/` + `models/` + `main.rs` | 🔴3 + 🟠6 + 🟡10 + 🟢8 | P0/P1 |
| Fix-Frontend-Core | `src/hooks/` + `src/services/` + `src/stores/` + `src/events/` + `src/types/` + `src/schemas/` + `src/utils/` | 🔴4 + 🟠9 + 🟡15 + 🟢4 | P0/P1 |
| Fix-Frontend-UI | `src/components/` + `src/pages/` + `App.tsx` + `main.tsx` | 🔴2 + 🟠8 + 🟡12 + 🟢8 | P0/P1 |
| Fix-Config-CI | `Cargo.toml` + `package.json` + CI/CD 配置 + `tauri.conf.json` | 🔴4 + 🟠10 + 🟡18 + 🟢13 | P0/P1 |

---

## 🔴 Critical (16项)

### Rust后端核心 (3项)
- [x] **CR-01** `import.rs:272-301` vs `search.rs:586-594` — `level_to_mask` 位定义完全相反，导致 segment pruning 跳过有效结果
- [x] **CR-02** `export.rs:21-28` — 路径遍历：允许绝对路径写入任意位置
- [x] **CR-03** `log_config.rs:71-85` — 路径遍历：`load_log_config`/`save_log_config` 直接使用用户传入路径

### Rust基础设施 (3项)
- [x] **CR-04** `utils/log_config.rs:196-211` — `RELOAD_HANDLE` 全局变量从未写入，动态日志级别调整完全失效
- [x] **CR-05** `utils/validation.rs:155-171` — `canonicalize_and_validate` 解析符号链接后路径逃逸
- [x] **CR-06** `main.rs:76` / `models/state.rs` — `AppState::default()` 中 `DiskResultStore::new()` panic 导致应用无法启动

### 前端核心逻辑 (4项)
- [x] **CR-07** `src/utils/logger.ts:101-141` — `info()`/`warn()`/`error()` 字符串重载未检查日志级别
- [x] **CR-08** `src/hooks/useSearchListeners.ts:63-139` — `Promise.all` 短路导致 Tauri 监听器泄漏
- [x] **CR-09** `src/services/api.ts:91-116` — `invokeWithTimeout` 超时竞态掩盖真实错误
- [x] **CR-10** `src/hooks/useServerQueries.ts:22-42` — `queryFn` 中直接执行 Zustand store 副作用

### 前端UI (2项)
- [x] **CR-11** `src/pages/SearchPage.tsx:209-214` — `useEffect` 依赖禁用 ESLint，`handleSearch` stale closure
- [x] **CR-12** `src/App.tsx:123` + `ErrorBoundary.tsx:356,518` — `MemoryRouter` 与 `window.location.hash` 不匹配，错误恢复失效

### 配置与CI/CD (4项)
- [x] **CR-13** `Cargo.toml` / `la-archive/Cargo.toml` — `zip = "2.2"` 受 CVE-2025-29787 影响
- [x] **CR-14** `Cargo.toml` — `bytes = "1.0"` 受 RUSTSEC-2026-0007 影响
- [x] **CR-15** `Cargo.toml` / `la-archive/Cargo.toml` — `async_zip = "0.0.18"` 已废弃停止维护
- [x] **CR-16** `Cargo.toml` — `sentry = "0.48"` 主 crate 未发布，会导致构建失败

---

## 🟠 High (38项)

### Rust后端核心 (5项)
- [x] **HI-01** `search.rs:879` — 每次搜索新建 `rayon::ThreadPool`
- [x] **HI-02** `search.rs:921` — 磁盘写入失败静默清空批次，数据丢失
- [x] **HI-03** `workspace.rs:688` — `cancel_task` 错误被丢弃，始终返回 `Ok(())`
- [ ] **HI-04** `file_watcher.rs:410` — 同步线程中 `tokio::Handle::current().block_on()`
- [ ] **HI-05** `workspace.rs:197` — `resolve_refresh_source_path` 未验证用户传入 `path`

### Rust基础设施 (6项)
- [ ] **HI-06** `utils/async_resource_manager.rs:327` — 已完成操作未从 `active_operations` 移除，内存泄漏
- [ ] **HI-07** `utils/retry.rs:95` — 同步函数使用 `std::thread::sleep()` 阻塞异步线程
- [x] **HI-08** `utils/cache.rs:443` — `start_cleanup_task()` 可被多次调用，旧任务泄漏
- [ ] **HI-09** `models/state.rs:70` — 搜索完成后未清理 `CancellationToken`，内存泄漏
- [ ] **HI-10** `utils/command_validation.rs:67` — 路径验证未覆盖 Null 字节/绝对路径/编码绕过
- [ ] **HI-11** `utils/workspace_paths.rs:28` — `workspace_id` 未验证直接拼接路径

### 前端核心逻辑 (9项)
- [ ] **HI-12** `src/services/api.ts:611` — `readFileByHash` fallback 导致 `catch` 永不可达
- [ ] **HI-13** `src/hooks/useErrorManagement.ts:72` — `error.message` 在 null/undefined/number 时崩溃
- [ ] **HI-14** `src/hooks/useFormValidation.ts:123` — `error` 对象直接拼接字符串模板
- [ ] **HI-15** `src/events/types.ts:126` — 对 `z.ZodError` 使用不必要的 `as` 断言
- [ ] **HI-16** `src/services/SearchQueryBuilder.ts:95` — `JSON.parse` + `as` 零运行时验证
- [ ] **HI-17** `src/types/api-responses.ts` — `WorkspaceStatusSchema` 与 `WorkspaceSchema.status` 枚举不一致
- [ ] **HI-18** `src/hooks/useQueryErrorHandler.ts:18` — `isNetworkError` 类型守卫过于宽松
- [ ] **HI-19** `src/hooks/useTauriEventListeners.ts:94` — `import-complete` 与 `task-update` 重复修改 store
- [ ] **HI-20** `src/services/SearchQueryBuilder.ts:156` — `toggleTerm`/`updateTermValue` 直接突变内部对象

### 前端UI (8项)
- [ ] **HI-21** `src/components/modals/KeywordModal.tsx` — 缺失焦点陷阱、ESC关闭、焦点恢复
- [ ] **HI-22** `src/components/modals/FileFilterSettings.tsx` — 缺少焦点陷阱和自动聚焦
- [ ] **HI-23** `src/components/modals/FilterPalette.tsx` — 缺少 ESC 关闭和 ARIA 属性
- [ ] **HI-24** `src/pages/SearchPage/components/LogRow.tsx` — `<div>` 绑定 onClick 但无 role/tabIndex/键盘事件
- [ ] **HI-25** `src/components/modals/KeywordModal.tsx` — `patterns.map` 使用 `index` 作为 `key`
- [ ] **HI-26** `src/pages/KeywordsPage.tsx` — 使用 `index` 作为 `key`
- [ ] **HI-27** `src/pages/SettingsPage.tsx` — Search/Task 配置全部硬编码中文
- [ ] **HI-28** `src/components/modals/KeywordModal.tsx` — 模态框全部硬编码英文

### 配置与CI/CD (10项)
- [ ] **HI-29** `tauri.conf.json` — CSP 包含 `'unsafe-inline'` 和 `http://ipc.localhost`
- [ ] **HI-30** `capabilities/external-open.json` — `opener:allow-open-url` 允许所有 `http://*` / `https://*`
- [ ] **HI-31** `Jenkinsfile` — Docker socket 挂载进构建容器
- [ ] **HI-32** `Jenkinsfile` — `agent any` 无节点标签限制
- [ ] **HI-33** `vite.config.ts` — 未显式设置 `sourcemap: false` / `build.target`
- [ ] **HI-34** `Cargo.toml` — `libloading = "0.8"` + 大量关键依赖仅锁大版本
- [ ] **HI-35** `.github/workflows/release.yml` — Tauri v1/v2 签名密钥复用
- [ ] **HI-36** `.gitlab-ci.yml` — `cargo audit || true` 掩盖安全漏洞
- [ ] **HI-37** `log-analyzer/package-lock.json` — `fast-check` 在 lockfile 中但不在 package.json
- [ ] **HI-38** `.github/workflows/ci.yml` — `RUST_BACKTRACE: full` 泄露敏感路径信息

---

## 🟡 Medium (65项) — 时间允许时处理

### Rust后端核心 (10项)
- [ ] **ME-01** `services/regex_engine.rs:620` — `needs_lookaround` 无法区分转义括号
- [ ] **ME-02** `services/query_executor.rs:27` — `compute_query_cache_key` 不必要克隆整个 Vec
- [ ] **ME-03** `commands/workspace.rs:302` — `remove_file_with_retry` / `remove_dir_with_retry` 重试逻辑混乱
- [ ] **ME-04** `commands/virtual_tree.rs:190` — `get_virtual_file_tree` 未验证 `workspaceId`
- [ ] **ME-05** `commands/import.rs:109` — `ensure_workspace_runtime_state` Check-Then-Act 竞态
- [ ] **ME-06** `services/file_watcher.rs:392` — 监听到修改时读取完整文件内容重新写入 CAS
- [ ] **ME-07** `services/traits.rs:46` — `build_execution_plan` 默认实现返回空计划
- [x] **ME-08** `commands/search.rs:1008` — 超时后仍可能 emit `search-progress`
- [ ] **ME-09** `services/file_watcher.rs:357` — 跨函数锁获取顺序不一致
- [ ] **ME-10** `commands/search.rs:366` — `split_query_by_pipe` 对未闭合括号不报错

### Rust基础设施 (10项)
- [ ] **ME-11** `utils/cache.rs:315` — `get_with_ttl_check()` 使用 write 锁做读操作
- [ ] **ME-12** `utils/cancellation_manager.rs:64` — 令牌仅在显式调用时移除，可能泄漏
- [ ] **ME-13** `state_sync/mod.rs:50` — `broadcast_workspace_event` 失败时前端状态不一致
- [ ] **ME-14** `task_manager/mod.rs:234` — Actor 内同步调用 `app.emit` 可能阻塞
- [ ] **ME-15** `task_manager/mod.rs:561` — `TaskManager::new()` 返回 Result 但无失败分支
- [ ] **ME-16** `utils/async_resource_manager.rs:216` — `graceful_shutdown()` 忙等待循环
- [x] **ME-17** `utils/encoding.rs:107` — `invalid_ratio` 使用字符数而非字节数计算
- [ ] **ME-18** `utils/log_config.rs:146` — `set_global_default` 使用 `expect` 可能 panic
- [ ] **ME-19** `utils/validation.rs:111` — `url_decode` 输入为 `&str` 限制原始二进制处理
- [ ] **ME-20** `utils/async_resource_manager.rs:161` — `cancel_workspace_operations` 循环内重复 `to_string()`

### 前端核心逻辑 (15项)
- [ ] **ME-21** `events/EventBus.ts:175` — `getInstance(config)` 单例已存在时忽略新 config
- [ ] **ME-22** `hooks/useConfigManager.ts:42` — `saveTimeoutRef` 类型声明为 `NodeJS.Timeout`
- [ ] **ME-23** `hooks/useResourceManager.ts:8` — 同上，浏览器环境使用 NodeJS 类型
- [ ] **ME-24** `hooks/useSearchQuery.ts:67` — `loadQuery` 反复执行 `JSON.stringify`
- [ ] **ME-25** `pages/SearchPage/hooks/useVirtualScroll.ts:34` — `measureElement` callback 为死代码
- [ ] **ME-26** `pages/SearchPage/hooks/useSearchEvents.ts:59` — `setTimeout` 未在 cleanup 中清除
- [ ] **ME-27** `schemas/keywordSchema.ts:146` — 大量 `as` 类型断言降级 Zod 类型安全
- [ ] **ME-28** `services/errors.ts:302` — `withErrorHandler` 装饰器丢失原函数签名
- [ ] **ME-29** `hooks/useEventBusSubscriptions.ts:76` — 批量事件导致 Toast 堆叠和重复刷新
- [ ] **ME-30** `hooks/useConfigInitializer.ts:33` — 已验证数据仍使用不必要的 `as` 断言
- [ ] **ME-31** `types/api-responses.ts:342` — `AppConfigFileFilterSchema` 与 `FileFilterConfigSchema` 类型不统一
- [ ] **ME-32** `hooks/useWorkspaceManagement.ts:26` — 与 `useDeleteWorkspaceMutation` 功能重叠
- [ ] **ME-33** `services/api.ts:52` — `sanitizeArgs` 未处理 `Date`、`RegExp` 等特殊对象
- [ ] **ME-34** `hooks/useConfigManager.ts:77` — `useEffect` 依赖 `saveConfig` 导致频繁重置定时器
- [ ] **ME-35** `pages/SearchPage/hooks/useSearchState.ts:31` — `keywordColors` 为空数组时返回 `undefined`

### 前端UI (12项)
- [ ] **ME-36** `pages/SearchPage.tsx:323` — `queryTerms` useMemo 依赖排除 `currentQuery.terms`
- [ ] **ME-37** `pages/SearchPage.tsx:159` — `useSearchEvents` options 对象每次 render 新引用
- [ ] **ME-38** `pages/SearchPage/components/ActiveKeywords.tsx:30` — 删除按钮仅 hover 显示，键盘不可见
- [ ] **ME-39** `pages/SearchPage/components/SearchResults.tsx:65` — 表头使用 div 而非语义化 table
- [ ] **ME-40** `pages/SearchPage/components/SearchResults.tsx:126` — 加载提示硬编码中文
- [ ] **ME-41** `components/ErrorBoundary.tsx` — 多个 `<button>` 未声明 `type="button"`
- [ ] **ME-42** `pages/SettingsPage.tsx:85` — `useEffect` 依赖 `showToast` 和 `t` 导致重复加载配置
- [ ] **ME-43** `pages/TasksPage.tsx:51` — `EmptyState` 描述硬编码中文
- [ ] **ME-44** `pages/WorkspacesPage.tsx:43` — `window.confirm` 对辅助技术不友好
- [ ] **ME-45** `components/renderers/HybridLogRenderer.tsx:303` — `renderHighlightedText` 未用 useCallback
- [ ] **ME-46** `components/renderers/HybridLogRenderer.tsx:369` — segments key 稳定性不足，DOM 节点过多
- [ ] **ME-47** `components/search/KeywordStatsPanel.tsx:83` — `keywords.map` 使用 `index` 作为 key

### 配置与CI/CD (18项)
- [ ] **ME-48** `.github/workflows/bump-and-tag.yml` — `actions: write` 过度授权
- [ ] **ME-49** `.github/workflows/ci.yml` — `RUST_BACKTRACE: full` 泄露敏感路径
- [ ] **ME-50** `jest.config.js` — `collectCoverage: true` 强制收集 + 阈值过低 (35%)
- [ ] **ME-51** `eslint.config.js` — `no-console: 'off'` 且缺少安全规则
- [ ] **ME-52** `tsconfig.json` — `skipLibCheck: true` 掩盖类型不兼容
- [ ] **ME-53** `tsconfig.node.json` — 缺少 `strict: true`
- [ ] **ME-54** `rust-toolchain.toml` — `channel = "stable"` 未指定具体版本
- [ ] **ME-55** `.clippy.toml` — 测试中完全允许 unwrap/expect/panic
- [ ] **ME-56** `proptest.toml` — `cases = 32` 偏少
- [ ] **ME-57** `.gitlab-ci.yml` — `GITLAB_TOKEN` 未声明，`macos:latest` 非标准镜像
- [ ] **ME-58** `Jenkinsfile` — `docker system prune -f` 清理所有未使用资源
- [ ] **ME-59** `Jenkinsfile` — Clippy 缺少 `-D warnings`，未使用辅助函数
- [ ] **ME-60** `tauri.conf.json` — 窗口缺少 `minWidth`/`minHeight`，`targets: "all"`
- [ ] **ME-61** `capabilities/file-dialog.json` — `dialog:allow-open/save` 未限制路径范围
- [ ] **ME-62** `package.json` — `@tauri-apps/cli@^2` 和 `react-router-dom@^7.0.0` 版本过宽
- [ ] **ME-63** 根目录 `package-lock.json` — 包含 `agentic-flow` 但子目录无此依赖
- [ ] **ME-64** `vite.config.ts` — 缺少 `@types/node`
- [ ] **ME-65** `.github/workflows/ci.yml` — 多线程测试失败后静默重试单线程

---

## 🟢 Low (42项) — 记录待后续迭代

### Rust后端核心 (9项)
- [ ] **LO-01** `search.rs:995` — 魔法数字 `10000` 含义不清
- [ ] **LO-02** `workspace.rs:844` — `resolve_workspace_id` 返回 BTreeMap 第一个 key（字母序）
- [ ] **LO-03** `workspace.rs:761` — `i64 as usize` 未校验非负
- [ ] **LO-04** `validation.rs:133` — 路径长度检查使用字节数而非字符数
- [ ] **LO-05** `config.rs:43` — 配置解析失败静默降级为默认配置
- [ ] **LO-06** `search.rs:825` — `validate_search_params` 不检查 `structuredQuery` 累积长度
- [ ] **LO-07** `query_planner.rs:27` — `regex_has_inline_case_flag` 按字节迭代多字节字符
- [ ] **LO-08** `search.rs:560` — `escaped.replace` 产生冗余 `^.*.*$`
- [ ] **LO-09** `search.rs:366` — `split_query_by_pipe` 对未闭合括号不报错

### Rust基础设施 (8项)
- [ ] **LO-10** `utils/path.rs:187` — `safe_path_join` 文档注释与实现不符
- [ ] **LO-11** `utils/command_validation.rs:19` — `validate_search_query` 使用字节长度
- [ ] **LO-12** `state_sync/models.rs:66` — `WorkspaceState.progress` 无边界校验
- [ ] **LO-13** `main.rs:152` — 退出处理未注册全局 panic hook
- [ ] **LO-14** `utils/cache.rs:151` — 缓存无最大容量限制
- [ ] **LO-15** `services/concurrency_property_tests.rs` — 测试通用模式而非生产代码实际锁顺序
- [ ] **LO-16** `state_sync/property_tests.rs` — `apply_event_to_map` 与生产逻辑不完全一致
- [ ] **LO-17** `utils/async_resource_manager.rs:161` — 循环内重复 `to_string()`

### 前端核心逻辑 (4项)
- [ ] **LO-18** `services/SearchQueryBuilder.ts:105` — `generateId` 使用已弃用 `substr`
- [ ] **LO-19** `pages/SearchPage/hooks/useSearchState.ts:31` — `keywordColors` 为空时返回 undefined
- [ ] **LO-20** `types/search.ts:89` — 使用动态 import 类型语法
- [ ] **LO-21** `hooks/useBackendSync.ts:12` — 无 cleanup 逻辑
- [ ] **LO-22** `lib/queryClient.ts:18` — `refetchOnWindowFocus: true` 全局默认

### 前端UI (8项)
- [ ] **LO-23** `pages/SearchPage.tsx:86` — `fetchNextPage` 引用稳定性
- [ ] **LO-24** `SearchControls.tsx:58` — placeholder 硬编码中文
- [ ] **LO-25** `SearchFilters.tsx` — 多个标签硬编码英文
- [ ] **LO-26** `KeywordsPage.tsx` — 标题按钮空状态硬编码英文
- [ ] **LO-27** `WorkspaceHeader.tsx` — 硬编码英文
- [ ] **LO-28** `KeywordModal.tsx:255` — `onChange` 内联定义导致重渲染
- [ ] **LO-29** `components/ui/Button.tsx:79` — `stopPropagation()` 过度使用
- [ ] **LO-30** `LogDetailPanel.tsx:75` — 格式化视图与复制内容不一致
- [ ] **LO-31** `ErrorBoundary.tsx:590` — Toast 提示硬编码中文
- [ ] **LO-32** `SearchPage.tsx:125` — 双重 `useMemo` 链
- [ ] **LO-33** `ErrorBoundary.tsx:339` — else 分支传递无效的 `onRetry`

### 配置与CI/CD (13项)
- [ ] **LO-34** `Cargo.toml` — `libc` 与 `rustix` 功能重叠
- [ ] **LO-35** `build.rs` — `expect` 缺少详细错误上下文
- [ ] **LO-36** `rustfmt.toml` — 注释掉的 nightly-only 配置
- [ ] **LO-37** `.vscode/extensions.json` — 缺少前端扩展推荐
- [ ] **LO-38** `Cargo.toml` — `unrar` 可选特性存在许可证冲突风险
- [ ] **LO-39** `eslint.config.js` — `src-tauri` 被完全排除
- [ ] **LO-40** `.github/workflows/release.yml` — tauri-action SHA 可能过时
- [ ] **LO-41** `package.json` — `prepare: husky` 自动安装 hooks
- [ ] **LO-42** `.github/workflows/ci.yml` — 测试重试逻辑掩盖 flaky tests
- [ ] **LO-43** `tauri.conf.json` — `main-window-core.json` capability 未引用
- [ ] **LO-44** `Cargo.toml` — 依赖版本未锁定到 minor
- [ ] **LO-45** `Jenkinsfile` — Slack channel 硬编码
- [ ] **LO-46** `.gitlab-ci.yml` — Windows/macOS 构建镜像配置问题
