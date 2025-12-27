#!/bin/bash

# Test Data Generator for CAS Migration Manual Testing
# This script creates test data for manual functional testing

set -e

echo "=== CAS Migration Test Data Generator ==="
echo ""

# Create base directory
TEST_DATA_DIR="test-data"
mkdir -p "$TEST_DATA_DIR"
cd "$TEST_DATA_DIR"

echo "Creating test data in: $(pwd)"
echo ""

# ============================================
# Test 1: Simple Folder Structure
# ============================================
echo "1. Creating simple folder structure..."
mkdir -p simple-folder/subfolder

cat > simple-folder/test1.log << 'EOF'
2024-01-01 10:00:00 INFO Application started
2024-01-01 10:00:01 INFO Loading configuration
2024-01-01 10:00:02 ERROR Failed to connect to database
2024-01-01 10:00:03 WARNING Retrying connection
2024-01-01 10:00:04 INFO Connection established
EOF

cat > simple-folder/test2.log << 'EOF'
2024-01-01 11:00:00 INFO Processing request
2024-01-01 11:00:01 DEBUG Request details: GET /api/users
2024-01-01 11:00:02 INFO Request completed successfully
2024-01-01 11:00:03 ERROR Unexpected error in handler
2024-01-01 11:00:04 INFO Shutting down gracefully
EOF

cat > simple-folder/subfolder/test3.log << 'EOF'
2024-01-01 12:00:00 INFO Subfolder log file
2024-01-01 12:00:01 WARNING This is a warning message
2024-01-01 12:00:02 ERROR This is an error message
2024-01-01 12:00:03 INFO Normal operation resumed
EOF

echo "   ✓ Created simple-folder/ with 3 log files"

# ============================================
# Test 2: Complex Folder Structure
# ============================================
echo "2. Creating complex folder structure..."
mkdir -p complex-folder/logs
mkdir -p complex-folder/data/cache

cat > complex-folder/README.md << 'EOF'
# Test Application Logs

This folder contains test logs for the CAS migration testing.

## Structure
- logs/ - Application logs
- data/ - Configuration and cache files
EOF

cat > complex-folder/logs/app.log << 'EOF'
2024-01-01 10:00:00 INFO [Main] Application starting
2024-01-01 10:00:01 INFO [Config] Loading configuration from config.json
2024-01-01 10:00:02 INFO [Database] Connecting to database
2024-01-01 10:00:03 ERROR [Database] Connection failed: timeout
2024-01-01 10:00:04 WARNING [Database] Retrying connection (attempt 1/3)
2024-01-01 10:00:05 INFO [Database] Connection successful
2024-01-01 10:00:06 INFO [Server] Starting HTTP server on port 8080
2024-01-01 10:00:07 INFO [Server] Server ready to accept connections
EOF

cat > complex-folder/logs/error.log << 'EOF'
2024-01-01 10:00:03 ERROR [Database] Connection failed: timeout
  at Database.connect (database.js:45)
  at Server.start (server.js:12)
  at main (index.js:5)
2024-01-01 11:30:15 ERROR [API] Request handler failed
  TypeError: Cannot read property 'id' of undefined
  at UserController.getUser (user.controller.js:23)
  at Router.handle (router.js:89)
EOF

cat > complex-folder/data/config.json << 'EOF'
{
  "database": {
    "host": "localhost",
    "port": 5432,
    "name": "testdb"
  },
  "server": {
    "port": 8080,
    "host": "0.0.0.0"
  }
}
EOF

cat > complex-folder/data/cache/temp.dat << 'EOF'
CACHE_DATA_12345
TIMESTAMP_1704110400
USER_SESSION_ABC123
EOF

echo "   ✓ Created complex-folder/ with nested structure"

# ============================================
# Test 3: Files with Duplicate Content
# ============================================
echo "3. Creating files with duplicate content (for deduplication test)..."
mkdir -p duplicate-test

cat > duplicate-test/file1.log << 'EOF'
This is identical content for deduplication testing.
Line 2 of identical content.
Line 3 of identical content.
EOF

# Create identical copy
cp duplicate-test/file1.log duplicate-test/file2.log
cp duplicate-test/file1.log duplicate-test/file3.log

echo "   ✓ Created duplicate-test/ with 3 identical files"

# ============================================
# Test 4: Simple Archive
# ============================================
echo "4. Creating simple archive..."
mkdir -p archive-content
cat > archive-content/log1.log << 'EOF'
2024-01-01 10:00:00 INFO Archive log file 1
2024-01-01 10:00:01 ERROR Error in archive
EOF

cat > archive-content/log2.log << 'EOF'
2024-01-01 11:00:00 INFO Archive log file 2
2024-01-01 11:00:01 WARNING Warning in archive
EOF

zip -q -r simple-archive.zip archive-content/
rm -rf archive-content/
echo "   ✓ Created simple-archive.zip"

# ============================================
# Test 5: Nested Archive
# ============================================
echo "5. Creating nested archive..."
mkdir -p nested-level1/nested-level2

cat > nested-level1/outer.log << 'EOF'
2024-01-01 10:00:00 INFO Outer level log
2024-01-01 10:00:01 INFO This is at level 1
EOF

cat > nested-level1/nested-level2/inner.log << 'EOF'
2024-01-01 10:00:00 INFO Inner level log
2024-01-01 10:00:01 INFO This is at level 2
EOF

# Create inner archive
cd nested-level1/nested-level2
zip -q inner.zip inner.log
rm inner.log
cd ../..

# Create outer archive
zip -q -r nested-archive.zip nested-level1/
rm -rf nested-level1/
echo "   ✓ Created nested-archive.zip (2 levels)"

# ============================================
# Test 6: Deep Nested Archive
# ============================================
echo "6. Creating deeply nested archive (3 levels)..."
mkdir -p deep-level1/deep-level2/deep-level3

cat > deep-level1/file-l1.log << 'EOF'
Level 1 file
EOF

cat > deep-level1/deep-level2/file-l2.log << 'EOF'
Level 2 file
EOF

cat > deep-level1/deep-level2/deep-level3/file-l3.log << 'EOF'
Level 3 file
EOF

# Create level 3 archive
cd deep-level1/deep-level2/deep-level3
zip -q level3.zip file-l3.log
rm file-l3.log
cd ..

# Create level 2 archive
zip -q -r level2.zip deep-level3/ file-l2.log
rm -rf deep-level3/ file-l2.log
cd ..

# Create level 1 archive
zip -q -r deep-nested.zip deep-level2/ file-l1.log
cd ..
rm -rf deep-level1/
echo "   ✓ Created deep-nested.zip (3 levels)"

# ============================================
# Test 7: Large Test Data
# ============================================
echo "7. Creating large test data..."
mkdir -p large-folder

for i in {1..100}; do
  cat > large-folder/log_$i.log << EOF
2024-01-01 10:00:00 INFO Log file $i
2024-01-01 10:00:01 DEBUG Processing item $i
2024-01-01 10:00:02 INFO Item $i processed successfully
2024-01-01 10:00:03 ERROR Error in item $i
2024-01-01 10:00:04 WARNING Warning for item $i
2024-01-01 10:00:05 INFO Completed item $i
EOF
done

echo "   ✓ Created large-folder/ with 100 log files"

# ============================================
# Test 8: Search Test Data
# ============================================
echo "8. Creating search test data..."
mkdir -p search-test

cat > search-test/errors.log << 'EOF'
2024-01-01 10:00:00 ERROR Database connection failed
2024-01-01 10:00:01 ERROR File not found: config.json
2024-01-01 10:00:02 ERROR Network timeout
2024-01-01 10:00:03 ERROR Invalid user credentials
2024-01-01 10:00:04 ERROR Out of memory
EOF

cat > search-test/warnings.log << 'EOF'
2024-01-01 10:00:00 WARNING Deprecated API usage
2024-01-01 10:00:01 WARNING High memory usage: 85%
2024-01-01 10:00:02 WARNING Slow query detected
2024-01-01 10:00:03 WARNING Cache miss rate high
EOF

cat > search-test/info.log << 'EOF'
2024-01-01 10:00:00 INFO Application started
2024-01-01 10:00:01 INFO User logged in: john@example.com
2024-01-01 10:00:02 INFO Request processed: GET /api/users
2024-01-01 10:00:03 INFO Cache hit: user_profile_123
2024-01-01 10:00:04 INFO Application shutdown initiated
EOF

cat > search-test/mixed.log << 'EOF'
2024-01-01 10:00:00 INFO Starting process
2024-01-01 10:00:01 DEBUG Loading configuration
2024-01-01 10:00:02 WARNING Configuration file is old
2024-01-01 10:00:03 ERROR Failed to parse configuration
2024-01-01 10:00:04 INFO Using default configuration
2024-01-01 10:00:05 INFO Process completed
EOF

echo "   ✓ Created search-test/ with categorized logs"

# ============================================
# Test 9: Different Archive Formats
# ============================================
echo "9. Creating different archive formats..."
mkdir -p format-test-content
cat > format-test-content/test.log << 'EOF'
Test log for archive format testing
EOF

# ZIP
zip -q format-test.zip format-test-content/test.log

# TAR
tar -czf format-test.tar.gz format-test-content/test.log

# TAR (uncompressed)
tar -cf format-test.tar format-test-content/test.log

# GZ (single file)
gzip -c format-test-content/test.log > format-test.log.gz

rm -rf format-test-content/
echo "   ✓ Created archives in multiple formats (.zip, .tar.gz, .tar, .gz)"

# ============================================
# Summary
# ============================================
echo ""
echo "=== Test Data Generation Complete ==="
echo ""
echo "Created test data:"
echo "  • simple-folder/          - Basic folder with 3 files"
echo "  • complex-folder/         - Complex nested structure"
echo "  • duplicate-test/         - Files with duplicate content"
echo "  • large-folder/           - 100 log files"
echo "  • search-test/            - Files for search testing"
echo "  • simple-archive.zip      - Basic ZIP archive"
echo "  • nested-archive.zip      - 2-level nested archive"
echo "  • deep-nested.zip         - 3-level nested archive"
echo "  • format-test.*           - Multiple archive formats"
echo ""
echo "Total size: $(du -sh . | cut -f1)"
echo ""
echo "You can now use this test data for manual testing."
echo "See TASK_33_MANUAL_TESTING_GUIDE.md for testing instructions."
