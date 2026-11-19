use std::{
    collections::HashMap,
    net::IpAddr,
    time::{Duration, Instant},
};

/// Fixed-window budget for new pastes: one per address and one shared by everyone.
pub struct Limiter {
    window: Duration,
    per_address: u32,
    total: u32,
    started: Instant,
    used: HashMap<IpAddr, u32>,
    used_total: u32,
}

impl Limiter {
    pub fn new(window: Duration, per_address: u32, total: u32, now: Instant) -> Self {
        Self {
            window,
            per_address,
            total,
            started: now,
            used: HashMap::new(),
            used_total: 0,
        }
    }

    pub fn allow(&mut self, address: IpAddr, now: Instant) -> bool {
        if now.duration_since(self.started) >= self.window {
            self.started = now;
            self.used.clear();
            self.used_total = 0;
        }
        let used = self.used.entry(address).or_default();
        if *used >= self.per_address || self.used_total >= self.total {
            return false;
        }
        *used += 1;
        self.used_total += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    const HOUR: Duration = Duration::from_secs(3600);

    fn address(last: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, last))
    }

    #[test]
    fn budget_is_tracked_per_address_and_resets_after_the_window() {
        let start = Instant::now();
        let mut limiter = Limiter::new(HOUR, 2, 100, start);

        assert!(limiter.allow(address(1), start));
        assert!(limiter.allow(address(1), start));
        assert!(!limiter.allow(address(1), start));
        assert!(limiter.allow(address(2), start));
        assert!(limiter.allow(address(1), start + HOUR));
    }

    #[test]
    fn total_budget_caps_all_addresses_together() {
        let start = Instant::now();
        let mut limiter = Limiter::new(HOUR, 2, 3, start);

        let allowed = (1..=5).filter(|last| limiter.allow(address(*last), start));

        assert_eq!(allowed.count(), 3);
    }
}
