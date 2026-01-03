"""
文件类型过滤功能 - 测试数据生成器

用法:
    python generate_test_data.py
"""

import os
from pathlib import Path

def create_test_data():
    # 创建测试目录
    base_dir = Path("test_data")
    base_dir.mkdir(exist_ok=True)

    # 1. 创建日志文件（应该被导入）
    logs_dir = base_dir / "logs"
    logs_dir.mkdir(exist_ok=True)

    (logs_dir / "app.log").write_text("""2024-01-01 12:00:00 INFO Application started
2024-01-01 12:00:01 ERROR Database connection failed
2024-01-01 12:00:02 WARN Retrying connection
2024-01-01 12:00:03 INFO Connection established
""")

    (logs_dir / "syslog").write_text("""Jan  1 12:00:00 server app[123]: Started
Jan  1 12:00:01 server app[123]: Error: Connection failed
Jan  1 12:00:02 server app[123]: Warning: High memory usage
""")

    (logs_dir / "messages").write_text("""Jan  1 12:00:00 kernel: [    0.000000] Linux version 6.1.0
Jan  1 12:00:01 server sshd[1234]: Accepted password for user
Jan  1 12:00:02 server cron[5678]: (root) CMD (run-parts /etc/cron.hourly)
""")

    (logs_dir / "error.log").write_text("""[ERROR] 2024-01-01 12:00:00 Failed to connect
[ERROR] 2024-01-01 12:00:01 Timeout occurred
[ERROR] 2024-01-01 12:00:02 Retry attempt 1 failed
""")

    (logs_dir / "access.2024-01-01").write_text("""127.0.0.1 - - [01/Jan/2024:12:00:00 +0000] "GET /api HTTP/1.1" 200 1234
127.0.0.1 - - [01/Jan/2024:12:00:01 +0000] "POST /api/data HTTP/1.1" 201 567
127.0.0.1 - - [01/Jan/2024:12:00:02 +0000] "GET /api/status HTTP/1.1" 200 89
""")

    (logs_dir / "debug.20250103").write_text("""[2025-01-03 10:00:00] DEBUG: Initializing module
[2025-01-03 10:00:01] DEBUG: Loading configuration
[2025-01-03 10:00:02] DEBUG: Starting worker threads
""")

    # 2. 创建二进制文件（应该被第1层拒绝）
    binary_dir = base_dir / "binary_files"
    binary_dir.mkdir(exist_ok=True)

    # PNG文件（8字节PNG魔数 + 其他数据）
    (binary_dir / "image.png").write_bytes(b'\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde')

    # JPEG文件（3字节JPEG魔数 + 其他数据）
    (binary_dir / "photo.jpg").write_bytes(b'\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00')

    # EXE文件（2字节EXE魔数 + 其他数据）
    (binary_dir / "program.exe").write_bytes(b'MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00\xff\xff\x00\x00\xb8\x00\x00\x00\x00\x00\x00\x00@')

    # MP3文件（3字节MP3魔数 + 其他数据）
    (binary_dir / "audio.mp3").write_bytes(b'ID3\x03\x00\x00\x00\x00\x00\x00\x00')

    # PDF文件（4字节PDF魔数 + 其他数据）
    (binary_dir / "document.pdf").write_bytes(b'%PDF-1.4\n1 0 obj\n<<')

    # 3. 创建文本文件（根据配置决定）
    text_dir = base_dir / "text_files"
    text_dir.mkdir(exist_ok=True)

    (text_dir / "data.csv").write_text("""id,name,value
1,Item1,100
2,Item2,200
3,Item3,300
""")

    (text_dir / "config.json").write_text("""{
  "app": "LogAnalyzer",
  "version": "1.0.0",
  "features": ["search", "filter", "export"]
}
""")

    (text_dir / "readme.txt").write_text("""# Log Analyzer

This is a readme file.
It contains plain text documentation.
""")

    (text_dir / "notes.md").write_text("""# Notes

## Feature List
- File type filtering
- Three-layer detection strategy
- Defensive design
""")

    # 4. 创建混合文件（用于测试边缘情况）
    mixed_dir = base_dir / "mixed"
    mixed_dir.mkdir(exist_ok=True)

    # 无扩展名的日志文件
    (mixed_dir / "stdout").write_text("""Container stdout log
Line 2: Application started
Line 3: Ready to serve requests
""")

    (mixed_dir / "stderr").write_text("""Container stderr log
ERROR: Connection failed
WARN: Retrying...
""")

    # 带日期的日志
    (mixed_dir / "application.2024-12-25").write_text("""2024-12-25 Log entry 1
2024-12-25 Log entry 2
""")

    (mixed_dir / "mylog.txt").write_text("""Custom log file with txt extension
This should match *log* pattern
""")

    print(f"✅ 测试数据创建成功: {base_dir.absolute()}")
    print("\n📁 文件结构:")
    print_tree(base_dir)

    print("\n\n📋 测试说明:")
    print("=" * 60)
    print("✅ 应该被导入的文件:")
    print("  - logs/*.log (匹配 *log* 模式)")
    print("  - logs/syslog (匹配 syslog 模式)")
    print("  - logs/messages (匹配 messages 模式)")
    print("  - logs/access.2024-01-01 (匹配 *.20* 模式)")
    print()
    print("❌ 应该被拒绝的文件 (二进制检测):")
    print("  - binary_files/*.png (PNG魔数)")
    print("  - binary_files/*.jpg (JPEG魔数)")
    print("  - binary_files/*.exe (EXE魔数)")
    print("  - binary_files/*.mp3 (MP3魔数)")
    print("  - binary_files/*.pdf (PDF魔数)")
    print()
    print("⚠️  根据配置决定:")
    print("  - text_files/* (取决于白名单/黑名单配置)")
    print("  - mixed/* (取决于文件名模式和扩展名)")
    print("=" * 60)

def print_tree(directory, prefix="", max_depth=5, current_depth=0):
    """打印目录树结构"""
    if current_depth >= max_depth:
        return

    try:
        entries = sorted(directory.iterdir(), key=lambda x: (not x.is_dir(), x.name))
    except PermissionError:
        return

    entries = list(entries)
    for i, entry in enumerate(entries):
        is_last = i == len(entries) - 1
        current_prefix = "└── " if is_last else "├── "
        print(f"{prefix}{current_prefix}{entry.name}")

        if entry.is_dir():
            extension = "    " if is_last else "│   "
            print_tree(entry, prefix + extension, max_depth, current_depth + 1)

if __name__ == "__main__":
    create_test_data()
