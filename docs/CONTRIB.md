# 贡献指南

本指南面向直接修改仓库代码的开发者，目标是让提交保持可验证、可回滚、可维护。

## 开发环境

要求：

- Node.js `>= 22.12.0`
- npm `>= 10`
- Rust `>= 1.70`
- 对应平台的 Tauri 依赖

初始化：

```bash
git clone https://github.com/ashllll/log-analyzer_rust.git
cd log-analyzer_rust/log-analyzer
npm install
```

开发运行：

```bash
npm run tauri dev
```

## 目录约定

- `log-analyzer/src/`
  - React 前端
- `log-analyzer/src-tauri/src/`
  - Tauri 后端主 crate
- `log-analyzer/src-tauri/crates/`
  - `la-core` / `la-storage` / `la-search` / `la-archive`
- `docs/`
  - 长期维护的核心文档

## 提交前最少验证

前端：

```bash
cd log-analyzer
npm run lint
npm run type-check
npm test
```

Rust：

```bash
cd log-analyzer/src-tauri
cargo fmt -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test -q
```

如果改动只触及单一模块，也至少补充对应模块测试。

## 文档维护规则

- 代码行为变化时，同时更新对应文档
- 只保留长期需要维护的文档
- 一次性报告、计划、复盘和 AI 工具说明不要继续堆在主文档集中
- 文档描述必须以当前代码真实行为为准

## 搜索链路约定

当前主搜索链路：

- 前端入口：`src/pages/SearchPage.tsx`
- 后端入口：`src-tauri/src/commands/search.rs`
- 核心匹配：`QueryExecutor` / `QueryPlanner` / `RegexEngine`
- 分页读取：`fetch_search_page`

注意：

- 当前 UI 主搜索仍使用简单字符串查询
- 关键词之间用 `|` 表示 OR 逻辑
- 结构化查询能力已存在，但不是当前主搜索入口

## 提交流程

建议步骤：

1. 在改动前确认真实业务路径
2. 先补测试或验证用例，再修改实现
3. 跑最少验证集
4. 更新受影响文档
5. `git diff` 自查后再提交

提交信息建议使用 Conventional Commits，例如：

```text
fix(search): tighten filters and precompile search boundaries
docs: prune outdated reports and refresh architecture docs
```

## 评审关注点

提交评审时优先检查：

- 是否修改了真实主链路，而不是未启用的预留能力
- 是否引入新的 I/O、锁或缓存边界问题
- 前后端字段命名与行为是否一致
- 文档是否仍然准确
