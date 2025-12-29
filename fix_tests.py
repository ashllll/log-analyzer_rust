#!/usr/bin/env python3
"""
批量修改E2E测试文件中的render调用，替换为await renderAppAndWait()
"""

import re
import sys


def fix_test_file(file_path):
    with open(file_path, "r", encoding="utf-8") as f:
        content = f.read()

    # 替换所有的render块为 await renderAppAndWait();
    # 匹配格式：
    #   render(
    #     <TestWrapper>
    #       <App />
    #     </TestWrapper>
    #   );
    pattern = r"      render\(\s*<TestWrapper>\s*<App\s*/>\s*</TestWrapper>\s*\);"

    replacement = "      await renderAppAndWait();"

    new_content = re.sub(pattern, replacement, content)

    # 检查是否有变化
    if new_content != content:
        with open(file_path, "w", encoding="utf-8") as f:
            f.write(new_content)
        print(f"Fixed {file_path}")
        return True
    else:
        print(f"No changes needed for {file_path}")
        return False


if __name__ == "__main__":
    files = [
        "log-analyzer/src/__tests__/e2e/CASMigrationWorkflows.test.tsx",
        "log-analyzer/src/__tests__/e2e/WorkspaceWorkflow.test.tsx",
        "log-analyzer/src/__tests__/e2e/VirtualFileTree.test.tsx",
    ]

    for file_path in files:
        try:
            fix_test_file(file_path)
        except Exception as e:
            print(f"Error processing {file_path}: {e}", file=sys.stderr)
