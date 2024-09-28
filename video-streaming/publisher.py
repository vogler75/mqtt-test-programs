import cv2
import numpy as np
import paho.mqtt.client as mqtt

import paho.mqtt.client as paho
from paho import mqtt

import time
import sys
import ssl

#ssl._create_default_https_context = ssl._create_unverified_context


# USB camera configuration
camera_device = 0  # 0 for the default camera, adjust if needed
if len(sys.argv) > 1:
    camera_device = int(sys.argv[1])

# MQTT broker configuration

#broker_address = "192.168.1.4"  # Replace with your MQTT broker's address
#broker_port = 1883

#broker_address = "bd9c43f59b7a42deba3248fca439f378.s1.eu.hivemq.cloud"
#broker_port = 8883

broker_address = "localhost"
broker_port = 1883

topic = "video/Cameras/1"  # Topic to publish the images
if len(sys.argv) > 2:
    topic = "video/Cameras/"+sys.argv[2]

# Create an MQTT client
client = paho.Client(protocol=paho.MQTTv31)

# Set TLS configuration
#client.tls_set(tls_version=mqtt.client.ssl.PROTOCOL_TLS)

# Set the username and password
#username = "unity_game"
#password = "Klaiu3878kdasdf"
#client.username_pw_set(username, password)


mqtt_connected = False


def on_connect(client, userdata, flags, rc, properties=None):
    global mqtt_connected
    print("CONNACK received with code %s." % rc)
    if rc==0:
        mqtt_connected = True


client.on_connect = on_connect
client.connect(broker_address, broker_port, 60)
client.loop_start()

# Open the USB camera
cap = cv2.VideoCapture(camera_device)

if not cap.isOpened():
    print("Error: Unable to access the camera.")
    exit()

prev_image = None
prev_image_time = time.time()

try:
    while True:
        current_time = time.time()
        elapsed_time = current_time - prev_image_time

        # Capture a frame from the camera
        ret, frame = cap.read()
        if not ret:
            print("Error: Failed to capture a frame.")
            break

        # Define the desired width
        desired_width = 480  # Change this to your desired width

        # Calculate the scaling factor to maintain aspect ratio
        height, width, _ = frame.shape
        scaling_factor = desired_width / width

        # Resize the image while maintaining aspect ratio
        new_width = int(width * scaling_factor)
        new_height = int(height * scaling_factor)
        resized_image = cv2.resize(frame, (new_width, new_height))

        # Compare with last picture
        image_gray = cv2.cvtColor(resized_image, cv2.COLOR_BGR2GRAY)
        if prev_image is not None and mqtt_connected:
            mse = np.mean((prev_image - image_gray) ** 2)
            #print(f"MSE: {mse}")
            if mse > 10 or elapsed_time > 1:
                prev_image_time = current_time

                # Encode picture
                _, encoded_frame = cv2.imencode(".jpg", resized_image)
                image_data = encoded_frame.tobytes()

                # Publish the image to the MQTT broker
                print(f"MSE: {mse} {width}x{height} Publish " + str(len(image_data)))
                client.publish(topic, image_data, qos=0, retain=False).wait_for_publish()

                # Wait for a moment (adjust the delay as needed)

        prev_image = image_gray
        time.sleep(0.1)

except KeyboardInterrupt:
    print("Keyboard interrupt")

# Release the camera and disconnect from MQTT
cap.release()
client.disconnect()
