# Rust 依赖管理快速参考卡

## 常用命令速查

### 依赖更新
```bash
# 更新所有依赖
cargo update

# 更新特定依赖
cargo update -p serde

# 更新到最新兼容版本
cargo upgrade  # 需要 cargo-edit: cargo install cargo-edit
```

### 依赖检查
```bash
# 检查过期依赖
cargo outdated -R

# 检查重复依赖
cargo tree --duplicates

# 查看特定依赖的树
cargo tree -i hashbrown

# 查找未使用的依赖
cargo machete
```

### 审计与安全
```bash
# 运行 cargo-deny
cargo deny check

# 安全审计
cargo audit

# 检查许可证
cargo deny check licenses
```

## 关键依赖升级速查

### zip 0.6 → 2.x
```rust
// 关键变更
ZipWriter::write_all() // 现在返回 ZipResult<()>
DateTime::from_msdos() // 签名变更

// 新特性
zip64 support          // >4GB 文件
AES-256 encryption     // 更强加密
```

### sqlx 0.7 → 0.8
```toml
# runtime 和 tls 分离
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
```

### tokio features 精简
```toml
# 从
features = ["full"]

# 到
features = [
    "rt-multi-thread", "macros", "sync", 
    "time", "fs", "io-util", "parking_lot"
]
```

## 编译优化配置

### .cargo/config.toml
```toml
[build]
jobs = 0  # 使用所有 CPU

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/bin/zld"]
```

### Cargo.toml profiles
```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
strip = true
```

## 常见问题解决

### 版本冲突
```bash
# 查看冲突
cargo tree -i <crate>

# 强制更新
cargo update -p <crate>
```

### 编译错误
```bash
# 清理并重建
cargo clean && cargo build

# 检查 features
cargo tree -e features -i <crate>
```

### sqlx 准备
```bash
# 生成查询元数据（离线模式）
sqlx prepare

# 检查离线数据
sqlx prepare --check
```

## 工具安装

```bash
# 必需工具
cargo install cargo-deny
cargo install cargo-outdated
cargo install cargo-audit
cargo install cargo-machete

# 推荐工具
cargo install cargo-tree
cargo install cargo-bloat
cargo install cargo-edit
```

## 性能对比

| 指标 | 优化前 | 优化后 | 改善 |
|------|--------|--------|------|
| 编译时间 | 8m 30s | 6m 15s | -26% |
| 增量编译 | 45s | 30s | -33% |
| 二进制大小 | 45MB | 38MB | -15% |

## CI/CD 集成

```yaml
# GitHub Actions 关键步骤
- uses: Swatinem/rust-cache@v2  # 缓存
- uses: EmbarkStudios/cargo-deny-action@v2  # 审计
- uses: rustsec/audit-check@v1  # 安全检查
```

## 相关文档

- [完整迁移指南](./rust-dependency-optimization-guide.md)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [cargo-deny Book](https://embarkstudios.github.io/cargo-deny/)
