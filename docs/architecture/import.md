# 导入链路

导入不仅是复制文件。它要管理路径安全、归档提取、内容去重、元数据事务、任务进度、完整性检查和索引收敛。

```mermaid
flowchart LR
  SOURCE[文件 / 目录 / 归档] --> CMD[import_folder command]
  CMD --> PIPE[ImportPipeline]
  PIPE --> SAFE[路径校验与规范化]
  SAFE --> EXTRACT[la-archive extraction]
  EXTRACT --> FILTER[FileTypeFilter]
  FILTER --> CAS[la-storage CAS]
  CAS --> META[SQLite MetadataStore]
  META --> STATS[统计收敛]
  STATS --> INDEX[Tantivy 事务重建 + CAS 哈希校验]
  PIPE --> TASK[TaskScheduler events]
```

## `ImportPipeline` 的职责

基础设施层的导入流水线统一承担：

- 工作区 id 与路径验证
- 源路径规范化和工作区目录准备
- TaskScheduler 创建、更新、完成与失败
- 工作区服务创建与 import 调用
- 失败后的资源清理
- 导入文件统计收敛
- 带 CAS 哈希校验的 Tantivy 事务重建

因此 Tauri command 可以保持薄边界，只处理调用参数和返回任务 id。

## 安全提取

`la-archive` 对归档条目执行目标路径约束，防止 `../` 等路径穿越逃逸提取目录，并对符号链接保持防护。嵌套归档由提取编排器递归处理，但仍受资源与安全策略约束。

## CAS 与元数据

内容按 SHA-256 写入 CAS；工作区、虚拟路径、原始来源和索引信息写入 SQLite。这样同一份内容可以被不同路径或工作区引用，而不会重复占用对象存储。

## 失败语义

流水线只在文件统计全部就绪、每个 CAS 对象的实际 SHA-256 与元数据一致、Tantivy
替换索引提交成功后完成任务。缺失或损坏对象会中止索引事务，保留上一次已提交索引，
并沿导入错误与任务失败链路返回；新建工作区导入失败时会清理未完成资源。
