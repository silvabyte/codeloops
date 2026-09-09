PREFIX ?= $(HOME)/.local/codeloops-history-preview
CARGO ?= cargo

.PHONY: check install run
check: node_modules/.package-lock.json
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings
	$(CARGO) test --workspace --locked
	npm run typecheck
	npm run lint
	npm test

node_modules/.package-lock.json: package.json package-lock.json
	npm ci

install:
	$(CARGO) build --release --locked -p codeloops
	install -d "$(PREFIX)/bin" "$(PREFIX)/share/codeloops/adapters/opencode" "$(PREFIX)/share/codeloops/adapters/cursor"
	install -m 755 target/release/codeloops "$(PREFIX)/bin/codeloops"
	install -m 644 adapters/opencode/history.ts "$(PREFIX)/share/codeloops/adapters/opencode/history.ts"
	install -m 644 adapters/cursor/hooks.example.json "$(PREFIX)/share/codeloops/adapters/cursor/hooks.example.json"

run:
	$(CARGO) run --locked -p codeloops -- serve
