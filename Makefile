PREFIX ?= $(HOME)/.local/codeloops-history-preview
CARGO ?= cargo
PROFILE ?= preview
DATA_DIR ?= $(HOME)/.local/share/codeloops-history/$(PROFILE)
ADDRESS ?= 127.0.0.1:47823
SETUP_ARGS ?=

.PHONY: check install setup start stop status logs run e2e e2e-service uninstall
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
	@printf '%s\n' 'Building CodeLoops (the first build can take a few minutes)...' >&2
	@$(CARGO) build --release --locked -p codeloops
	@printf 'Installing CodeLoops into %s...\n' "$(PREFIX)" >&2
	@target/release/codeloops install --prefix "$(PREFIX)" > /dev/null

setup: install
	@printf '%s\n' 'Registering capture and MCP for Cursor and OpenCode...' >&2
	@"$(PREFIX)/bin/codeloops" --data-dir "$(DATA_DIR)" --address "$(ADDRESS)" setup --profile "$(PROFILE)" $(SETUP_ARGS) > /dev/null
	@printf '%s\n' 'Capture and MCP registered.' >&2

start: setup
	@"$(PREFIX)/bin/codeloops" service start

stop status logs:
	@"$(PREFIX)/bin/codeloops" service $@

run:
	@if ! test -x "$(PREFIX)/bin/codeloops"; then \
		printf '%s\n' 'CodeLoops is not installed. Run make start to build, configure, and launch it.' >&2; \
		exit 1; \
	fi
	@printf '%s\n' 'Starting the history service...' >&2
	@printf '%s\n' 'Foreground development mode. Ctrl+C stops the service; use make start for background operation.' >&2
	@exec "$(PREFIX)/bin/codeloops" serve

e2e:
	$(CARGO) build --release --locked -p codeloops
	CODELOOPS_E2E_BINARY="$(CURDIR)/target/release/codeloops" $(CARGO) test --locked -p codeloops --test installation -- --nocapture

e2e-service:
	$(CARGO) build --release --locked -p codeloops
	python3 crates/codeloops/tests/native_service.py

uninstall:
	@if test -x "$(PREFIX)/bin/codeloops"; then \
		"$(PREFIX)/bin/codeloops" uninstall; \
	elif test -f "$(PREFIX)/share/codeloops/setup.json" || \
		test -f "$(PREFIX)/share/codeloops/service.json" || \
		test -f "$(PREFIX)/share/codeloops/install.json" || \
		test -f "$(PREFIX)/share/codeloops/install-pending.json"; then \
		$(CARGO) run --locked -p codeloops -- uninstall --prefix "$(PREFIX)"; \
	fi
