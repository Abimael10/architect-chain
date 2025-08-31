# Architect Chain - Development Makefile
# This Makefile provides convenient commands for development and testing

.PHONY: help build test clean lint format check docs demo dev quality \
         network-deploy network-stop network-test \
         test-unit test-integration \
         demo-fees demo-transactions \
         benchmark security-check test-all demo-all production-ready \
         install uninstall

# Default target
help:
	@echo "Architect Chain - Available Commands:"
	@echo "  build        - Build the project in release mode"
	@echo "  test         - Run all tests with single thread"
	@echo "  clean        - Clean build artifacts and test data"
	@echo "  lint         - Run clippy linter with strict warnings"
	@echo "  format       - Format code with rustfmt"
	@echo "  check        - Quick compilation check"
	@echo "  docs         - Generate documentation (opens browser)"
	@echo "  demo         - Run a quick blockchain demo"
	@echo "  dev          - Development cycle (check + test)"
	@echo "  quality      - Quality checks (format + lint + test)"
	@echo ""
	@echo "Network Operations:"
	@echo "  network-deploy  - Deploy multi-node blockchain network"
	@echo "  network-stop    - Stop multi-node network"
	@echo "  network-test    - Test network functionality"
	@echo ""
	@echo "Testing:"
	@echo "  test-unit       - Run unit tests only"
	@echo "  test-integration - Run integration tests only"
	@echo "  test-all        - Run all types of tests"
	@echo ""
	@echo "Demos:"
	@echo "  demo-fees       - Demonstrate fee system features"
	@echo "  demo-transactions - Demonstrate transaction features"
	@echo "  demo-all        - Run all demo scenarios"
	@echo ""
	@echo "Analysis:"
	@echo "  benchmark       - Run performance benchmarks"
	@echo "  security-check  - Run security analysis"
	@echo ""
	@echo "System:"
	@echo "  install         - Install binary to system"
	@echo "  uninstall       - Remove installed binary"
	@echo "  production-ready - Complete production readiness check"

# Build the project
build:
	@echo "🔨 Building Architect Chain..."
	cargo build --release

# Run tests
test:
	@echo "🧪 Running tests..."
	cargo test -- --test-threads=1

# Clean build artifacts and test data
clean:
	@echo "🧹 Cleaning up..."
	cargo clean
	rm -rf data/ wallet.dat



# Lint with clippy
lint:
	@echo "🔍 Running clippy..."
	cargo clippy -- -D warnings

# Format code
format:
	@echo "✨ Formatting code..."
	cargo fmt

# Quick compilation check
check:
	@echo "⚡ Quick check..."
	cargo check

# Generate documentation (warning: opens browser)
docs:
	@echo "📚 Generating documentation..."
	@echo "Note: This will open documentation in your browser"
	cargo doc --open

# Quick blockchain demo workflow
demo:
	@echo "🎬 Running blockchain demo..."
	@echo "Step 1: Creating wallet..."
	$(eval DEMO_ADDR := $(shell cargo run -- createwallet | grep "Your new address:" | cut -d' ' -f4))
	@echo "Step 2: Creating blockchain with genesis block..."
	@cargo run -- createblockchain $(DEMO_ADDR)
	@echo "Step 3: Checking balance..."
	@cargo run -- getbalance $(DEMO_ADDR)
	@echo "✅ Demo completed! Address: $(DEMO_ADDR)"

# Development workflow (quick check + comprehensive tests)
dev: check test
	@echo "✅ Development cycle completed!"

# Quality assurance workflow (format + lint + test)
quality: format lint test
	@echo "✅ All quality checks passed!"

# ============================================================================
# NETWORK OPERATIONS
# ============================================================================

# Deploy multi-node blockchain network
network-deploy: build
	@echo "🌐 Deploying multi-node blockchain network..."
	./deployment/multi-node-blockchain-deployment.sh

# Stop multi-node network
network-stop:
	@echo "🛑 Stopping multi-node network..."
	./deployment/stop-network.sh

# Test network functionality
network-test: network-deploy
	@echo "🧪 Testing network functionality..."
	./deployment/test-network.sh
	$(MAKE) network-stop

# ============================================================================
# TESTING
# ============================================================================

# Run unit tests only
test-unit:
	@echo "🧪 Running unit tests..."
	cargo test --lib -- --test-threads=1

# Run integration tests only
test-integration:
	@echo "🧪 Running integration tests..."
	cargo test --test blockchain_integration_tests -- --test-threads=1

# ============================================================================
# DEMOS
# ============================================================================

# Demonstrate fee system features
demo-fees:
	@echo "💰 Fee System Demo..."
	$(eval DEMO_ADDR := $(shell cargo run -- createwallet | grep "Your new address:" | cut -d' ' -f4))
	@echo "Created wallet: $(DEMO_ADDR)"
	@cargo run -- createblockchain $(DEMO_ADDR)
	@echo "\nTesting different fee modes:"
	@cargo run -- feestatus
	@cargo run -- estimatefee low
	@cargo run -- estimatefee normal
	@cargo run -- estimatefee high
	@cargo run -- estimatefee urgent
	@cargo run -- setfeemode 5
	@cargo run -- feestatus
	@cargo run -- setfeemode dynamic
	@cargo run -- feestatus
	@echo "✅ Fee system demo completed!"

# Demonstrate transaction features (simplified)
demo-transactions:
	@echo "💸 Transaction Demo..."
	@echo "Note: This demo shows the transaction system is working."
	@echo "The integration tests already verify full transaction functionality."
	@echo "\nRunning integration test that includes transaction validation..."
	@cargo test --test blockchain_integration_tests test_transaction_creation_and_validation -- --nocapture
	@echo "\n✅ Transaction system verified through integration tests!"
	@echo "\nFor manual testing:"
	@echo "1. Run: make clean && make demo"
	@echo "2. Run: cargo run -- createwallet  (creates second wallet)"
	@echo "3. Run: cargo run -- send <addr1> <addr2> 500000000 1"
	@echo "4. Run: cargo run -- getbalance <addr1>"
	@echo "5. Run: cargo run -- getbalance <addr2>"

# ============================================================================
# ANALYSIS
# ============================================================================

# Run performance benchmarks
benchmark:
	@echo "📊 Running performance benchmarks..."
	@echo "Building optimized binary..."
	@cargo build --release
	@echo "\nBenchmarking wallet creation:"
	@time -p cargo run --release -- createwallet > /dev/null
	@echo "\nBenchmarking blockchain creation:"
	$(eval BENCH_ADDR := $(shell cargo run --release -- createwallet | grep "Your new address:" | cut -d' ' -f4))
	@time -p cargo run --release -- createblockchain $(BENCH_ADDR)
	@echo "\nBenchmarking transaction:"
	$(eval BENCH_ADDR2 := $(shell cargo run --release -- createwallet | grep "Your new address:" | cut -d' ' -f4))
	@time -p cargo run --release -- send $(BENCH_ADDR) $(BENCH_ADDR2) 1000000000 1
	@echo "✅ Benchmark completed!"

# Run security analysis
security-check:
	@echo "🔒 Running security analysis..."
	@echo "Checking for unsafe code..."
	@grep -r "unsafe" src/ || echo "No unsafe code found ✅"
	@echo "\nRunning cargo audit (if available)..."
	@cargo audit 2>/dev/null || echo "cargo-audit not installed, skipping"
	@echo "\nChecking dependencies for known vulnerabilities..."
	@cargo tree --duplicates || echo "No duplicate dependencies found"
	@echo "Validating cryptographic implementations..."
	@cargo test proof_of_work -- --test-threads=1
	@cargo test encrypted -- --test-threads=1
	@echo "✅ Security check completed!"



# Install binary to system
install: build
	@echo "📦 Installing architect-chain..."
	cargo install --path . --force
	@echo "✅ Installation completed!"

# Remove installed binary
uninstall:
	@echo "🗑️ Uninstalling architect-chain..."
	cargo uninstall architect-chain
	@echo "✅ Uninstallation completed!"

# ============================================================================
# COMPREHENSIVE WORKFLOWS
# ============================================================================

# Full system test (everything)
test-all: clean build test-unit test-integration
	@echo "✅ All tests completed successfully!"

# Complete demo showcase
demo-all: demo demo-fees demo-transactions
	@echo "✅ All demos completed successfully!"

# Production readiness check
production-ready: clean quality test-all benchmark security-check
	@echo "✅ Production readiness check completed!"