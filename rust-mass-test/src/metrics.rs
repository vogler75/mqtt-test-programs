use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct ClientMetrics {
    pub id: usize,
    total_published: Arc<AtomicU64>,
    total_received: Arc<AtomicU64>,
    #[allow(dead_code)]
    last_pub_vps_time: Arc<AtomicU64>,
    #[allow(dead_code)]
    last_pub_vps_count: Arc<AtomicU64>,
    #[allow(dead_code)]
    cached_pub_vps: Arc<AtomicU64>,
    #[allow(dead_code)]
    last_recv_vps_time: Arc<AtomicU64>,
    #[allow(dead_code)]
    last_recv_vps_count: Arc<AtomicU64>,
    #[allow(dead_code)]
    cached_recv_vps: Arc<AtomicU64>,
    connected: Arc<AtomicBool>,
}

impl ClientMetrics {
    pub fn new(id: usize) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        ClientMetrics {
            id,
            total_published: Arc::new(AtomicU64::new(0)),
            total_received: Arc::new(AtomicU64::new(0)),
            last_pub_vps_time: Arc::new(AtomicU64::new(now)),
            last_pub_vps_count: Arc::new(AtomicU64::new(0)),
            cached_pub_vps: Arc::new(AtomicU64::new(0)),
            last_recv_vps_time: Arc::new(AtomicU64::new(now)),
            last_recv_vps_count: Arc::new(AtomicU64::new(0)),
            cached_recv_vps: Arc::new(AtomicU64::new(0)),
            connected: Arc::new(AtomicBool::new(false)),
        }
    }

    #[allow(dead_code)]
    pub fn increment_published(&self) {
        self.total_published.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn increment_received(&self) {
        self.total_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_total_published(&self) -> u64 {
        self.total_published.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn get_total_received(&self) -> u64 {
        self.total_received.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn get_counter(&self) -> u64 {
        // Counter is just the total count (sequence number)
        self.total_published.load(Ordering::Relaxed)
    }

    pub fn calculate_vps(&self) -> f64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let last_time = self.last_pub_vps_time.load(Ordering::Relaxed);

        // Only recalculate if at least 1 second has passed
        if now <= last_time {
            // Return cached value
            let cached_bits = self.cached_pub_vps.load(Ordering::Relaxed);
            return f64::from_bits(cached_bits);
        }

        let current_count = self.total_published.load(Ordering::Relaxed);
        let last_count = self.last_pub_vps_count.load(Ordering::Relaxed);

        let time_delta = now.saturating_sub(last_time);
        if time_delta == 0 {
            return 0.0;
        }

        let count_delta = current_count.saturating_sub(last_count);
        let vps = count_delta as f64 / time_delta as f64;

        // Update for next check
        self.last_pub_vps_time.store(now, Ordering::Relaxed);
        self.last_pub_vps_count.store(current_count, Ordering::Relaxed);
        self.cached_pub_vps.store(vps.to_bits(), Ordering::Relaxed);

        vps
    }

    #[allow(dead_code)]
    pub fn calculate_received_vps(&self) -> f64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let last_time = self.last_recv_vps_time.load(Ordering::Relaxed);

        // Only recalculate if at least 1 second has passed
        if now <= last_time {
            // Return cached value
            let cached_bits = self.cached_recv_vps.load(Ordering::Relaxed);
            return f64::from_bits(cached_bits);
        }

        let current_count = self.total_received.load(Ordering::Relaxed);
        let last_count = self.last_recv_vps_count.load(Ordering::Relaxed);

        let time_delta = now.saturating_sub(last_time);
        if time_delta == 0 {
            return 0.0;
        }

        let count_delta = current_count.saturating_sub(last_count);
        let vps = count_delta as f64 / time_delta as f64;

        // Update for next check
        self.last_recv_vps_time.store(now, Ordering::Relaxed);
        self.last_recv_vps_count.store(current_count, Ordering::Relaxed);
        self.cached_recv_vps.store(vps.to_bits(), Ordering::Relaxed);

        vps
    }

    #[allow(dead_code)]
    pub fn reset(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.total_published.store(0, Ordering::Relaxed);
        self.total_received.store(0, Ordering::Relaxed);
        self.last_pub_vps_time.store(now, Ordering::Relaxed);
        self.last_pub_vps_count.store(0, Ordering::Relaxed);
        self.cached_pub_vps.store(0, Ordering::Relaxed);
        self.last_recv_vps_time.store(now, Ordering::Relaxed);
        self.last_recv_vps_count.store(0, Ordering::Relaxed);
        self.cached_recv_vps.store(0, Ordering::Relaxed);
    }

    pub fn set_connected(&self, connected: bool) {
        self.connected.store(connected, Ordering::Relaxed);
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}

pub struct GlobalMetrics {
    pub clients: Vec<ClientMetrics>,
}

impl GlobalMetrics {
    pub fn new(num_clients: usize) -> Self {
        let clients = (0..num_clients)
            .map(|i| ClientMetrics::new(i))
            .collect();

        GlobalMetrics { clients }
    }

    pub fn get_total_published(&self) -> u64 {
        self.clients.iter().map(|p| p.get_total_published()).sum()
    }

    #[allow(dead_code)]
    pub fn get_total_received(&self) -> u64 {
        self.clients.iter().map(|p| p.get_total_received()).sum()
    }

    pub fn get_total_vps(&self) -> f64 {
        self.clients.iter().map(|p| p.calculate_vps()).sum()
    }

    #[allow(dead_code)]
    pub fn get_total_received_vps(&self) -> f64 {
        self.clients.iter().map(|p| p.calculate_received_vps()).sum()
    }

    #[allow(dead_code)]
    pub fn reset(&self) {
        for client in &self.clients {
            client.reset();
        }
    }

    pub fn get_connected_count(&self) -> usize {
        self.clients.iter().filter(|c| c.is_connected()).count()
    }
}
