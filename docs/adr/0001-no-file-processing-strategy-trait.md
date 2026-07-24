# ADR-0001: 文件处理策略不定义 trait（假想 seam）

- 状态：已接受（Accepted）
- 日期：2026-07-20
- 背景：`/improve-codebase-architecture` 评审，候选 #1「深化 SearchUseCase」

## 背景

`application/search.rs` 的 `run_blocking` 约 160 行，混合了文件分类、Rayon 并行、
大文件流式读取、取消检查与批量冲刷。评审决定提取两个具体策略结构体
（`ParallelSmallFileStrategy` / `StreamingLargeFileStrategy`），由编排器按有序段
match 派发，使 `run_blocking` 瘦身为纯编排。

## 决策

**不**定义 `FileProcessingStrategy` trait。策略以两个具体结构体存在于
`application/search_strategy.rs`，编排器直接实例化并调用。

## 理由

- 策略 seam 只有一个消费者（`SearchUseCase` 编排器）。词汇表原则：
  一个 adapter = 假想 seam，两个 = 真 seam。
- P7 重构已删除过度抽象的 `search_executor.rs`；`SearchBatch` 以
  「无 trait 纯模块」形式提取并证明成功——本决策沿用同一先例。
- 策略测试直接实例化具体结构体（各 4 个 stub），不需要 trait 对象做替身。

## 后果

- 若未来出现第二个策略消费者，或第三种策略（如二进制文件搜索），
  应重新评估是否引入 trait。
- 未来架构审查**不应再建议**「提取 `FileProcessingStrategy` trait」，
  除非上述条件出现。

## 同次评审的相关决策（本 ADR 的上下文）

- 结果严格按文件列表顺序落库：编排器分段派发，非按策略分组。
- `SearchBatch` 全局唯一，由编排器持有、`&mut` 共享给策略；
  `max_results` 全局截断依赖此共享。
- `LogEntry.id` 偏移不对称（小文件传 0 / 大文件传 `batch.total()`）
  保持现状；小文件 id 跨文件重复的问题另开 issue 追踪。
- 测试分层：现有 7 个 `run_blocking` 集成测试原样保留（行为保持安全网），
  另为两个策略新增聚焦单测。
