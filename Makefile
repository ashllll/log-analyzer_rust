.PHONY: test check coverage clean

test:
	cd log-analyzer/src-tauri && cargo test --workspace

check:
	cd log-analyzer/src-tauri && cargo check --workspace

coverage:
	cd log-analyzer/src-tauri && cargo tarpaulin --config tarpaulin.toml --out Html --out Lcov --output-dir coverage

clean:
	cd log-analyzer/src-tauri && cargo clean
