#!/bin/bash
# Test script for web command adaptations

echo "Testing new web command adaptation system..."

# Test the clear command through the IO API
echo ""
echo "Testing clear command adaptation:"
curl -s "http://localhost:8000/api/io?input=:::::%20clear%20:::::" | jq '.executed_actions[0].result'

echo ""
echo "Testing cls command adaptation:"
curl -s "http://localhost:8000/api/io?input=:::::%20cls%20:::::" | jq '.executed_actions[0].result'

echo ""
echo "Testing regular command (should work normally):"
curl -s "http://localhost:8000/api/io?input=:::::%20pwd%20:::::" | jq '.executed_actions[0].result'

echo ""
echo "Test completed!"
