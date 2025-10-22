use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct ProducerMetrics {
    pub id: usize,
    total_published: Arc<AtomicU64>,
    last_vps_time: Arc<AtomicU64>,
    last_vps_count: Arc<AtomicU64>,
    cached_vps: Arc<AtomicU64>, // Store as u64 bits to avoid sync issues
}

impl ProducerMetrics {
    pub fn new(id: usize) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        ProducerMetrics {
            id,
            total_published: Arc::new(AtomicU64::new(0)),
            last_vps_time: Arc::new(AtomicU64::new(now)),
            last_vps_count: Arc::new(AtomicU64::new(0)),
            cached_vps: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn increment_published(&self) {
        self.total_published.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_total(&self) -> u64 {
        self.total_published.load(Ordering::Relaxed)
    }

    pub fn get_counter(&self) -> u64 {
        // Counter is just the total count (sequence number)
        self.total_published.load(Ordering::Relaxed)
    }

    pub fn calculate_vps(&self) -> f64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let last_time = self.last_vps_time.load(Ordering::Relaxed);

        // Only recalculate if at least 1 second has passed
        if now <= last_time {
            // Return cached value
            let cached_bits = self.cached_vps.load(Ordering::Relaxed);
            return f64::from_bits(cached_bits);
        }

        let current_count = self.total_published.load(Ordering::Relaxed);
        let last_count = self.last_vps_count.load(Ordering::Relaxed);

        let time_delta = now.saturating_sub(last_time);
        if time_delta == 0 {
            return 0.0;
        }

        let count_delta = current_count.saturating_sub(last_count);
        let vps = count_delta as f64 / time_delta as f64;

        // Update for next check
        self.last_vps_time.store(now, Ordering::Relaxed);
        self.last_vps_count.store(current_count, Ordering::Relaxed);
        self.cached_vps.store(vps.to_bits(), Ordering::Relaxed);

        vps
    }
}

pub struct GlobalMetrics {
    pub producers: Vec<ProducerMetrics>,
}

impl GlobalMetrics {
    pub fn new(num_producers: usize) -> Self {
        let producers = (0..num_producers)
            .map(|i| ProducerMetrics::new(i))
            .collect();

        GlobalMetrics { producers }
    }

    pub fn get_total_published(&self) -> u64 {
        self.producers.iter().map(|p| p.get_total()).sum()
    }

    pub fn get_total_vps(&self) -> f64 {
        self.producers.iter().map(|p| p.calculate_vps()).sum()
    }

    pub fn get_producer(&self, id: usize) -> Option<&ProducerMetrics> {
        self.producers.get(id)
    }
}
