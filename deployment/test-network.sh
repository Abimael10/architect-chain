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
