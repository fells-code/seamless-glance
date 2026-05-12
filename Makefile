APP_NAME := seamless-glance
DIST_DIR := dist

# Extract version from Cargo.toml
VERSION := $(shell grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

# Targets
MAC_ARM := aarch64-apple-darwin
MAC_X86 := x86_64-apple-darwin
LINUX_X86 := x86_64-unknown-linux-gnu

.PHONY: help clean version build build-macos build-linux dist checksums release-local release-helper fmt lint test

help:
	@echo "Seamless Glance – Make targets"
	@echo ""
	@echo "make build            Build all binaries"
	@echo "make build-macos      Build macOS arm + x86"
	@echo "make build-linux      Build Linux x86_64"
	@echo "make dist             Build and prepare dist/"
	@echo "make checksums        Generate SHA256 checksums"
	@echo "make release-local    Build + checksums (local release)"
	@echo "make release-helper   Build + sync support repos for current version"
	@echo "make clean            Remove build artifacts"
	@echo ""

version:
	@echo $(VERSION)

clean:
	rm -rf target $(DIST_DIR)

# ---------- Build Targets ----------

build: build-macos build-linux

build-macos:
	cargo build --release --target $(MAC_ARM)
	cargo build --release --target $(MAC_X86)

build-linux:
	docker run --rm \
	  --platform=linux/amd64 \
	  -v "$(PWD)":/app \
	  -w /app \
	  rust:1.91 \
	  bash -c "\
	    rustup target add x86_64-unknown-linux-gnu && \
	    cargo build --profile release-linux --target x86_64-unknown-linux-gnu \
	  "

# ---------- Dist Packaging ----------

dist: clean build
	mkdir -p $(DIST_DIR)

	cp target/$(MAC_ARM)/release/$(APP_NAME) \
		$(DIST_DIR)/$(APP_NAME)-$(VERSION)-$(MAC_ARM)

	cp target/$(MAC_X86)/release/$(APP_NAME) \
		$(DIST_DIR)/$(APP_NAME)-$(VERSION)-$(MAC_X86)

	cp target/$(LINUX_X86)/release-linux/$(APP_NAME) \
		$(DIST_DIR)/$(APP_NAME)-$(VERSION)-$(LINUX_X86)

	@echo "✅ Binaries copied to $(DIST_DIR)/"

# ---------- Checksums ----------

checksums:
	cd $(DIST_DIR) && shasum -a 256 * > SHA256SUMS.txt
	@echo "✅ SHA256SUMS.txt generated"

# ---------- Local Release Helper ----------

release-local: dist checksums
	@echo ""
	@echo "Local release ready:"
	@ls -lh $(DIST_DIR)
	@echo ""
	@echo "Next steps:"
	@echo "  - Upload binaries to distro repo release"
	@echo "  - Update Homebrew formula with checksums"
	@echo ""

release-helper:
	./scripts/release-helper.sh

test:
	cargo test --all

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

install:
	cp $(DIST_DIR)/$(APP_NAME)-$(VERSION)-$(MAC_ARM) /usr/local/bin/$(APP_NAME)

verify:
	seamless-glance --help || true
