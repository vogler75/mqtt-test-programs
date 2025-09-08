#!/bin/bash

# MQTT Topic Replicator Runner Script
# Sets all possible environment variables and starts the replicator.py program

# Broker A (Source) Configuration
export BROKER_A_HOST="scada"
export BROKER_A_PORT="1883"
export BROKER_A_USERNAME=""
export BROKER_A_PASSWORD=""

# Broker B (Destination) Configuration  
export BROKER_B_HOST="test.monstermq.com"
export BROKER_B_PORT="1883"
export BROKER_B_USERNAME=""
export BROKER_B_PASSWORD=""

# Activate virtual environment
source .venv/bin/activate

# Start the replicator
echo "Starting MQTT Topic Replicator..."
python replicator.py