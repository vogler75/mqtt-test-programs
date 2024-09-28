import cv2
import time
import numpy as np
import paho.mqtt.client as mqtt

# MQTT broker configuration
broker_address = "192.168.1.20"
broker_port = 1883
topic = "video/Cameras/1"

# Create an MQTT client
client = mqtt.Client()

def on_connect(client, userdata, flags, rc):
    print("Connected with result code " + str(rc))
    client.subscribe(topic)

def on_message(client, userdata, msg):
    # Decode the image data
    np_arr = np.frombuffer(msg.payload, np.uint8)
    image = cv2.imdecode(np_arr, cv2.IMREAD_COLOR)

    # Display the image
    try:
        cv2.imshow("Received Image", image)
        cv2.waitKey(1)
    except cv2.error as e:
        print(f"OpenCV error: {e}")


client.on_connect = on_connect
client.on_message = on_message

client.connect(broker_address, broker_port, 60)
client.loop_start()

try:
    while True:
        time.sleep(1)
except KeyboardInterrupt:
    print("Keyboard interrupt")

client.loop_stop()
client.disconnect()
cv2.destroyAllWindows()