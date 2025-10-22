use crate::config::Config;
use crate::metrics::ProducerMetrics;
use crate::topic::TopicGenerator;
use bytes::Bytes;
use chrono::Utc;
use rumqttc::{Client, MqttOptions, QoS};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

pub async fn run_producer(
    producer_id: usize,
    config: Arc<Config>,
    metrics: Arc<ProducerMetrics>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create MQTT connection options
    let client_id = Uuid::new_v4().to_string();
    let mut mqttoptions = MqttOptions::new(
        client_id.clone(),
        config.broker_host.clone(),
        config.broker_port,
    );
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_max_packet_size(100 * 1024, 100 * 1024);
    mqttoptions.set_inflight(1000); // Buffer for many in-flight messages

    // Create client and connection
    let (mut client, mut connection) = Client::new(mqttoptions, 1000);

    // Spawn connection event loop in background - THIS IS CRITICAL
    // The connection task must run to actually process MQTT protocol
    #[allow(unused)]
    let connection_task = tokio::task::spawn_blocking(move || {
        loop {
            match connection.recv() {
                Ok(evt) => {
                    // Just drain the events, we don't care about them for this test
                    match evt {
                        Err(e) => {
                            eprintln!("Producer {}: Connection error: {}", producer_id, e);
                            break;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    eprintln!("Producer {}: Receive error: {:?}", producer_id, e);
                    break;
                }
            }
        }
    });

    // Give the connection a moment to establish
    sleep(Duration::from_millis(500)).await;

    // Generate topics for this producer
    let topic_gen = TopicGenerator::new(
        config.topic_prefix.clone(),
        producer_id + 1,
        config.topics_per_node,
        config.max_depth,
    );

    let topics: Vec<String> = topic_gen.generate_all();
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

    // Main publishing loop
    loop {
        for topic in &topics {
            // Skip metrics topics (we handle those separately)
            if topic.contains("/metrics") {
                continue;
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

            match client.publish(topic.clone(), qos, config.retained, payload_bytes) {
                Ok(_) => {
                    metrics.increment_published();
                }
                Err(e) => {
                    eprintln!("Producer {}: Publish error: {}", producer_id + 1, e);
                }
            }

            // Sleep between publishes
            sleep(Duration::from_millis(config.sleep_ms)).await;
        }

        // Publish metrics every cycle
        let metrics_topic = format!("{}{:05}/metrics", config.topic_prefix, producer_id + 1);
        let metrics_counter = metrics.get_counter();
        let metrics_total = metrics.get_total();
        let metrics_payload = json!({
            "ts": Utc::now().to_rfc3339(),
            "producer_id": producer_id + 1,
            "total_published": metrics_total,
            "counter": metrics_counter,
        });

        let payload_bytes = Bytes::from(metrics_payload.to_string());

        match client.publish(&metrics_topic, qos, config.retained, payload_bytes) {
            Ok(_) => {
                metrics.increment_published();
            }
            Err(e) => {
                eprintln!(
                    "Producer {}: Metrics publish error: {}",
                    producer_id + 1,
                    e
                );
            }
        }
    }
}
