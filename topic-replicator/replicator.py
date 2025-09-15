import paho.mqtt.client as mqtt
import time
import os
import threading
import uuid

# --- Configuration ---
# Broker A (Source)
BROKER_A_HOST = os.environ.get("BROKER_A_HOST", "scada")  # Change if different
BROKER_A_PORT = int(os.environ.get("BROKER_A_PORT", 1883))
BROKER_A_CLIENT_ID = f"broker_a_subscriber_bridge_{uuid.uuid4().hex[:8]}"
# Optional: Credentials for Broker A
BROKER_A_USERNAME = os.environ.get("BROKER_A_USERNAME", "")
BROKER_A_PASSWORD = os.environ.get("BROKER_A_PASSWORD", "")

# Broker B (Destination)
BROKER_B_HOST = os.environ.get("BROKER_B_HOST", "test.monstermq.com")  # Change if different
BROKER_B_PORT = int(os.environ.get("BROKER_B_PORT", 1883))  # Change if different, e.g., another local broker
BROKER_B_CLIENT_ID = f"broker_b_publisher_bridge_{uuid.uuid4().hex[:8]}"

# Optional: Credentials for Broker B
BROKER_B_USERNAME = os.environ.get("BROKER_B_USERNAME", None)
BROKER_B_PASSWORD = os.environ.get("BROKER_B_PASSWORD", None)

# List of wildcard topics to subscribe to on Broker A
# Example: ["sensors/+/temperature", "home/lights/#", "alerts/critical"]
# Can be overridden with TOPICS_TO_SUBSCRIBE environment variable (comma-separated)
TOPICS_TO_SUBSCRIBE = os.environ.get("TOPICS_TO_SUBSCRIBE", "#").split(",")
if len(TOPICS_TO_SUBSCRIBE) == 1 and TOPICS_TO_SUBSCRIBE[0] == "":
    TOPICS_TO_SUBSCRIBE = []  # Handle empty env var case

# --- Global variable to hold the client for Broker B ---
# This is needed so on_message_A can publish to client_B
client_b_publisher = None

# --- Metrics tracking ---
message_count = 0
start_time = time.time()
last_report_time = time.time()
last_message_count = 0
metrics_lock = threading.Lock()


# --- MQTT Callbacks for Broker A (Subscriber) ---

def on_connect_A(client, userdata, flags, rc, properties=None):
    """Callback for when client_A connects to Broker A."""
    if rc == 0:
        print(f"Successfully connected to Broker A: {BROKER_A_HOST}:{BROKER_A_PORT}")
        # Subscribe to the specified topics
        for topic in TOPICS_TO_SUBSCRIBE:
            print(f"Subscribing to topic on Broker A: {topic}")
            # Using QoS 1 for subscription as a sensible default
            client.subscribe(topic, qos=0)
    else:
        print(f"Failed to connect to Broker A, return code {rc}\n")
        # Consider adding reconnection logic or exiting if critical


def on_message_A(client, userdata, msg):
    """Callback for when a message is received from Broker A."""
    global client_b_publisher, message_count, metrics_lock
    #print(f"Broker A | Topic: {msg.topic} | QoS: {msg.qos} | Retained: {msg.retain} | Payload: {msg.payload.decode()[:60]}...")

    # Increment message counter
    with metrics_lock:
        message_count += 1

    if client_b_publisher and client_b_publisher.is_connected():
        try:
            # Publish the message to Broker B, preserving topic, payload, QoS, and retain flag
            result = client_b_publisher.publish(
                topic=msg.topic,
                payload=msg.payload,
                qos=0, # msg.qos,
                retain=msg.retain
            )
            result.wait_for_publish(timeout=5)  # Wait for publish confirmation (optional)
            #if result.is_published():
            #    print(f"  -> Broker B | Published | Topic: {msg.topic} | Retained: {msg.retain}")
            #else:
            #    print(f"  -> Broker B | FAILED to publish | Topic: {msg.topic} | Mid: {result.mid}")

        except Exception as e:
            print(f"  -> Broker B | Error publishing message: {e}")
    else:
        pass
        #print("  -> Broker B | Not connected or publisher not initialized. Message not forwarded.")


def on_disconnect_A(client, userdata, rc, properties=None):
    """Callback for when client_A disconnects from Broker A."""
    print(f"Disconnected from Broker A with result code {rc}. Attempting to reconnect...")
    # Paho MQTT's loop_forever() handles reconnection attempts by default.
    # You can add custom logic here if needed, e.g., exponential backoff.


def on_log_A(client, userdata, level, buf):
    """Optional: Log callback for Broker A client."""
    # print(f"Log A: {buf}")
    pass


# --- MQTT Callbacks for Broker B (Publisher) ---

def on_connect_B(client, userdata, flags, rc, properties=None):
    """Callback for when client_B connects to Broker B."""
    if rc == 0:
        print(f"Successfully connected to Broker B: {BROKER_B_HOST}:{BROKER_B_PORT}")
    else:
        print(f"Failed to connect to Broker B, return code {rc}\n")


def on_disconnect_B(client, userdata, rc, properties=None):
    """Callback for when client_B disconnects from Broker B."""
    print(f"Disconnected from Broker B with result code {rc}. Will attempt to reconnect.")
    # Paho MQTT's loop_forever() handles reconnection attempts by default.


def on_publish_B(client, userdata, mid):
    """Callback when a message is successfully published to Broker B."""
    # This callback confirms the message has left the client.
    # print(f"  -> Broker B | Message (mid: {mid}) successfully sent to broker.")
    pass


def on_log_B(client, userdata, level, buf):
    """Optional: Log callback for Broker B client."""
    # print(f"Log B: {buf}")
    pass


# --- Metrics reporting ---
def print_metrics():
    """Print current replication metrics."""
    global message_count, start_time, last_report_time, last_message_count, metrics_lock
    
    current_time = time.time()
    with metrics_lock:
        current_count = message_count
        
        # Calculate overall metrics
        total_elapsed = current_time - start_time
        overall_rate = current_count / total_elapsed if total_elapsed > 0 else 0
        
        # Calculate recent metrics (since last report)
        interval_elapsed = current_time - last_report_time
        interval_messages = current_count - last_message_count
        interval_rate = interval_messages / interval_elapsed if interval_elapsed > 0 else 0
        
        # Update for next interval
        last_report_time = current_time
        last_message_count = current_count
    
    print(f"METRICS: Total: {current_count} msgs, Overall: {overall_rate:.1f} msg/s, Recent: {interval_rate:.1f} msg/s")


def metrics_reporter():
    """Background thread function to periodically report metrics."""
    while True:
        time.sleep(10)  # Report every 10 seconds
        print_metrics()


# --- Main Script Logic ---
if __name__ == "__main__":
    print("Starting MQTT Bridge...")

    # --- Start metrics reporting thread ---
    metrics_thread = threading.Thread(target=metrics_reporter, daemon=True)
    metrics_thread.start()

    # --- Initialize Client for Broker B (Publisher) ---
    client_b_publisher = mqtt.Client(client_id=BROKER_B_CLIENT_ID, protocol=mqtt.MQTTv311)

    if BROKER_B_USERNAME and BROKER_B_PASSWORD:
        client_b_publisher.username_pw_set(BROKER_B_USERNAME, BROKER_B_PASSWORD)

    client_b_publisher.on_connect = on_connect_B
    client_b_publisher.on_disconnect = on_disconnect_B
    client_b_publisher.on_publish = on_publish_B  # Useful for QoS > 0
    # client_b_publisher.on_log = on_log_B # Uncomment for detailed logs

    try:
        print(f"Attempting to connect to Broker B: {BROKER_B_HOST}:{BROKER_B_PORT}")
        client_b_publisher.connect(BROKER_B_HOST, BROKER_B_PORT, keepalive=60)
        client_b_publisher.loop_start()  # Start a background thread for Broker B's network loop
    except Exception as e:
        print(f"Could not connect to Broker B: {e}")
        # Decide if the script should exit or retry later
        # For now, we'll let it proceed and try to connect Broker A,
        # but publishing will fail until Broker B is up.

    # --- Initialize Client for Broker A (Subscriber) ---
    # Pass client_b_publisher via userdata so on_message_A can access it
    # This is not strictly necessary with the global variable approach, but good practice.
    client_a_subscriber = mqtt.Client(client_id=BROKER_A_CLIENT_ID, protocol=mqtt.MQTTv311)
    # client_a_subscriber = mqtt.Client(client_id=BROKER_A_CLIENT_ID, userdata={'client_b': client_b_publisher}) # For MQTTv3.1.1

    if BROKER_A_USERNAME and BROKER_A_PASSWORD:
        client_a_subscriber.username_pw_set(BROKER_A_USERNAME, BROKER_A_PASSWORD)

    client_a_subscriber.on_connect = on_connect_A
    client_a_subscriber.on_message = on_message_A
    client_a_subscriber.on_disconnect = on_disconnect_A
    # client_a_subscriber.on_log = on_log_A # Uncomment for detailed logs

    try:
        print(f"Attempting to connect to Broker A: {BROKER_A_HOST}:{BROKER_A_PORT}")
        client_a_subscriber.connect(BROKER_A_HOST, BROKER_A_PORT, keepalive=60)
    except Exception as e:
        print(f"Could not connect to Broker A: {e}. Exiting.")
        if client_b_publisher.is_connected():
            client_b_publisher.loop_stop()
            client_b_publisher.disconnect()
        exit(1)

    # Start the network loop for Broker A (Subscriber) in the main thread.
    # This call is blocking and will keep the script running.
    # It also handles reconnections automatically.
    try:
        print("MQTT Bridge is running. Press Ctrl+C to exit.")
        client_a_subscriber.loop_forever()
    except KeyboardInterrupt:
        print("Shutting down MQTT Bridge...")
    except Exception as e:
        print(f"An unexpected error occurred: {e}")
    finally:
        print("Disconnecting clients...")
        if client_a_subscriber.is_connected():
            client_a_subscriber.disconnect()
            client_a_subscriber.loop_stop()  # Ensure loop is stopped if not using loop_forever

        if client_b_publisher.is_connected():
            client_b_publisher.loop_stop()  # Stop the background thread
            client_b_publisher.disconnect()
        print("MQTT Bridge stopped.")
