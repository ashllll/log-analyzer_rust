#!/usr/bin/env python3
import re
import sys

def fix_match_lock(content):
    """修复 match ...lock() { Ok(...) => ..., Err(...) => ... } 模式"""
    # 匹配 match 语句
    pattern = r'match\s+([^.]+)\.lock\(\)\s*\{\s*Ok\(([^)]+)\)\s*=>'
    
    def replace_match(match):
        var_name = match.group(1).strip()
        guard_name = match.group(2).strip()
        return f'{{\n        let {guard_name} = {var_name}.lock();'
    
    # 先替换 match 开头
    content = re.sub(pattern, replace_match, content)
    
    # 移除 Err 分支 (需要手动处理复杂情况)
    # 这里只处理简单的情况
    
    return content

def fix_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        original = content
        
        # 修复 match 语句
        content = fix_match_lock(content)
        
        if content != original:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"Fixed: {filepath}")
            return True
        else:
            print(f"No changes: {filepath}")
            return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
        return False

if __name__ == "__main__":
    files = [
        "log-analyzer/src-tauri/src/commands/workspace.rs",
        "log-analyzer/src-tauri/src/commands/watch.rs",
    ]
    
    for f in files:
        fix_file(f)
