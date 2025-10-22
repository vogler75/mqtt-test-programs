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

    // Generate topic generator (once, reuse for reconnections)
    let topic_generator = TopicGenerator::new(
        config.topic_prefix.clone(),
        metrics.id + 1,
        config.topics_per_node,
        config.max_depth,
    );

    // Get topics to subscribe to (once, reuse for reconnections)
    let (all_topics, is_wildcard) = if config.use_leafs {
        if config.use_wildcard {
            (topic_generator.generate_wildcard_subscriptions(), true)
        } else {
            (topic_generator.generate_leaves_only(), false)
        }
    } else {
        if config.use_wildcard {
            (topic_generator.generate_single_wildcard(), true)
        } else {
            (topic_generator.generate_all(), false)
        }
    };

    let total_topics_count = all_topics.len();

    // Select a percentage of topics to subscribe to
    let num_topics_to_subscribe = (total_topics_count as f64 * (config.subscribe_percentage as f64 / 100.0)).round() as usize;
    let mut topics_to_subscribe: Vec<String> = all_topics
        .into_iter()
        .collect();
    if !is_wildcard {
        fastrand::shuffle(&mut topics_to_subscribe);
    }
    topics_to_subscribe.truncate(num_topics_to_subscribe);

    let sub_count = topics_to_subscribe.len();

    // Outer loop for reconnection attempts
    loop {
        // Check for shutdown before attempting to connect
        if *shutdown_rx.borrow() {
            eprintln!("Subscriber {}: Received shutdown signal. Exiting...", metrics.id + 1);
            return Ok(());
        }

        eprintln!("Subscriber {}: [DEBUG] Connecting to {}:{} with client ID {}", metrics.id + 1, &config.broker_host, config.broker_port, client_id);
        let mut mqttoptions = MqttOptions::new(client_id.clone(), &config.broker_host, config.broker_port);
        mqttoptions.set_keep_alive(Duration::from_secs(120));
        eprintln!("Subscriber {}: [DEBUG] MqttOptions configured", metrics.id + 1);

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
        eprintln!("Subscriber {}: [DEBUG] Client created, waiting for events", metrics.id + 1);

        // Debug output to show what we're actually subscribing to
        if config.use_leafs && config.use_wildcard {
            eprintln!("Subscriber {}: Using WILDCARD at parent-of-leaf level: {:?}", metrics.id + 1, topics_to_subscribe);
        } else if config.use_leafs {
            eprintln!("Subscriber {}: Using individual LEAF topics ({} total)", metrics.id + 1, sub_count);
        } else if config.use_wildcard {
            eprintln!("Subscriber {}: Using WILDCARD subscription at base level: {:?}", metrics.id + 1, topics_to_subscribe);
        } else {
            eprintln!("Subscriber {}: Using ALL topics ({} total)", metrics.id + 1, sub_count);
        }

        eprintln!("Subscriber {}: Subscribing to {} topics...", metrics.id + 1, sub_count);

        let mut topic_index = 0;
        let mut subscribed_count = 0;
        let mut connection_phase = true;
        let mut should_shutdown = false;

        // Event-driven subscription loop: wait for ConnAck, then subscribe to topics one at a time
        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    eprintln!("Subscriber {}: Received shutdown signal. Disconnecting...", metrics.id + 1);
                    let _ = client.disconnect().await;
                    should_shutdown = true;
                    break;
                }
                event = eventloop.poll() => {
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
                                connection_phase = false;
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
                            eprintln!("Subscriber {}: ⚠️  Connection error during subscription: {:?}, reconnecting...", metrics.id + 1, e);
                            break;
                        }
                    }
                }
            }
        }

        // If shutdown was received, exit the outer loop
        if should_shutdown {
            break;
        }

        // If we're still in connection phase, something went wrong, reconnect
        if connection_phase {
            eprintln!("Subscriber {}: Failed to complete subscriptions, reconnecting in 2 seconds...", metrics.id + 1);
            time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        println!("Subscribed to {} of {} topics.", topics_to_subscribe.len(), total_topics_count);

        // Event loop to process incoming messages
        eprintln!("Subscriber {}: Now receiving messages...", metrics.id + 1);
        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    eprintln!("Subscriber {}: Received shutdown signal. Disconnecting...", metrics.id + 1);
                    let _ = client.disconnect().await;
                    should_shutdown = true;
                    break;
                }
                event = eventloop.poll() => {
                    match event {
                        Ok(Event::Incoming(rumqttc::Packet::Publish(_p))) => {
                            metrics.increment_received();
                        }
                        Ok(Event::Incoming(rumqttc::Packet::Disconnect)) => {
                            eprintln!("Subscriber {}: ⚠️  Broker sent DISCONNECT, reconnecting...", metrics.id + 1);
                            break;
                        }
                        Ok(Event::Incoming(_packet)) => {
                            // Ignore other packets
                        }
                        Ok(Event::Outgoing(_outgoing)) => {
                            // Ignore outgoing events
                        }
                        Err(e) => {
                            eprintln!("Subscriber {}: ⚠️  Connection error: {:?}, reconnecting...", metrics.id + 1, e);
                            break;
                        }
                    }
                }
            }
        }

        // If shutdown was received, exit the outer loop
        if should_shutdown {
            break;
        }

        eprintln!("Subscriber {}: Reconnecting in 2 seconds...", metrics.id + 1);
        time::sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}
