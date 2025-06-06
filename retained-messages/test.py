import paho.mqtt.client as mqtt
import threading
import time

# Config
BROKERS = [
    #{"host": "192.168.1.179", "port": 1883},    
    {"host": "localhost", "port": 10001, "status": False},
    #{"host": "localhost", "port": 10002, "status": False},
    {"host": "localhost", "port": 10003, "status": False},
]
MQTT_QOS = 0
MAX_DEPTH = 10
VALUES_PER_LEVEL = 10
LOG_INTERVAL = 1000
MESSAGES_PER_SECOND = 10_000  # Target messages per second
MAX_MESSAGES = 100_000_000_000  # Maximum messages to publish per broker, 0 for no limit
MQTT_USERNAME = ""  # Replace with your MQTT username
MQTT_PASSWORD = ""  # Replace with your MQTT password

def generate_topic_paths():
    """Generator that yields all increasing-depth topic paths up to MAX_DEPTH with VALUES_PER_LEVEL."""
    def recurse(path=[], level=0):
        if level == MAX_DEPTH:
            return
        for i in range(VALUES_PER_LEVEL):
            new_path = path + [str(i)]
            yield new_path
            yield from recurse(new_path, level + 1)
    yield from recurse()

def publish_to_broker(broker_config, broker_index):
    try:
        # Using a list to pass mutable reference for rc and a flag to on_connect
        broker_config["status"] = False

        def on_connect_callback(client_obj, userdata, flags, rc, properties=None):
            """Callback for when the client receives a CONNACK response from the server."""
            broker_config["status"] = True
            if rc == 0:
                print(f"[Broker {broker_index}] Connected successfully to {broker_config['host']}:{broker_config['port']} (on_connect).")
            else:
                print(f"[Broker {broker_index}] Connection to {broker_config['host']}:{broker_config['port']} failed with code {rc} (on_connect).")

        client = mqtt.Client()
        client.on_connect = on_connect_callback # Set the callback before connecting

        if MQTT_USERNAME and MQTT_PASSWORD:
            client.username_pw_set(MQTT_USERNAME, MQTT_PASSWORD)

        print(f"[Broker {broker_index}] Attempting to connect to {broker_config['host']}:{broker_config['port']}...")
        client.connect(broker_config["host"], broker_config["port"], 60)
        client.loop_start() # Start the network loop for handling outgoing/incoming messages.

        while not broker_config["status"]:
            print("Sleep...")
            time.sleep(1)         
 
        if MESSAGES_PER_SECOND > 0:
            desired_interval = 1.0 / MESSAGES_PER_SECOND * 100            
        else:
            desired_interval = 0  # No rate limiting, publish as fast as possible

        iteration_start_time = time.monotonic()
        count = 0
        start_time = time.time()
        last_log_time = start_time  # Initialize time of the last log message
        for path_parts in generate_topic_paths():

            #topic = "/".join(["root"] + path_parts)
            topic = "/".join(path_parts)
            payload = f"Payload for {topic}"
            info = client.publish(topic, payload, qos=MQTT_QOS, retain=True)
            info.wait_for_publish()

            count += 1

            if MAX_MESSAGES > 0 and count >= MAX_MESSAGES:
                print(f"[Broker {broker_index}] Reached MAX_MESSAGES limit of {MAX_MESSAGES}.")
                break # Exit the loop if max messages are published

            # Log progress approximately every second
            current_time = time.time()
            if current_time - last_log_time >= 1.0:
                elapsed_total_time = current_time - start_time # Total time since publishing started
                if elapsed_total_time > 0:
                    current_rate = count / elapsed_total_time
                    print(f"[Broker {broker_index}] Published: {count} messages (Rate: {current_rate:.2f} msg/s)")
                else:
                    print(f"[Broker {broker_index}] Published: {count} messages")
                last_log_time = current_time # Update the time of the last log

            if count % 100 == 0 and desired_interval > 0:
                time_taken_this_iteration = time.monotonic() - iteration_start_time
                sleep_duration = desired_interval - time_taken_this_iteration
                if sleep_duration > 0:
                    #print(sleep_duration, desired_interval, time_taken_this_iteration)
                    time.sleep(sleep_duration)
                iteration_start_time = time.monotonic()


    except Exception as e:
        print(f"[Broker {broker_index}] Error: {e} at counter {count}")

    duration = time.time() - start_time
    print(f"[Broker {broker_index}] Done. Total published: {count} in {duration:.2f} seconds")
    client.loop_stop()
    client.disconnect()

def main():
    threads = []

    for i, broker in enumerate(BROKERS):
        t = threading.Thread(target=publish_to_broker, args=(broker, i + 1))
        threads.append(t)
        t.start()

    for t in threads:
        t.join()

if __name__ == "__main__":
    main()
