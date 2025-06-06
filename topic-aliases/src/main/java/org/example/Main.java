package org.example;

import org.eclipse.paho.mqttv5.client.*;
import org.eclipse.paho.mqttv5.common.MqttException;
import org.eclipse.paho.mqttv5.common.MqttMessage;
import org.eclipse.paho.mqttv5.common.packet.MqttProperties;

import java.util.UUID;

public class Main {
    public static void main(String[] args) {
        //String broker = "tcp://broker.emqx.io:1883";
        //String broker = "tcp://test.mosquitto.org:1883";
        String broker = "tcp://broker.hivemq.com:1883";

        String clientId = UUID.randomUUID().toString();
        String topic = "rocworks/#";

        try {
            // Create an MQTT client
            MqttClient client = new MqttClient(broker, clientId);

            // Configure MQTT connection options
            MqttConnectionOptions options = new MqttConnectionOptions();
            options.setAutomaticReconnect(false);
            options.setCleanStart(true);
            options.setTopicAliasMaximum(10);

            // Set a callback to handle incoming messages
            client.setCallback(new MqttCallback() {
                @Override
                public void disconnected(MqttDisconnectResponse mqttDisconnectResponse) {
                    System.out.println("Disconnected: " + mqttDisconnectResponse.getReasonString());
                }

                @Override
                public void mqttErrorOccurred(MqttException e) {
                    System.out.println("MQTT error: " + e.getMessage());
                }

                @Override
                public void messageArrived(String topic, MqttMessage message) throws Exception {
                    System.out.println("Message received:");
                    System.out.println("  Topic: " + topic);
                    System.out.println("  Payload: " + new String(message.getPayload()));

                    // Access MQTT 5.0 properties
                    MqttProperties properties = message.getProperties();
                    if (properties != null) {
                        Integer topicAlias = properties.getTopicAlias();
                        if (topicAlias != null) {
                            System.out.println("  Topic Alias: " + topicAlias);
                        } else {
                            System.out.println("  No Topic Alias in properties.");
                        }
                    } else {
                        System.out.println("  No MQTT 5.0 properties available.");
                    }
                }

                @Override
                public void deliveryComplete(IMqttToken token) {
                    // Not used for subscribing
                }

                @Override
                public void connectComplete(boolean b, String s) {

                }

                @Override
                public void authPacketArrived(int i, MqttProperties mqttProperties) {

                }
            });

            // Connect to the broker
            System.out.println("Connecting to broker: " + broker);
            client.connect(options);

            // Subscribe to a topic
            client.subscribe(topic, 1);
            System.out.println("Subscribed to topic: " + topic);

        } catch (MqttException e) {
            e.printStackTrace();
        }
    }
}