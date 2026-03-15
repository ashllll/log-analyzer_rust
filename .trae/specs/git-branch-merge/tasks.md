# Tasks

## 任务列表

- [ ] Task 1: 检查当前 Git 状态
  - [ ] SubTask 1.1: 查看当前所在分支
  - [ ] SubTask 1.2: 查看修改的文件列表
  - [ ] SubTask 1.3: 确认更改已暂存

- [ ] Task 2: 提交当前分支更改
  - [ ] SubTask 2.1: 添加所有更改到暂存区
  - [ ] SubTask 2.2: 创建带有描述性信息的提交

- [ ] Task 3: 合并到 main 分支
  - [ ] SubTask 3.1: 切换到 main 分支
  - [ ] SubTask 3.2: 拉取 main 分支最新代码
  - [ ] SubTask 3.3: 合并当前分支到 main

- [ ] Task 4: 解决合并冲突（如有）
  - [ ] SubTask 4.1: 识别冲突文件
  - [ ] SubTask 4.2: 手动解决冲突
  - [ ] SubTask 4.3: 提交解决后的合并

- [ ] Task 5: 验证合并结果
  - [ ] SubTask 5.1: 检查 main 分支状态
  - [ ] SubTask 5.2: 验证编译通过

# Task Dependencies

- [Task 1] 是所有其他任务的先行任务
- [Task 2] 依赖于 Task 1
- [Task 3] 依赖于 Task 2
- [Task 4] 仅在有冲突时执行
- [Task 5] 依赖于 Task 3 或 Task 4
