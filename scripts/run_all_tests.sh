#!/bin/bash
set -e

echo "========================================="
echo "Running Log Analyzer Test Suite"
echo "========================================="

cd log-analyzer/src-tauri

echo ""
echo "1. Running cargo check..."
cargo check --all-features

echo ""
echo "2. Running unit tests..."
cargo test --lib -- --nocapture

echo ""
echo "3. Running FFI integration tests..."
cargo test --test ffi_integration_tests -- --nocapture

echo ""
echo "4. Running CAS concurrent tests..."
cargo test --test cas_concurrent_tests -- --nocapture

echo ""
echo "5. Running search async tests..."
cargo test --test search_async_tests -- --nocapture

echo ""
echo "6. Running cargo clippy..."
cargo clippy --all-features -- -D warnings

echo ""
echo "========================================="
echo "All tests passed!"
echo "========================================="
