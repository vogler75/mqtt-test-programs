use crate::config::Config;
use crate::metrics::ClientMetrics;
use crate::topic::TopicGenerator;
use bytes::Bytes;
use chrono::Utc;
use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::{time, sync::watch};
use uuid::Uuid;

pub async fn run_producer(
    producer_id: usize,
    config: Arc<Config>,
    metrics: Arc<ClientMetrics>,
    mut shutdown_rx: watch::Receiver<bool>,
    mut pause_rx: watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create MQTT connection options
    let client_id = format!("pub-{}", Uuid::new_v4().to_string());
    let mut mqttoptions = MqttOptions::new(
        client_id.clone(),
        config.broker_host.clone(),
        config.broker_port,
    );
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_max_packet_size(100 * 1024, 100 * 1024);
    mqttoptions.set_inflight(1000); // Buffer for many in-flight messages

    // Create client and connection
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    eprintln!("Producer {}: Waiting for connection to broker...", producer_id + 1);

    // Wait for CONNACK before proceeding
    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                eprintln!("Producer {}: Received shutdown signal before connecting. Disconnecting...", producer_id + 1);
                client.disconnect().await?;
                return Ok(());
            }
            event = eventloop.poll() => {
                match event {
                    Ok(Event::Incoming(rumqttc::Packet::ConnAck(_ack))) => {
                        eprintln!("Producer {}: ✅ Connected to broker", producer_id + 1);
                        break;
                    }
                    Ok(Event::Incoming(_)) => {},
                    Ok(Event::Outgoing(_)) => {},
                    Err(e) => {
                        eprintln!("Producer {}: ❌ Connection error: {:?}", producer_id + 1, e);
                        return Err(e.into());
                    }
                }
            }
        }
    }

    // Generate topics for this producer
    let topic_gen = TopicGenerator::new(
        config.topic_prefix.clone(),
        producer_id + 1,
        config.topics_per_node,
        config.max_depth,
    );

    let topics: Vec<String> = if config.use_leafs {
        topic_gen.generate_leaves_only()
    } else {
        topic_gen.generate_all()
    };
    let qos = match config.qos {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        _ => QoS::ExactlyOnce,
    };

    eprintln!(
        "Producer {}: Starting with {} topics, sleep_ms={}",
        producer_id + 1,
        topics.len(),
        config.sleep_ms
    );

    // Create a timer for publishing with the configured sleep_ms
    let mut publish_timer = time::interval(Duration::from_millis(config.sleep_ms));

    // Track which topic to publish to
    let mut topic_index = 0;

    // Main publishing loop
    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                eprintln!("Producer {}: Received shutdown signal. Disconnecting...", producer_id + 1);
                client.disconnect().await?;
                break;
            }
            _ = pause_rx.changed() => {
                // Pause state changed, just acknowledge it by continuing the loop
            }
            event = eventloop.poll() => {
                match event {
                    Ok(Event::Incoming(rumqttc::Packet::Disconnect)) => {
                        eprintln!("Producer {}: ❌ Broker sent DISCONNECT!", producer_id + 1);
                        break;
                    }
                    Ok(Event::Incoming(_)) => {},
                    Ok(Event::Outgoing(_)) => {},
                    Err(e) => {
                        eprintln!("Producer {}: Eventloop error: {:?}", producer_id + 1, e);
                        time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            _ = publish_timer.tick() => {
                // Check if paused before publishing
                if *pause_rx.borrow() {
                    continue; // Skip publishing but keep the timer ticking
                }

                // Generate payload
                let counter = metrics.get_counter();
                let random_value: f64 = fastrand::f64();
                let timestamp = Utc::now().to_rfc3339();

                let payload = json!({
                    "ts": timestamp,
                    "counter": counter,
                    "value": random_value,
                });

                let payload_bytes = Bytes::from(payload.to_string());

                let topic = &topics[topic_index % topics.len()];
                match client.publish(topic.clone(), qos, config.retained, payload_bytes).await {
                    Ok(_) => {
                        metrics.increment_published();
                    }
                    Err(e) => {
                        eprintln!("Producer {}: Publish error: {}", producer_id + 1, e);
                    }
                }

                // Move to next topic for next publish
                topic_index += 1;
            }
        }
    }

    Ok(())
}
