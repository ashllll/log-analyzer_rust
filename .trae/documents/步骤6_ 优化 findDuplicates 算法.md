## 步骤6: 优化 SearchQueryBuilder.ts findDuplicates 算法 O(n²) → O(n)

**任务**: 修复 line 271-285 `findDuplicates` 方法的时间复杂度

**当前问题**:
- 使用 Map 查找重复项
- 每个元素检查可能需要 O(n) 时间
- 总时间复杂度 O(n²)

**修改内容**:
- 使用 HashSet 实现 O(1) 查找
- 总时间复杂度优化到 O(n)

**验证方法**:
- 运行前端 lint 检查
- 运行单元测试

**影响范围**:
- 仅影响 SearchQueryBuilder 重复检测功能