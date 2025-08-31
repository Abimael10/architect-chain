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
