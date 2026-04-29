# Firmament Development Justfile

# Install all development dependencies
setup:
    rustup show
    cargo install cargo-deny --locked
    cargo install mdbook --locked
    cargo install prek --locked
    cargo fetch
    prek install

# Run pre-commit hooks on all files
lint:
    prek run --all-files

# Format all code
fmt:
    cargo fmt --all

# Run clippy lints
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run all tests
test *ARGS:
    cargo test --workspace --all-features {{ ARGS }}

# Run dependency audit
deny:
    cargo deny check

# Build API docs
doc:
    cargo doc --workspace --all-features --no-deps

# mdBook documentation: build, dev (default: build)
book variant="build":
    @if [ "{{ variant }}" = "build" ]; then \
        mdbook build docs; \
    elif [ "{{ variant }}" = "dev" ]; then \
        mdbook serve docs --open; \
    else \
        echo "Unknown variant '{{ variant }}'. Use: build, dev"; \
        exit 1; \
    fi

# Run all checks (use before submitting a PR)
check: fmt clippy test deny doc

# Build the project
build:
    cargo build --workspace --all-features
