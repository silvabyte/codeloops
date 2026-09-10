PREFIX ?= $(HOME)/.local/codeloops-history-preview
CARGO ?= cargo
PROFILE ?= preview
DATA_DIR ?= $(HOME)/.local/share/codeloops-history/$(PROFILE)
ADDRESS ?= 127.0.0.1:47823
SETUP_ARGS ?=

.PHONY: check install setup run e2e uninstall
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
	target/release/codeloops install --prefix "$(PREFIX)"

setup:
	"$(PREFIX)/bin/codeloops" --data-dir "$(DATA_DIR)" --address "$(ADDRESS)" setup --profile "$(PROFILE)" $(SETUP_ARGS)

run:
	"$(PREFIX)/bin/codeloops" serve

e2e:
	$(CARGO) build --release --locked -p codeloops
	CODELOOPS_E2E_BINARY="$(CURDIR)/target/release/codeloops" $(CARGO) test --locked -p codeloops --test installation -- --nocapture

uninstall:
	@if test -x "$(PREFIX)/bin/codeloops"; then \
		"$(PREFIX)/bin/codeloops" uninstall; \
	elif test -f "$(PREFIX)/share/codeloops/setup.json" || \
		test -f "$(PREFIX)/share/codeloops/install.json" || \
		test -f "$(PREFIX)/share/codeloops/install-pending.json"; then \
		$(CARGO) run --locked -p codeloops -- uninstall --prefix "$(PREFIX)"; \
	fi
