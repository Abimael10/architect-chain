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
