import paho.mqtt.client as mqtt
import base64
import time

# MQTT Broker details
broker_address = "test.monstermq.com"
port = 1883
topic = "test/picture"

# Picture file path
picture_file = "picture.jpg"  # Replace with your picture file path

# Read the picture file and encode it as base64
try:
    with open(picture_file, "rb") as f:
        picture_data = f.read()
        encoded_picture = base64.b64encode(picture_data).decode('utf-8')
except FileNotFoundError:
    print(f"Error: Picture file '{picture_file}' not found.")
    exit()
except Exception as e:
    print(f"Error reading or encoding picture file: {e}")
    exit()

# Create an MQTT client instance
client = mqtt.Client()

# Connect to the broker
try:
    client.connect(broker_address, port)
    print(f"Connected to MQTT broker at {broker_address}:{port}")
except Exception as e:
    print(f"Error connecting to MQTT broker: {e}")
    exit()

time.sleep(1)

# Publish the base64 encoded picture
try:
    client.publish(topic, encoded_picture, retain=True)
    print(f"Published picture to topic: {topic}")
except Exception as e:
    print(f"Error publishing message: {e}")

# Disconnect from the broker
client.disconnect()
print("Disconnected from MQTT broker")
