.PHONY: help build build-release test lint fmt check install clean release

help:
	@echo "om - LLM Context Tool"
	@echo ""
	@echo "Available targets:"
	@echo "  build          - Build debug binary"
	@echo "  build-release  - Build release binary"
	@echo "  test           - Run all tests"
	@echo "  lint           - Run clippy"
	@echo "  fmt            - Format code"
	@echo "  check          - Run fmt + lint + test"
	@echo "  install        - Install to /usr/local/bin"
	@echo "  clean          - Clean build artifacts"
	@echo "  release        - Create release tarball"

build:
	cargo build

build-release:
	cargo build --release

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

check: fmt lint test
	@echo "All checks passed!"

install: build-release
	sudo cp target/release/om /usr/local/bin/
	@echo "Installed om to /usr/local/bin/om"

clean:
	cargo clean
	rm -rf releases/

release: build-release
	@mkdir -p releases
	@VERSION=$$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[0].version'); \
	TARBALL="releases/om-$$VERSION-$$(uname -s)-$$(uname -m).tar.gz"; \
	tar -czf $$TARBALL -C target/release om; \
	echo "Created $$TARBALL"
