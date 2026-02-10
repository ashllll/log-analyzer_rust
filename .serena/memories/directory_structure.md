# 项目目录结构

## 根目录
```
log-analyzer_rust/
├── log-analyzer/           # 主应用目录
│   ├── src/               # React前端源码
│   │   ├── components/    # UI组件
│   │   ├── pages/        # 页面
│   │   ├── services/     # API封装
│   │   ├── stores/       # Zustand状态管理
│   │   └── types/        # TypeScript类型定义
│   └── src-tauri/        # Rust后端源码
│       └── src/
│           ├── commands/          # Tauri命令
│           ├── archive/           # 压缩包处理
│           │   ├── actors/        # Actor模型
│           │   ├── fault_tolerance/ # 容错处理
│           │   └── streaming/     # 流式处理
│           ├── domain/            # 领域模型
│           ├── events/            # 事件总线
│           ├── infrastructure/    # 基础设施
│           ├── models/            # 数据模型
│           │   ├── config.rs      # 配置管理
│           │   └── extraction_policy.rs # 提取策略
│           ├── search_engine/     # 搜索引擎
│           ├── services/          # 业务服务
│           │   ├── pattern_matcher.rs    # 模式匹配
│           │   ├── query_executor.rs     # 查询执行
│           │   ├── file_type_filter.rs   # 文件过滤
│           │   └── event_bus.rs         # 事件总线
│           ├── state_sync/       # 状态同步
│           ├── storage/          # CAS存储
│           ├── task_manager/     # 任务管理
│           └── utils/            # 工具函数
├── docs/                     # 项目文档
├── scripts/                  # 构建脚本
├── CHANGELOG.md             # 更新日志
├── CLAUDE.md                # AI指导文档
└── README.md                # 项目说明
```

## 核心模块说明

### archive/ - 压缩包处理
- 支持 ZIP/TAR/GZ/RAR/7Z 格式
- 递归解压嵌套压缩包
- 压缩炸弹检测
- 流式处理大文件

### search_engine/ - 搜索引擎
- Tantivy全文搜索
- Aho-Corasick多模式匹配
- 正则表达式支持
- 布尔查询(AND/OR/NOT)

### services/ - 业务服务
- PatternMatcher: 模式匹配器
- QueryExecutor: 查询执行器
- QueryValidator: 查询验证器
- QueryPlanner: 查询计划器
- FileWatcher: 文件监听器

### storage/ - 存储系统
- CAS (内容寻址存储)
- SQLite数据库
- FTS5全文搜索
- Gzip压缩索引
