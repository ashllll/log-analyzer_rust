.PHONY: test check coverage clean shellcheck ci-autofix-loop ci-autofix-loop-push

test:
	cd log-analyzer/src-tauri && cargo test --workspace

check:
	cd log-analyzer/src-tauri && cargo check --workspace

coverage:
	cd log-analyzer/src-tauri && cargo tarpaulin --config tarpaulin.toml --out Html --out Lcov --output-dir coverage

clean:
	cd log-analyzer/src-tauri && cargo clean

shellcheck:
	shellcheck scripts/*.sh

ci-autofix-loop:
	scripts/ci-autofix-loop.sh

ci-autofix-loop-push:
	CI_AUTOFIX_PUSH=1 scripts/ci-autofix-loop.sh
