import paho.mqtt.client as mqtt
import time
import threading

# --- Configuration ---
BROKER_ADDRESS = "etm-pc"  # Public MQTT broker
BROKER_PORT = 10004
CLIENT_ID = "mqtt_time_measurement_client"  # Choose a unique client ID

# Define the three wildcard topics to subscribe to, along with a description
TOPICS_TO_SUBSCRIBE = [
    ("0/0/0/+/0/0/0/0", "Test 1"),
    ("0/0/0/0/0/0/+/0", "Test 2"),
]

# Timeout for waiting for all messages (in seconds)
OPERATION_TIMEOUT = 120

# --- Shared State ---
# This dictionary will store the state for each subscribed wildcard topic.
# Format: {wildcard_topic: {"start_time": float, "received": bool, "description": str}}
topic_states = {}
# Event to signal that all topics have received their first message or an error occurred.
all_topics_responded_event = threading.Event()
# Lock for thread-safe access to topic_states
lock = threading.Lock()


# --- MQTT Callbacks ---

def on_connect(client, userdata, flags, reason_code, properties):
    """Callback for when the client receives a CONNACK response from the server."""
    if reason_code == 0:
        print(f"Successfully connected to MQTT Broker: {BROKER_ADDRESS}")
        print("Subscribing to topics...")
        with lock:
            # Clear previous states if any (e.g., on reconnect)
            topic_states.clear()
            successfully_initiated_subscriptions = 0
            for topic_wildcard, description in TOPICS_TO_SUBSCRIBE:
                # Subscribe to the topic
                result, mid = client.subscribe(topic_wildcard)
                if result == mqtt.MQTT_ERR_SUCCESS:
                    print(f"  -> Initiated subscription to: {topic_wildcard}")
                    # Record the start time for this subscription
                    topic_states[topic_wildcard] = {
                        "start_time": time.monotonic(),
                        "received": False,
                        "description": description
                    }
                    successfully_initiated_subscriptions += 1
                else:
                    print(f"  -> Failed to initiate subscription to: {topic_wildcard} (Error code: {result})")

            if successfully_initiated_subscriptions == 0 and TOPICS_TO_SUBSCRIBE:
                print("Error: No subscriptions could be initiated. Exiting.")
                all_topics_responded_event.set()  # Signal to stop if no subscriptions work
            elif successfully_initiated_subscriptions < len(TOPICS_TO_SUBSCRIBE):
                print("Warning: Not all subscriptions were successfully initiated.")


    else:
        print(f"Failed to connect to MQTT Broker, reason code: {reason_code}")
        # Signal the main loop to stop if connection fails
        all_topics_responded_event.set()


def on_message(client, userdata, msg):
    """Callback for when a PUBLISH message is received from the server."""
    received_topic = msg.topic
    payload_str = msg.payload.decode(errors='ignore')  # Decode payload, ignore errors for non-text
    current_time = time.monotonic()

    # print(f"DEBUG: Message received on topic '{received_topic}' with payload '{payload_str[:30]}...'")

    all_target_topics_responded = True
    message_matched_a_tracked_topic = False

    with lock:
        for wildcard, state in topic_states.items():
            # Check if this message matches a wildcard we are tracking and haven't received yet
            if not state["received"] and mqtt.topic_matches_sub(wildcard, received_topic):
                duration = current_time - state["start_time"]
                print(f"\n--- Measurement Recorded ---")
                print(f"  Wildcard Subscription: {wildcard} ({state['description']})")
                print(f"  Actual Topic Matched:  {received_topic}")
                print(f"  Time to First Message: {duration:.4f} seconds")
                print(f"  Message Payload (first 60 chars): {payload_str[:60]}{'...' if len(payload_str) > 60 else ''}")

                state["received"] = True
                message_matched_a_tracked_topic = True
                # Note: A single message might match multiple wildcards.
                # This loop ensures each relevant wildcard subscription gets its time recorded once.

        # After processing, check if all *successfully initiated* subscriptions have received a message
        if message_matched_a_tracked_topic:  # Only check if we updated a state
            if not topic_states:  # No successful subscriptions
                all_target_topics_responded = False
            else:
                for state in topic_states.values():
                    if not state["received"]:
                        all_target_topics_responded = False
                        break

            if all_target_topics_responded and topic_states:  # Ensure topic_states is not empty
                print("\nAll targeted wildcard subscriptions have received their first message.")
                all_topics_responded_event.set()


def on_subscribe(client, userdata, mid, granted_qos, properties=None):
    """Callback for when the broker responds to a subscription request."""
    # This callback confirms that the subscription is acknowledged by the broker.
    # For this problem, we start timing from the client.subscribe() call.
    # You could use this callback if you wanted to time from broker acknowledgement.
    # print(f"DEBUG: Subscription acknowledged by broker. MID: {mid}, Granted QoS: {granted_qos}")
    pass


# --- Main Program ---
def main():
    # Using paho-mqtt v2.x callback API style.
    # If using paho-mqtt v1.x, use `mqtt.Client(CLIENT_ID)` and adjust callback signatures.
    client = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2, client_id=CLIENT_ID)

    # Assigning callbacks
    client.on_connect = on_connect
    client.on_message = on_message
    client.on_subscribe = on_subscribe  # Good practice to define, even if minimal

    print(f"Attempting to connect to MQTT broker: {BROKER_ADDRESS}:{BROKER_PORT}...")
    try:
        # Connect to the broker. keepalive is in seconds.
        client.connect(BROKER_ADDRESS, BROKER_PORT, keepalive=60)
    except Exception as e:
        print(f"Error during initial connection: {e}")
        return  # Exit if connection fails at the start

    # Start the network loop in a background thread.
    # This handles Paho-MQTT's network communication, callbacks, and reconnections.
    client.loop_start()

    print(f"Waiting for messages on subscribed topics or timeout ({OPERATION_TIMEOUT}s)...")
    print("To test, publish messages to topics matching your wildcards, e.g.:")
    print("  - sensor/room1/temperature")
    print("  - device/light01/status")
    print("  - log/system/app/info")

    # Wait for the event to be set (all messages received or error) or timeout.
    event_triggered = all_topics_responded_event.wait(timeout=OPERATION_TIMEOUT)

    if not event_triggered:
        print(f"\nTimeout of {OPERATION_TIMEOUT} seconds reached.")
        with lock:
            all_received = True
            if not topic_states:
                print("  No subscriptions were active or no messages received.")
            else:
                for wildcard, state in topic_states.items():
                    if not state["received"]:
                        all_received = False
                        print(f"  - No message received for wildcard: {wildcard} ({state['description']})")
                if all_received:  # Should not happen if timeout occurred unless topic_states was empty
                    print("  All subscribed topics seem to have received messages, but timeout occurred (unexpected).")

    print("Shutting down MQTT client...")
    client.loop_stop()  # Stop the background network loop

    # Disconnecting the client. This is a blocking call.
    # Paho's disconnect is safe to call even if not fully connected or already disconnected.
    client.disconnect()

    print("Program finished.")


if __name__ == "__main__":
    main()