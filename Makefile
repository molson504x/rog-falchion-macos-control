# Developer entry points. CI runs the same cargo commands.
CARGO      ?= cargo
DIST       ?= dist
EXE        ?=
# Files that need real hardware or a display; excluded from coverage.
COV_IGNORE := '(main|app|backend)\.rs$$'

.PHONY: build test lint fmt coverage coverage-html run-cli run-gui dist clean

build:
	$(CARGO) build --workspace

test:
	$(CARGO) test --workspace --all-targets

lint:
	$(CARGO) fmt --all --check
	$(CARGO) clippy --workspace --all-targets -- -D warnings

fmt:
	$(CARGO) fmt --all

coverage:
	$(CARGO) llvm-cov --workspace --ignore-filename-regex $(COV_IGNORE) --fail-under-lines 90

coverage-html:
	$(CARGO) llvm-cov --workspace --ignore-filename-regex $(COV_IGNORE) --html --open

run-cli:
	$(CARGO) run -p falchion-cli -- $(ARGS)

run-gui:
	$(CARGO) run -p falchion-gui

dist:
	$(CARGO) build --workspace --release
	mkdir -p $(DIST)
	cp target/release/falchion$(EXE) target/release/falchion-gui$(EXE) $(DIST)/

clean:
	$(CARGO) clean
	find $(DIST) -mindepth 1 ! -name .gitkeep -delete
