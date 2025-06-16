import paho.mqtt.client as mqtt
import time

# MQTT broker configuration
MQTT_BROKER = "linux0"
MQTT_PORT = 1883
MQTT_TOPIC_ROOT = "root"
LEVELS = 10
ENTRIES_PER_LEVEL = 10

client = mqtt.Client()
client.connect(MQTT_BROKER, MQTT_PORT, 60)
client.loop_start()


def generate_topics(prefix=["root"], level=0):
    """Recursively generate MQTT topic paths as lists."""
    if level == LEVELS:
        yield prefix
    else:
        for i in range(ENTRIES_PER_LEVEL):
            yield from generate_topics(prefix + [str(i)], level + 1)


def main():
    total_published = 0
    start_time = time.time()

    for topic_parts in generate_topics():
        topic_str = "/".join(topic_parts)
        payload = f"Payload for {topic_str}"
        print(f"Publishing: {topic_str} -> {payload}")
        client.publish(topic_str, payload, qos=0, retain=True)
        total_published += 1

        if total_published % 10000 == 0:
            print(f"Published: {total_published}")

    duration = time.time() - start_time
    print(f"Done. Total published: {total_published} in {duration:.2f}s")

    client.loop_stop()
    client.disconnect()


if __name__ == "__main__":
    main()
