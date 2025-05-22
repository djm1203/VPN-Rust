#!/bin/bash

echo "Cleaning up VPN interfaces..."

# Find and remove all rustvpn interfaces
for interface in $(ip link show | grep -o 'rustvpn[0-9]*'); do
    echo "Removing interface: $interface"
    sudo ip link set $interface down 2>/dev/null
    sudo ip link delete $interface 2>/dev/null
done

# Remove any lingering routes
sudo ip route del 10.8.0.0/24 2>/dev/null

# Kill any running VPN processes
pkill -f "target/debug/server" 2>/dev/null
pkill -f "target/debug/client" 2>/dev/null

echo "Cleanup complete!"
