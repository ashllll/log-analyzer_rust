# CAS 架构

本项目使用内容寻址存储（Content-Addressable Storage, CAS）保存导入后的文件内容，并使用 SQLite 保存元数据。

## 为什么使用 CAS

目标：

- 让工作区内容与原始导入路径解耦
- 对重复内容自动去重
- 支撑嵌套压缩包导入
- 为搜索、虚拟文件树和导出提供稳定的数据源

相对传统“路径映射”方案，CAS 的优势是：

- 文件内容只按哈希存一份
- 虚拟路径可独立维护
- 不依赖临时解压路径长期存在

## 核心组成

### 1. CAS 对象存储

实现位置：

- `log-analyzer/src-tauri/src/storage/`
- `log-analyzer/src-tauri/crates/la-storage/`

对象以 SHA-256 哈希为标识保存。

目录形态：

```text
workspace/
├── objects/
│   ├── ab/
│   │   └── cdef...
│   └── ...
└── metadata.db
```

### 2. SQLite 元数据

元数据保存：

- 文件虚拟路径
- 原始名称
- 大小和修改时间
- 所属压缩包关系
- 层级深度

核心表：

- `files`
- `archives`

### 3. 搜索读取链路

当前真实搜索主链路不是直接依赖数据库全文索引，而是：

1. `MetadataStore::get_all_files()` 取出候选文件
2. 通过 CAS 读取文件内容
3. 由 `QueryExecutor` 做逐行匹配
4. 结果写入磁盘分页存储

这意味着：

- CAS 是搜索主链路的真实内容来源
- 文档描述必须以这条链路为准

## 数据流

导入：

```text
文件/压缩包
→ 计算 SHA-256
→ 写入 CAS 对象目录
→ 写入 SQLite 元数据
→ 建立文件与归档关系
```

搜索：

```text
search_logs
→ MetadataStore 取文件列表
→ CAS 读取内容
→ QueryExecutor 匹配
→ DiskResultStore 按页写盘
→ fetch_search_page 读取分页
```

## 与压缩包处理的关系

压缩包能力主要由 `la-archive` 提供。CAS 在这里承担两件事：

- 保存解压后的实际内容
- 让嵌套文件通过虚拟路径对外可见

因此业务上可以同时保留：

- 内容哈希
- 虚拟路径
- 父归档层级关系

## 当前边界

当前需要认清的边界：

- CAS 负责内容与元数据，不等于“搜索引擎”
- SQLite 中存在与搜索相关的基础设施，但主搜索链路仍是文件扫描 + 匹配执行
- 旧迁移说明和历史兼容文档已移除，不再作为当前架构依据

## 相关文档

- [IPC API 概览](./API.md)
- [模块架构](./modules/MODULE_ARCHITECTURE.md)
- [搜索优化与边界条件审核](../search-optimization-review.md)
