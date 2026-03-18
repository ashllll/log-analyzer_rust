# Tasks - 搜索引擎剩余缺陷修复

## P1 高优先级修复

- [x] Task 1: HLE-M2 - SnippetCacheKey Hash依赖Debug
  - [x] 1.1: 检查SnippetCacheKey当前实现
  - [x] 1.2: 使用确定性格式替代Debug

- [x] Task 2: CSE-M2 - ReaderPool创建失败无降级
  - [x] 2.1: 分析ReaderPool::new失败场景
  - [x] 2.2: 添加降级处理逻辑

## P2 中优先级修复

- [x] Task 3: SEM-M1 - SearchError不区分错误类型
  - [x] 3.1: 定义RetryableError trait或枚举
  - [x] 3.2: 实现is_retryable()方法

- [x] Task 4: MAN-L1 - 多关键词未考虑引号短语
  - [x] 4.1: 分析当前split_whitespace逻辑
  - [x] 4.2: 添加引号短语解析支持

- [x] Task 5: ADV-L1 - document_count只记录最大ID
  - [x] 5.1: 分析当前document_count逻辑
  - [x] 5.2: 改为使用实际文档计数

- [x] Task 6: ADV-L2 - Regex编译失败错误消息不含pattern
  - [x] 6.1: 在错误消息中添加pattern信息

- [x] Task 7: ADV-L3 - 负时间戳分区键计算错误
  - [x] 7.1: 修复负时间戳的分区键计算

- [x] Task 8: IOP-L1 - created_indexes Vec无去重
  - [x] 8.1: 添加去重逻辑

## P4 低优先级修复

- [x] Task 9: IOP-L4 - 时钟调回cleanup_old_patterns跳过
  - [x] 9.1: 改进时钟回拨处理

- [x] Task 10: QOP-L1 - 硬编码top 3建议数
  - [x] 10.1: 提取为配置常量

- [x] Task 11: SEM-L1 - delete_file_documents持锁期间commit
  - [x] 11.1: 参照commit()的锁作用域模式

## 验证任务

- [x] Task 12: 编译验证
  - [x] 12.1: cargo build
  - [x] 12.2: cargo clippy

- [x] Task 13: 测试验证
  - [x] 13.1: cargo test search_engine

# Task Dependencies
- Task 12, 13 依赖于 Task 1-11 全部完成
