#!/bin/bash

# Multi-Node Blockchain Deployment Script
# This script deploys a proper multi-node blockchain network with isolated databases

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
BINARY_PATH="./target/release/architect-chain"
BASE_PORT=2001
NUM_NODES=3

# Helper functions
print_header() {
    echo -e "${BLUE}🔗 Architect Chain - Multi-Node Blockchain Deployment${NC}"
    echo "====================================================="
}

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if binary exists
check_binary() {
    if [ ! -f "$BINARY_PATH" ]; then
        print_error "Binary not found at $BINARY_PATH"
        print_info "Run: cargo build --release"
        exit 1
    fi
}

# Clean up previous deployment
cleanup() {
    print_step "Cleaning up previous deployment..."
    
    # Stop any running nodes
    pkill -f "architect-chain startnode" 2>/dev/null || true
    
    # Clean up data directories
    rm -rf deployment/nodes
    rm -rf deployment/logs
    rm -rf data/node_*
    
    # Create fresh directories
    mkdir -p deployment/nodes
    mkdir -p deployment/logs
    mkdir -p data
}

# Create wallets for miners
create_wallets() {
    print_step "Creating wallets for miners..."
    
    # Create miner wallets
    MINER1_ADDR=$($BINARY_PATH createwallet | grep "Your new address:" | cut -d' ' -f4)
    MINER2_ADDR=$($BINARY_PATH createwallet | grep "Your new address:" | cut -d' ' -f4)
    
    print_info "Miner 1: $MINER1_ADDR"
    print_info "Miner 2: $MINER2_ADDR"
    
    # Export for use in other functions
    export MINER1_ADDR
    export MINER2_ADDR
}

# Create genesis blockchain for the first node
create_genesis() {
    print_step "Creating genesis blockchain..."
    
    # Create genesis blockchain with first miner's address
    NODE_ADDRESS="127.0.0.1:$BASE_PORT" NODE_ID="$BASE_PORT" $BINARY_PATH createblockchain "$MINER1_ADDR"
    
    # Copy genesis blockchain to node-specific directory
    mkdir -p "data/node_$BASE_PORT"
    cp -r data/* "data/node_$BASE_PORT/" 2>/dev/null || true
    
    print_info "Genesis blockchain created for node $BASE_PORT"
}

# Copy blockchain data to other nodes
sync_initial_blockchain() {
    print_step "Synchronizing initial blockchain to other nodes..."
    
    for ((i=1; i<NUM_NODES; i++)); do
        local node_port=$((BASE_PORT + i))
        local node_dir="data/node_$node_port"
        
        mkdir -p "$node_dir"
        cp -r "data/node_$BASE_PORT"/* "$node_dir/"
        
        print_info "Blockchain synced to node $node_port"
    done
}

# Start a blockchain node
start_node() {
    local node_id=$1
    local port=$2
    local mining_addr=$3
    local node_type=$4
    
    print_step "Starting node$node_id on port $port..."
    
    # Set environment variables for this node
    export NODE_ADDRESS="127.0.0.1:$port"
    export NODE_ID="$port"
    
    # Start node
    if [ -n "$mining_addr" ]; then
        nohup env NODE_ADDRESS="127.0.0.1:$port" NODE_ID="$port" $BINARY_PATH startnode "$mining_addr" > "deployment/logs/node$node_id.log" 2>&1 &
        print_info "node$node_id started as $node_type with address: $mining_addr"
    else
        nohup env NODE_ADDRESS="127.0.0.1:$port" NODE_ID="$port" $BINARY_PATH startnode > "deployment/logs/node$node_id.log" 2>&1 &
        print_info "node$node_id started as $node_type"
    fi
    
    local pid=$!
    echo $pid > "deployment/nodes/node$node_id.pid"
    print_info "PID: $pid"
    
    # Wait a moment for node to start
    sleep 2
}

# Start all nodes
start_network() {
    print_step "Starting blockchain network..."
    
    # Start bootstrap node (node1) - non-mining
    start_node 1 $BASE_PORT "" "BOOTSTRAP"
    
    # Start mining nodes
    start_node 2 $((BASE_PORT + 1)) "$MINER1_ADDR" "MINER"
    start_node 3 $((BASE_PORT + 2)) "$MINER2_ADDR" "MINER"
}

# Create management scripts
create_management_scripts() {
    print_step "Creating management scripts..."
    
    # Create monitoring script
    cat > deployment/monitor-network.sh << 'EOF'
#!/bin/bash

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "🔗 Multi-Node Blockchain Network Status"
echo "======================================="

for i in {1..3}; do
    if [ -f "deployment/nodes/node$i.pid" ]; then
        pid=$(cat "deployment/nodes/node$i.pid")
        if kill -0 $pid 2>/dev/null; then
            echo -e "✅ node$i (PID: $pid) - ${GREEN}RUNNING${NC}"
        else
            echo -e "❌ node$i - ${RED}STOPPED${NC}"
        fi
    else
        echo -e "❌ node$i - ${RED}NOT STARTED${NC}"
    fi
done

echo ""
echo "Recent Activity:"
for i in {1..3}; do
    if [ -f "deployment/logs/node$i.log" ]; then
        echo "node$i: $(tail -1 deployment/logs/node$i.log | cut -c1-80)"
    fi
done
EOF
    chmod +x deployment/monitor-network.sh
    
    # Create stop script
    cat > deployment/stop-network.sh << 'EOF'
#!/bin/bash

echo "🔗 Stopping Multi-Node Blockchain Network"
echo "=========================================="

for i in {1..3}; do
    if [ -f "deployment/nodes/node$i.pid" ]; then
        pid=$(cat "deployment/nodes/node$i.pid")
        if kill -0 $pid 2>/dev/null; then
            echo "Stopping node$i (PID: $pid)..."
            kill $pid
            rm "deployment/nodes/node$i.pid"
        fi
    fi
done

echo "Multi-node blockchain network stopped."
EOF
    chmod +x deployment/stop-network.sh
    
    # Create test script
    cat > deployment/test-network.sh << 'EOF'
#!/bin/bash

echo "🔗 Testing Multi-Node Blockchain Network"
echo "========================================"

# Test basic operations
echo "Testing wallet operations..."
./target/release/architect-chain listaddresses

echo ""
echo "Testing blockchain status..."
./target/release/architect-chain printchain | head -20

echo ""
echo "Testing fee system..."
./target/release/architect-chain feestatus

echo ""
echo "Network test completed."
EOF
    chmod +x deployment/test-network.sh
}

# Main deployment function
main() {
    print_header
    
    check_binary
    cleanup
    create_wallets
    create_genesis
    sync_initial_blockchain
    start_network
    create_management_scripts
    
    print_step "Deployment completed!"
    echo ""
    print_info "Multi-node blockchain network running:"
    print_info "  - Node 1 (Bootstrap): 127.0.0.1:$BASE_PORT"
    print_info "  - Node 2 (Miner):     127.0.0.1:$((BASE_PORT + 1)) ($MINER1_ADDR)"
    print_info "  - Node 3 (Miner):     127.0.0.1:$((BASE_PORT + 2)) ($MINER2_ADDR)"
    echo ""
    print_info "Each node has its own isolated database:"
    print_info "  - Node 1: data/node_$BASE_PORT/"
    print_info "  - Node 2: data/node_$((BASE_PORT + 1))/"
    print_info "  - Node 3: data/node_$((BASE_PORT + 2))/"
    echo ""
    print_info "Management commands:"
    print_info "  - Monitor: ./deployment/monitor-network.sh"
    print_info "  - Stop:    ./deployment/stop-network.sh"
    print_info "  - Test:    ./deployment/test-network.sh"
    echo ""
    print_info "Logs: deployment/logs/"
}

# Run main function
main "$@"