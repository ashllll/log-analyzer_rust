#!/usr/bin/env node
/**
 * IPC 一致性检查脚本
 *
 * 验证前端 invoke() 调用与后端 #[tauri::command] 函数的契约一致性：
 * 1. 前端调用的命令是否在后端存在
 * 2. 参数命名是否兼容（camelCase ↔ snake_case）
 * 3. 报告未被前端调用的后端命令（死代码提示）
 *
 * 用法: node scripts/check_ipc_consistency.js
 */

const fs = require('fs');
const path = require('path');

// ============================================================================
// 配置
// ============================================================================
const CONFIG = {
  backendDir: path.join(__dirname, '..', 'log-analyzer', 'src-tauri', 'src', 'commands'),
  frontendDir: path.join(__dirname, '..', 'log-analyzer', 'src'),
  // 忽略的文件/目录模式
  ignorePatterns: [
    /node_modules/,
    /\.test\./,
    /\.spec\./,
    /__tests__/,
    /__mocks__/,
  ],
};

// ============================================================================
// 工具函数
// ============================================================================

/**
 * camelCase 转 snake_case
 */
function toSnakeCase(str) {
  return str
    .replace(/([a-z])([A-Z])/g, '$1_$2')
    .replace(/([A-Z])([A-Z][a-z])/g, '$1_$2')
    .toLowerCase();
}

/**
 * snake_case 转 camelCase
 */
function toCamelCase(str) {
  return str.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
}

/**
 * 递归读取目录下所有匹配扩展名的文件
 */
function walkDir(dir, extensions, result = []) {
  if (!fs.existsSync(dir)) return result;

  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);

    if (entry.isDirectory()) {
      if (CONFIG.ignorePatterns.some((p) => p.test(fullPath))) continue;
      walkDir(fullPath, extensions, result);
    } else if (entry.isFile()) {
      const ext = path.extname(entry.name);
      if (extensions.includes(ext)) {
        result.push(fullPath);
      }
    }
  }
  return result;
}

/**
 * 读取文件内容
 */
function readFile(filePath) {
  return fs.readFileSync(filePath, 'utf-8');
}

// ============================================================================
// 后端命令提取
// ============================================================================

/**
 * 从 Rust 源文件中提取 #[tauri::command] 函数
 */
function extractBackendCommands(content) {
  const commands = [];
  // 匹配 #[command] 或 #[tauri::command] 及其变体后的函数定义
  const regex = /#\[\s*(?:tauri::)?command\s*\]\s*(?:#\[\s*\w+\s*\([^)]*\)\]\s*)*\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*\(([^)]*)\)/gs;
  let match;

  while ((match = regex.exec(content)) !== null) {
    const fnName = match[1];
    const paramsStr = match[2];

    // 解析参数名（忽略类型）
    const paramNames = [];
    // 匹配模式: name: Type 或 mut name: Type 或 _name: Type
    const paramRegex = /(?:(?:mut\s+)?|_?)([a-zA-Z_]\w*)\s*:/g;
    let paramMatch;
    while ((paramMatch = paramRegex.exec(paramsStr)) !== null) {
      const paramName = paramMatch[1];
      // 排除 Rust 关键字和 self
      if (['self', 'mut', 'ref'].includes(paramName)) continue;
      paramNames.push(paramName);
    }

    commands.push({
      name: fnName,
      params: paramNames,
    });
  }

  return commands;
}

// ============================================================================
// 前端调用提取
// ============================================================================

/**
 * 从 TypeScript 源文件中提取 invoke() 调用
 */
function extractFrontendInvokes(content) {
  const invokes = [];

  // 匹配 invoke('commandName', ...) 或 invoke("commandName", ...)
  // 支持: invoke('cmd'), invoke('cmd', { ... }), invoke('cmd', params), invoke('cmd', args as InvokeArgs)
  const regex = /invoke\s*\(\s*['"]([^'"]+)['"]\s*(?:,\s*([^)]*\)))?/g;
  let match;

  while ((match = regex.exec(content)) !== null) {
    const commandName = match[1];
    const argsSection = match[2] || '';

    // 提取参数名：只从对象字面量 { key: value, ... } 中提取
    const argNames = [];
    // 尝试匹配 { ... } 内的参数
    const objLiteralMatch = argsSection.match(/\{([^}]*)\}/);
    if (objLiteralMatch) {
      const objContent = objLiteralMatch[1];
      // 匹配对象字面量中的 key: value 或 key 简写
      // 注意：需要排除类型断言中的内容
      const cleanObjContent = objContent.replace(/as\s+\w+(<[^}]*>)?/g, '');
      const argRegex = /([a-zA-Z_]\w*)\s*:/g;
      let argMatch;
      while ((argMatch = argRegex.exec(cleanObjContent)) !== null) {
        argNames.push(argMatch[1]);
      }
    }

    invokes.push({
      name: commandName,
      args: argNames,
    });
  }

  return invokes;
}

// ============================================================================
// 主逻辑
// ============================================================================

function main() {
  console.log('=== IPC Consistency Check ===\n');

  let hasErrors = false;
  let hasWarnings = false;

  // --------------------------------------------------------------------------
  // 1. 提取后端命令
  // --------------------------------------------------------------------------
  const backendFiles = walkDir(CONFIG.backendDir, ['.rs']);
  const backendCommands = new Map(); // name -> { file, params: Set }

  for (const file of backendFiles) {
    const content = readFile(file);
    const commands = extractBackendCommands(content);
    for (const cmd of commands) {
      if (!backendCommands.has(cmd.name)) {
        backendCommands.set(cmd.name, {
          file: path.basename(file),
          params: new Set(cmd.params),
        });
      }
    }
  }

  console.log(`[1/4] Backend commands extracted: ${backendCommands.size} commands`);

  // --------------------------------------------------------------------------
  // 2. 提取前端调用
  // --------------------------------------------------------------------------
  const frontendFiles = walkDir(CONFIG.frontendDir, ['.ts', '.tsx']);
  const frontendInvokes = new Map(); // name -> [{ file, args: Set }]

  for (const file of frontendFiles) {
    const content = readFile(file);
    const invokes = extractFrontendInvokes(content);
    for (const inv of invokes) {
      if (!frontendInvokes.has(inv.name)) {
        frontendInvokes.set(inv.name, []);
      }
      frontendInvokes.get(inv.name).push({
        file: path.relative(path.join(__dirname, '..'), file),
        args: new Set(inv.args),
      });
    }
  }

  console.log(`[2/4] Frontend invokes extracted: ${frontendInvokes.size} unique commands`);

  // --------------------------------------------------------------------------
  // 3. 验证：前端命令是否在后端存在
  // --------------------------------------------------------------------------
  console.log('\n[3/4] Checking command existence...');
  const missingBackendCommands = [];

  for (const [cmdName, callSites] of frontendInvokes) {
    if (!backendCommands.has(cmdName)) {
      missingBackendCommands.push({
        name: cmdName,
        files: callSites.map((s) => s.file),
      });
    }
  }

  if (missingBackendCommands.length > 0) {
    hasErrors = true;
    console.log(`  ${'❌'.padEnd(2)} ERROR: ${missingBackendCommands.length} frontend command(s) not found in backend:`);
    for (const cmd of missingBackendCommands) {
      console.log(`      - ${cmd.name} (called in: ${cmd.files.join(', ')})`);
    }
  } else {
    console.log(`  ${'✅'.padEnd(2)} All ${frontendInvokes.size} frontend commands exist in backend`);
  }

  // --------------------------------------------------------------------------
  // 4. 验证：参数命名一致性（camelCase ↔ snake_case）
  // --------------------------------------------------------------------------
  console.log('\n[4/4] Checking parameter naming consistency...');
  const paramIssues = [];

  for (const [cmdName, callSites] of frontendInvokes) {
    const backendCmd = backendCommands.get(cmdName);
    if (!backendCmd) continue;

    for (const site of callSites) {
      for (const argName of site.args) {
        const snakeArg = toSnakeCase(argName);
        const camelArg = toCamelCase(argName);

        // 检查参数是否在后端存在（原生或转换后）
        const existsNative = backendCmd.params.has(argName);
        const existsAsSnake = backendCmd.params.has(snakeArg);
        const existsAsCamel = backendCmd.params.has(camelArg);

        if (!existsNative && !existsAsSnake && !existsAsCamel) {
          // 特殊处理：前端有时传递的是对象字面量嵌套参数，当前简单检查会误报
          // 这里只报告明显不匹配的参数（排除 config, query 等常见对象参数）
          const commonObjectParams = ['config', 'query', 'params', 'args', 'options', 'filters', 'logs'];
          if (!commonObjectParams.includes(argName) && argName !== 'workspaceId' && argName !== 'searchId') {
            paramIssues.push({
              command: cmdName,
              frontendParam: argName,
              expectedSnake: snakeArg,
              backendParams: Array.from(backendCmd.params),
              file: site.file,
            });
          }
        }
      }
    }
  }

  // 特殊检查已知问题
  const knownIssues = [
    {
      command: 'save_file_filter_config',
      frontendParam: 'filter_config',
      expected: 'filter_config',
      note: '前后端一致使用 snake_case，功能正常',
    },
  ];

  if (paramIssues.length > 0) {
    hasWarnings = true;
    console.log(`  ${'⚠️'.padEnd(2)} WARNING: ${paramIssues.length} potential parameter mismatch(es) found:`);
    for (const issue of paramIssues) {
      console.log(`      - ${issue.command}: frontend sends '${issue.frontendParam}', backend expects one of [${issue.backendParams.join(', ')}]`);
    }
  } else {
    console.log(`  ${'✅'.padEnd(2)} No obvious parameter mismatches detected`);
  }

  // 报告已知例外
  for (const issue of knownIssues) {
    console.log(`  ${'ℹ️'.padEnd(2)} NOTE: ${issue.command} uses '${issue.frontendParam}' - ${issue.note}`);
  }

  // --------------------------------------------------------------------------
  // 5. 报告未暴露的后端命令（信息级）
  // --------------------------------------------------------------------------
  console.log('\n[INFO] Backend commands not called from frontend:');
  let unusedCount = 0;
  for (const [cmdName, cmdInfo] of backendCommands) {
    if (!frontendInvokes.has(cmdName)) {
      unusedCount++;
      console.log(`  - ${cmdName} (${cmdInfo.file})`);
    }
  }
  console.log(`  Total: ${unusedCount} commands (may be internal or reserved for future use)`);

  // --------------------------------------------------------------------------
  // 6. 总结
  // --------------------------------------------------------------------------
  console.log('\n=== Summary ===');
  if (hasErrors) {
    console.log('Result: FAILED - Command existence errors found');
    process.exit(1);
  } else if (hasWarnings) {
    console.log('Result: PASSED with warnings - Review parameter mismatches above');
    process.exit(0);
  } else {
    console.log('Result: PASSED - All checks passed');
    process.exit(0);
  }
}

main();
