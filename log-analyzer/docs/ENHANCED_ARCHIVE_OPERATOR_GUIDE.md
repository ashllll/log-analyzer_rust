# Enhanced Archive Handling - Operator Guide

## Overview

This guide is for system operators and administrators responsible for deploying, configuring, and monitoring the Enhanced Archive Handling system.

## Table of Contents

1. [Installation and Deployment](#installation-and-deployment)
2. [Configuration Management](#configuration-management)
3. [Monitoring and Metrics](#monitoring-and-metrics)
4. [Security Operations](#security-operations)
5. [Performance Tuning](#performance-tuning)
6. [Backup and Recovery](#backup-and-recovery)
7. [Troubleshooting](#troubleshooting)
8. [Maintenance](#maintenance)

## Installation and Deployment

### System Requirements

**Minimum Requirements:**
- CPU: 2 cores
- RAM: 4GB
- Disk: 100GB free space
- OS: Windows 10+, Linux (kernel 4.x+), macOS 10.15+

**Recommended Requirements:**
- CPU: 4+ cores
- RAM: 8GB+
- Disk: 500GB+ SSD
- OS: Latest stable versions

### Installation Steps

#### 1. Install Dependencies

**Rust (1.70+)**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Node.js (18+)**:
```bash
# Using nvm
nvm install 18
nvm use 18
```

#### 2. Build the Application

```bash
cd log-analyzer
npm install
npm run tauri build
```

#### 3. Deploy Configuration

```bash
# Create configuration directory
mkdir -p config

# Copy default configuration
cp config/extraction_policy.toml.example config/extraction_policy.toml

# Edit configuration as needed
nano config/extraction_policy.toml
```

#### 4. Initialize Database

```bash
# Run migrations
cargo run --bin migrate_to_enhanced_archive -- \
  --new-db data/enhanced_archive.db \
  --workspace-root ./workspaces
```

#### 5. Start the Application

```bash
# Production mode
npm run tauri build
./target/release/log-analyzer

# Development mode
npm run tauri dev
```

### Migration from Legacy System

#### Pre-Migration Checklist

- [ ] Backup existing database
- [ ] Backup existing configuration
- [ ] Document current workspace structure
- [ ] Verify disk space (2x current usage)
- [ ] Schedule maintenance window
- [ ] Notify users of downtime

#### Migration Process

```bash
# 1. Backup old data
cp data/archive.db data/archive.db.backup
cp config/archive.json config/archive.json.backup

# 2. Run configuration migration
cargo run --bin config_migration -- \
  --old-config config/archive.json \
  --new-config config/extraction_policy.toml \
  --backup

# 3. Run data migration (dry run first)
cargo run --bin migrate_to_enhanced_archive -- \
  --old-db data/archive.db \
  --new-db data/enhanced_archive.db \
  --workspace-root ./workspaces \
  --dry-run

# 4. Review dry run output
cat migration.log

# 5. Run actual migration
cargo run --bin migrate_to_enhanced_archive -- \
  --old-db data/archive.db \
  --new-db data/enhanced_archive.db \
  --workspace-root ./workspaces

# 6. Verify migration
cargo test --test migration_verification

# 7. Update application configuration
# Edit config/app.toml to use new database path

# 8. Restart application
systemctl restart log-analyzer
```

#### Post-Migration Verification

```bash
# Check database integrity
sqlite3 data/enhanced_archive.db "PRAGMA integrity_check;"

# Verify path mappings
sqlite3 data/enhanced_archive.db "SELECT COUNT(*) FROM path_mappings;"

# Test extraction with sample archive
./scripts/test_extraction.sh

# Monitor logs for errors
tail -f logs/app.log
```

## Configuration Management

### Configuration File Structure

**Location**: `config/extraction_policy.toml`

```toml
[extraction]
# Maximum nesting depth (1-20)
max_depth = 10

# Maximum size for a single file (bytes)
max_file_size = 104857600  # 100MB

# Maximum total extraction size per archive (bytes)
max_total_size = 10737418240  # 10GB

# Maximum total size per workspace (bytes)
max_workspace_size = 53687091200  # 50GB

# Number of concurrent extractions (default: CPU cores / 2)
concurrent_extractions = 4

# Buffer size for streaming extraction (bytes)
buffer_size = 65536  # 64KB

[security]
# Compression ratio threshold for zip bomb detection
compression_ratio_threshold = 100.0

# Exponential backoff threshold for nested archives
exponential_backoff_threshold = 1000000.0

# Enable zip bomb detection
enable_zip_bomb_detection = true

[paths]
# Enable long path support (Windows UNC prefix)
enable_long_paths = true

# Threshold for automatic path shortening (0.0-1.0)
# 0.8 = shorten when path reaches 80% of OS limit
shortening_threshold = 0.8

# Hash algorithm for path shortening
hash_algorithm = "SHA256"

# Length of hash in shortened paths
hash_length = 16

[performance]
# Temporary directory TTL in hours
temp_dir_ttl_hours = 24

# Log retention period in days
log_retention_days = 90

# Enable streaming extraction
enable_streaming = true

[audit]
# Enable audit logging
enable_audit_logging = true

# Log format: json, text, pretty
log_format = "json"

# Log level: trace, debug, info, warn, error
log_level = "info"
```

### Configuration Validation

```bash
# Validate configuration file
cargo run --bin validate_config -- config/extraction_policy.toml

# Test configuration with sample archive
cargo run --bin test_config -- \
  --config config/extraction_policy.toml \
  --archive test_data/sample.zip
```

### Hot Reload Configuration

The system supports hot reloading of configuration without restart:

```bash
# Send SIGHUP to reload configuration
kill -HUP $(pidof log-analyzer)

# Or use the API
curl -X POST http://localhost:8080/api/config/reload
```

### Environment-Specific Configurations

**Development**:
```toml
[extraction]
max_depth = 5
max_file_size = 10485760  # 10MB
concurrent_extractions = 2

[audit]
log_level = "debug"
```

**Production**:
```toml
[extraction]
max_depth = 10
max_file_size = 104857600  # 100MB
concurrent_extractions = 8

[audit]
log_level = "info"
enable_audit_logging = true
```

**High-Security**:
```toml
[security]
compression_ratio_threshold = 50.0
exponential_backoff_threshold = 100000.0
enable_zip_bomb_detection = true

[audit]
log_level = "info"
enable_audit_logging = true
log_format = "json"
```

## Monitoring and Metrics

### Key Metrics to Monitor

#### System Metrics

1. **CPU Usage**: Should stay below 80% during normal operations
2. **Memory Usage**: Monitor for memory leaks (should be stable)
3. **Disk I/O**: High during extraction, low otherwise
4. **Disk Space**: Alert when workspace exceeds 80% capacity

#### Application Metrics

1. **Extraction Rate**: Files/second, MB/second
2. **Active Extractions**: Number of concurrent operations
3. **Queue Depth**: Pending extraction requests
4. **Error Rate**: Failed extractions per hour
5. **Security Events**: Zip bombs detected, path traversal attempts

### Monitoring Tools

#### Built-in Metrics Endpoint

```bash
# Get current metrics
curl http://localhost:8080/api/metrics

# Example response
{
  "extractions": {
    "active": 3,
    "queued": 5,
    "completed_today": 150,
    "failed_today": 2
  },
  "performance": {
    "avg_extraction_speed_mbps": 75.5,
    "avg_extraction_time_seconds": 45.2
  },
  "security": {
    "zip_bombs_detected_today": 0,
    "suspicious_files_flagged": 3
  },
  "storage": {
    "total_workspace_size_gb": 125.5,
    "path_mappings_count": 15420
  }
}
```

#### Log Analysis

**Structured JSON Logs**:
```bash
# Count extractions by status
cat logs/app.log | jq -r 'select(.event=="extraction_complete") | .status' | sort | uniq -c

# Average extraction time
cat logs/app.log | jq -r 'select(.event=="extraction_complete") | .duration_seconds' | awk '{sum+=$1; count++} END {print sum/count}'

# Security events
cat logs/app.log | jq -r 'select(.level=="WARN" and .category=="security")'
```

#### Prometheus Integration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'log-analyzer'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/api/metrics/prometheus'
```

#### Grafana Dashboard

Import the provided dashboard: `monitoring/grafana-dashboard.json`

**Key Panels**:
- Extraction throughput over time
- Active vs queued extractions
- Error rate and types
- Security events timeline
- Disk space usage by workspace

### Alerting Rules

#### Critical Alerts

```yaml
# Disk space critical
- alert: DiskSpaceCritical
  expr: workspace_disk_usage_percent > 90
  for: 5m
  annotations:
    summary: "Workspace disk usage above 90%"

# High error rate
- alert: HighExtractionErrorRate
  expr: rate(extraction_errors_total[5m]) > 0.1
  for: 10m
  annotations:
    summary: "Extraction error rate above 10%"

# Zip bomb detected
- alert: ZipBombDetected
  expr: increase(zip_bombs_detected_total[1h]) > 0
  annotations:
    summary: "Zip bomb detected in last hour"
    severity: "critical"
```

#### Warning Alerts

```yaml
# High queue depth
- alert: HighQueueDepth
  expr: extraction_queue_depth > 50
  for: 15m
  annotations:
    summary: "Extraction queue depth above 50"

# Slow extraction speed
- alert: SlowExtractionSpeed
  expr: avg_extraction_speed_mbps < 20
  for: 30m
  annotations:
    summary: "Average extraction speed below 20 MB/s"
```

## Security Operations

### Security Event Types

1. **Zip Bomb Detected**: Compression ratio exceeds threshold
2. **Path Traversal Attempt**: Archive contains `../` in paths
3. **Forbidden Extension**: Executable or script file detected
4. **Excessive File Count**: Archive contains > 1 million files
5. **Symlink Cycle**: Circular symlink reference detected

### Security Audit Log

**Location**: `logs/security_audit.log`

**Format**:
```json
{
  "timestamp": "2024-01-15T10:30:45Z",
  "event_type": "zip_bomb_detected",
  "severity": "critical",
  "user_id": "user123",
  "workspace_id": "ws456",
  "archive_path": "/uploads/suspicious.zip",
  "details": {
    "compression_ratio": 1500.0,
    "nesting_depth": 3,
    "risk_score": 3375000.0
  },
  "action_taken": "extraction_halted"
}
```

### Security Best Practices

1. **Enable All Security Features**:
   ```toml
   [security]
   enable_zip_bomb_detection = true
   compression_ratio_threshold = 100.0
   ```

2. **Monitor Security Logs Daily**:
   ```bash
   # Daily security report
   ./scripts/security_report.sh --date today
   ```

3. **Regular Security Audits**:
   ```bash
   # Run security audit
   cargo run --bin security_audit -- \
     --workspace-root ./workspaces \
     --report-path reports/security_audit_$(date +%Y%m%d).pdf
   ```

4. **Incident Response**:
   - Isolate affected workspace
   - Review security audit logs
   - Analyze suspicious archive
   - Update security rules if needed
   - Document incident

### Compliance

#### Audit Trail Requirements

- All extraction operations logged with user ID
- Security events logged at WARN level
- Logs retained for 90 days (configurable)
- Structured JSON format for automated analysis

#### Data Retention

```bash
# Configure retention policy
[audit]
log_retention_days = 90

# Manual cleanup of old logs
find logs/ -name "*.log" -mtime +90 -delete

# Archive old logs
tar -czf logs_archive_$(date +%Y%m).tar.gz logs/*.log.$(date +%Y-%m-*)
```

## Performance Tuning

### CPU Optimization

```toml
[extraction]
# Set to CPU cores / 2 for balanced performance
concurrent_extractions = 4

# For CPU-intensive workloads, reduce to avoid contention
concurrent_extractions = 2

# For I/O-bound workloads, can increase
concurrent_extractions = 8
```

### Memory Optimization

```toml
[extraction]
# Reduce buffer size if memory is limited
buffer_size = 32768  # 32KB

# Increase for better performance with available RAM
buffer_size = 131072  # 128KB
```

### Disk I/O Optimization

1. **Use SSD Storage**: 3-5x faster than HDD
2. **Separate Temp Directory**: Use different disk for temp files
3. **Batch Directory Creation**: Enabled by default
4. **Streaming Extraction**: Enabled by default

```toml
[performance]
enable_streaming = true
temp_dir_ttl_hours = 24
```

### Network Storage Considerations

If using network storage (NFS, SMB):

```toml
[extraction]
# Reduce concurrent extractions to avoid network saturation
concurrent_extractions = 2

# Increase buffer size to reduce network round-trips
buffer_size = 262144  # 256KB
```

### Performance Benchmarks

Run benchmarks to establish baseline:

```bash
# Run extraction benchmarks
cargo bench --bench extraction_benchmarks

# Results location
cat target/criterion/extraction_speed/report/index.html
```

**Expected Performance**:
- Extraction speed: 50-100 MB/s (SSD)
- Memory usage: < 100MB per concurrent extraction
- CPU usage: 50-70% during active extraction

## Backup and Recovery

### What to Backup

1. **Database**: `data/enhanced_archive.db`
2. **Configuration**: `config/extraction_policy.toml`
3. **Workspaces**: `workspaces/` (optional, can be large)
4. **Logs**: `logs/` (for audit trail)

### Backup Strategy

#### Daily Backups

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backups/log-analyzer/$(date +%Y%m%d)"
mkdir -p "$BACKUP_DIR"

# Backup database
sqlite3 data/enhanced_archive.db ".backup '$BACKUP_DIR/enhanced_archive.db'"

# Backup configuration
cp config/extraction_policy.toml "$BACKUP_DIR/"

# Backup logs (last 7 days)
find logs/ -name "*.log" -mtime -7 -exec cp {} "$BACKUP_DIR/logs/" \;

# Compress backup
tar -czf "$BACKUP_DIR.tar.gz" "$BACKUP_DIR"
rm -rf "$BACKUP_DIR"
```

#### Automated Backup with Cron

```cron
# Run daily at 2 AM
0 2 * * * /opt/log-analyzer/scripts/backup.sh
```

### Recovery Procedures

#### Database Recovery

```bash
# Stop application
systemctl stop log-analyzer

# Restore database
cp /backups/log-analyzer/20240115/enhanced_archive.db data/

# Verify integrity
sqlite3 data/enhanced_archive.db "PRAGMA integrity_check;"

# Start application
systemctl start log-analyzer
```

#### Configuration Recovery

```bash
# Restore configuration
cp /backups/log-analyzer/20240115/extraction_policy.toml config/

# Validate configuration
cargo run --bin validate_config -- config/extraction_policy.toml

# Reload configuration
kill -HUP $(pidof log-analyzer)
```

## Troubleshooting

### Common Issues

#### Issue: High Memory Usage

**Symptoms**: Memory usage continuously increasing

**Diagnosis**:
```bash
# Check memory usage
ps aux | grep log-analyzer

# Check for memory leaks
valgrind --leak-check=full ./target/release/log-analyzer
```

**Solutions**:
- Reduce `concurrent_extractions`
- Reduce `buffer_size`
- Check for stuck extractions
- Restart application

#### Issue: Slow Extraction Performance

**Symptoms**: Extraction speed < 20 MB/s

**Diagnosis**:
```bash
# Check disk I/O
iostat -x 1

# Check CPU usage
top -p $(pidof log-analyzer)

# Check extraction metrics
curl http://localhost:8080/api/metrics | jq '.performance'
```

**Solutions**:
- Move to SSD storage
- Increase `buffer_size`
- Reduce `concurrent_extractions` if CPU-bound
- Check network latency if using network storage

#### Issue: Database Locked Errors

**Symptoms**: "database is locked" errors in logs

**Diagnosis**:
```bash
# Check for long-running transactions
sqlite3 data/enhanced_archive.db "SELECT * FROM sqlite_master;"

# Check file locks
lsof data/enhanced_archive.db
```

**Solutions**:
- Increase SQLite timeout
- Reduce concurrent database access
- Check for zombie processes
- Consider using WAL mode

### Debug Mode

Enable debug logging for troubleshooting:

```toml
[audit]
log_level = "debug"
```

```bash
# Restart with debug logging
RUST_LOG=debug ./target/release/log-analyzer

# Filter debug logs
tail -f logs/app.log | grep DEBUG
```

### Support Information Collection

```bash
# Collect support bundle
./scripts/collect_support_info.sh

# Bundle includes:
# - Configuration files
# - Recent logs (last 24 hours)
# - System information
# - Database statistics
# - Performance metrics
```

## Maintenance

### Regular Maintenance Tasks

#### Daily
- [ ] Monitor disk space
- [ ] Review error logs
- [ ] Check security events

#### Weekly
- [ ] Review performance metrics
- [ ] Clean up old temporary files
- [ ] Verify backups

#### Monthly
- [ ] Database optimization
- [ ] Log rotation and archival
- [ ] Security audit
- [ ] Performance benchmarking

### Database Maintenance

```bash
# Vacuum database (reclaim space)
sqlite3 data/enhanced_archive.db "VACUUM;"

# Analyze database (update statistics)
sqlite3 data/enhanced_archive.db "ANALYZE;"

# Check integrity
sqlite3 data/enhanced_archive.db "PRAGMA integrity_check;"
```

### Log Rotation

```bash
# Rotate logs manually
./scripts/rotate_logs.sh

# Or configure logrotate
# /etc/logrotate.d/log-analyzer
/opt/log-analyzer/logs/*.log {
    daily
    rotate 90
    compress
    delaycompress
    notifempty
    create 0640 loganalyzer loganalyzer
    sharedscripts
    postrotate
        kill -HUP $(pidof log-analyzer)
    endscript
}
```

### Cleanup Old Data

```bash
# Clean up old path mappings (unused for 90+ days)
sqlite3 data/enhanced_archive.db "
DELETE FROM path_mappings 
WHERE created_at < strftime('%s', 'now', '-90 days')
AND access_count = 0;
"

# Clean up old temporary files
find /tmp/log-analyzer-* -mtime +1 -delete
```

## Appendix

### Configuration Reference

See [Configuration Management](#configuration-management) section.

### API Reference

See `docs/API.md` for complete API documentation.

### Metrics Reference

See [Monitoring and Metrics](#monitoring-and-metrics) section.

### Security Event Codes

| Code | Event Type | Severity | Action |
|------|-----------|----------|--------|
| SEC001 | Zip Bomb Detected | Critical | Halt extraction |
| SEC002 | Path Traversal | High | Skip file |
| SEC003 | Forbidden Extension | Medium | Skip file |
| SEC004 | Excessive File Count | High | Halt extraction |
| SEC005 | Symlink Cycle | Medium | Skip symlink |

### Support Contacts

- Technical Support: support@example.com
- Security Issues: security@example.com
- Documentation: docs@example.com
