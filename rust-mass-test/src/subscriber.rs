use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
use tokio::time;
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::watch;
use uuid::Uuid;

use crate::config::Config;
use crate::metrics::ClientMetrics;
use crate::topic::TopicGenerator;

pub async fn run(config: Arc<Config>, metrics: Arc<ClientMetrics>, mut shutdown_rx: watch::Receiver<bool>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client_id = format!("sub-{}", Uuid::new_v4().to_string());
    eprintln!("Subscriber {}: [DEBUG] Connecting to {}:{} with client ID {}", metrics.id + 1, &config.broker_host, config.broker_port, client_id);
    let mut mqttoptions = MqttOptions::new(client_id.clone(), &config.broker_host, config.broker_port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    eprintln!("Subscriber {}: [DEBUG] MqttOptions configured", metrics.id + 1);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    eprintln!("Subscriber {}: [DEBUG] Client created, waiting for events", metrics.id + 1);

    // Generate all topics first
    let topic_generator = TopicGenerator::new(
        config.topic_prefix.clone(),
        metrics.id + 1, // Use client's ID as base_topic_index
        config.topics_per_node,
        config.max_depth,
    );
    let all_topics = topic_generator.generate_all();
    let total_topics_count = all_topics.len();

    // Select a percentage of topics to subscribe to
    let num_topics_to_subscribe = (total_topics_count as f64 * (config.subscribe_percentage as f64 / 100.0)).round() as usize;
    let mut topics_to_subscribe: Vec<String> = all_topics
        .into_iter()
        .collect();
    fastrand::shuffle(&mut topics_to_subscribe);
    topics_to_subscribe.truncate(num_topics_to_subscribe);

    let sub_count = topics_to_subscribe.len();
    eprintln!("Subscriber {}: Subscribing to {} topics...", metrics.id + 1, sub_count);

    let mut topic_index = 0;
    let mut subscribed_count = 0;

    // Event-driven subscription loop: wait for ConnAck, then subscribe to topics one at a time
    loop {
        let event = eventloop.poll().await;

        match event {
            Ok(Event::Incoming(rumqttc::Packet::ConnAck(ack))) => {
                eprintln!("Subscriber {}: ✅ Connected to broker: {:?}", metrics.id + 1, ack);
                // After ConnAck, subscribe to the first topic
                if topic_index < sub_count {
                    let topic = &topics_to_subscribe[topic_index];
                    match client.subscribe(topic, QoS::AtMostOnce).await {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Subscriber {}: ❌ Failed to subscribe to {}: {:?}", metrics.id + 1, topic, e);
                        }
                    }
                    topic_index += 1;
                }
            }
            Ok(Event::Incoming(rumqttc::Packet::SubAck(_suback))) => {
                subscribed_count += 1;
                eprintln!("Subscriber {}: ✅ Received SUBACK {}/{}", metrics.id + 1, subscribed_count, sub_count);

                if subscribed_count >= sub_count {
                    eprintln!("Subscriber {}: ✅✅ All {} subscriptions confirmed!", metrics.id + 1, sub_count);
                    break;
                }

                // Subscribe to next topic
                if topic_index < sub_count {
                    let topic = &topics_to_subscribe[topic_index];
                    match client.subscribe(topic, QoS::AtMostOnce).await {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Subscriber {}: ❌ Failed to subscribe to {}: {:?}", metrics.id + 1, topic, e);
                        }
                    }
                    topic_index += 1;
                }
            }
            Ok(Event::Incoming(_packet)) => {
                // Ignore other packets during subscription phase
            }
            Ok(Event::Outgoing(_outgoing)) => {
                // Ignore outgoing events
            }
            Err(e) => {
                eprintln!("Subscriber {}: ❌ Error: {:?}", metrics.id + 1, e);
                return Err(e.into());
            }
        }
    }

    println!("Subscribed to {} of {} topics.", topics_to_subscribe.len(), total_topics_count);

    // Event loop to process incoming messages
    eprintln!("Subscriber {}: Now receiving messages...", metrics.id + 1);
    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                eprintln!("Subscriber {}: Received shutdown signal. Disconnecting...", metrics.id + 1);
                client.disconnect().await?;
                break;
            }
            event = eventloop.poll() => {
                match event {
                    Ok(Event::Incoming(rumqttc::Packet::Publish(_p))) => {
                        metrics.increment_received();
                    }
                    Ok(Event::Incoming(rumqttc::Packet::Disconnect)) => {
                        eprintln!("Subscriber {}: ❌ Broker sent DISCONNECT!", metrics.id + 1);
                        break;
                    }
                    Ok(Event::Incoming(_packet)) => {
                        // Ignore other packets (like PingResp, ConnAck, etc.)
                    }
                    Ok(Event::Outgoing(_outgoing)) => {
                        // Ignore outgoing events
                    }
                    Err(e) => {
                        eprintln!("Subscriber {}: ❌ Eventloop error: {:?}", metrics.id + 1, e);
                        time::sleep(Duration::from_secs(1)).await;
                        // Continue looping to attempt recovery
                    }
                }
            }
        }
    }

    Ok(())
}
