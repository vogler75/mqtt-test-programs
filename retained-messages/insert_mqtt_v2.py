import paho.mqtt.client as mqtt
import time

# Config
MQTT_BROKER = "linux0"
MQTT_PORT = 1883
MAX_DEPTH = 10
VALUES_PER_LEVEL = 10
LOG_INTERVAL = 10000

client = mqtt.Client()
client.connect(MQTT_BROKER, MQTT_PORT, 60)
client.loop_start()

published_count = 0

def publish_path(path):
    global published_count
    topic = "/".join(["root"] + path)
    payload = f"Payload for {topic}"
    client.publish(topic, payload, qos=0, retain=True)
    published_count += 1

    if published_count % LOG_INTERVAL == 0:
        print(f"Published: {published_count} messages")


def generate_and_publish(path=[], level=0):
    if level == MAX_DEPTH:
        return
    for i in range(VALUES_PER_LEVEL):
        new_path = path + [str(i)]
        publish_path(new_path)
        generate_and_publish(new_path, level + 1)

def main():
    start_time = time.time()
    generate_and_publish()
    duration = time.time() - start_time
    print(f"Done. Total published: {published_count} in {duration:.2f} seconds")
    client.loop_stop()
    client.disconnect()

if __name__ == "__main__":
    main()
