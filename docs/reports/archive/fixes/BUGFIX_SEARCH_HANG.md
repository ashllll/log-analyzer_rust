# 搜索无响应问题修复

## 问题描述
用户报告：关键词搜索后无反应，图标点击后卡住了。

## 根本原因
1. **参数名不匹配**：前端调用 `search_logs` 时使用了 `searchPath` 参数，但后端期望的是 `workspaceId`
2. **工作空间未加载**：后端使用全局 `path_map` 存储文件索引，但只有调用 `load_workspace` 后才会填充。如果工作空间未加载，`path_map` 为空，搜索无结果
3. **事件监听不完整**：前端没有监听 `search-start` 和 `search-progress` 事件
4. **错误处理不足**：搜索失败时没有显示详细错误信息

## 修复内容

### 1. 修正参数名（SearchPage.tsx）
```typescript
// 修复前
await invoke("search_logs", { 
  query, 
  searchPath: activeWorkspace.path,  // ❌ 错误的参数名
  filters: filters
});

// 修复后
await invoke("search_logs", { 
  query, 
  workspaceId: activeWorkspace.id,  // ✅ 正确的参数名
  filters: filters
});
```

### 2. 确保工作空间已加载（SearchPage.tsx）
```typescript
// 在搜索前检查工作空间状态
if (activeWorkspace.status !== 'READY') {
  addToast('error', `Workspace is ${activeWorkspace.status}. Please wait for it to be READY.`);
  return;
}

// 确保工作空间索引已加载到内存
await invoke('load_workspace', { workspaceId: activeWorkspace.id });
```

### 3. 后端错误检测（search.rs）
```rust
// 检测 path_map 是否为空
if guard.is_empty() {
    println!("[search_logs] ERROR: path_map is empty!");
    let _ = emit::search_error(format!(
        "Workspace '{}' is not loaded. Please load the workspace first.",
        workspace_id
    ));
    return;
}
```

### 4. 完善事件监听
- 添加 `search-start` 事件监听
- 添加 `search-progress` 事件监听
- 添加详细的控制台日志用于调试

### 5. 增强错误处理
- 添加查询验证（空查询检查）
- 添加工作空间状态检查
- 添加详细的控制台日志
- 改进错误提示信息

## 测试验证
1. 选择一个工作空间（确保状态为 READY）
2. 输入搜索关键词（如 "pass"）
3. 点击搜索按钮
4. 应该能看到：
   - 控制台输出详细的搜索日志
   - 工作空间自动加载（如果未加载）
   - 搜索结果正常显示
   - 搜索完成后显示成功提示

## 调试信息
修复后会在控制台输出以下日志：
- `[SearchPage] handleSearch called` - 搜索函数被调用
- `[SearchPage] Ensuring workspace is loaded...` - 正在加载工作空间
- `[search_logs] Command invoked` - 后端收到搜索命令
- `[search_logs] path_map size: X` - 索引文件数量
- `[Event] search-start` - 搜索开始
- `[Event] search-results` - 收到搜索结果
- `[Event] search-complete` - 搜索完成

## 相关文件
- `log-analyzer/src/pages/SearchPage.tsx` - 前端搜索页面
- `log-analyzer/src-tauri/src/commands/search.rs` - 后端搜索命令
- `log-analyzer/src-tauri/src/commands/workspace.rs` - 工作空间加载命令
- `log-analyzer/src-tauri/src/events/bridge.rs` - 事件桥接

## 注意事项
- 确保工作空间状态为 READY 才能搜索
- 后端使用全局 `path_map`，切换工作空间时会覆盖
- 搜索前会自动调用 `load_workspace` 确保索引已加载
- 如果 `path_map` 为空，会返回明确的错误信息
