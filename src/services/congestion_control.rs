use std::time::Duration;

#[derive(Debug)]
pub struct CongestionControlState {
    rtt_min: Duration,
    rtt_max: Duration,
    rtt_previous: Duration,
    base_delay: Duration, // in case there needs to be a minimum base delay
    pub(crate) sleep_duration: Duration,
}

impl CongestionControlState {
    fn new() -> Self {
        CongestionControlState {
            rtt_min: Duration::from_micros(u64::MAX),
            rtt_max: Duration::from_micros(u64::MIN),
            rtt_previous: Duration::from_micros(u64::MAX),
            base_delay: Duration::from_millis(crate::config::get().base_delay_ms),
            sleep_duration: Duration::from_micros(0),
        }
    }

    // inspired by but only loosely based on the acclaimed BBR congestion control method for TCP
    // TODO: windowing state update to ~ 50 to 100 most recent requests
    pub fn update_cc_state(&mut self, current_rtt: &Duration) {
        self.rtt_max = std::cmp::max(self.rtt_max, *current_rtt);
        self.rtt_min = std::cmp::min(self.rtt_min, *current_rtt);
        let rtt_prev = self.rtt_previous;
        self.rtt_previous = *current_rtt;
        let estimated_btl_bw = self.rtt_max - self.rtt_min;
        // base delaying duration, to adjust initial load and avoid spikes to the downstream's queue
        // updated: base_delay is now the minimum delay, which is more intuitive for system operator
        let base_delay = self.base_delay;

        // 0.9 = compensation factor, so (1) it'll never get to 0 delay and (2) to account for the
        // fact that rtt_min is still not approximate rtt_prop and has some queueing overhead on
        // the downstream side, coarsely approximate to be 10%
        let elapsed_scale = current_rtt.as_micros() as f64 - 0.9 * self.rtt_min.as_micros() as f64;
        // base_delay used to be an adjustment to the elapsed time scale, but it's not very
        // intuitive for system operators, who just want to increase/decrease the minumum delay, so
        // the below was scrapped
        // (base_delay + *current_rtt).as_micros() as f64 - 0.9 * self.rtt_min.as_micros() as f64;
        // alternatively
        // let elapsed_scaled = 1.1 * (*current_rtt - self.rtt_min).as_micros() as f64;//

        let w = elapsed_scale / estimated_btl_bw.as_micros() as f64;
        // delaying gain: kinda like a reciprocal to BBR's pacing gain
        let delaying_gain = if *current_rtt > rtt_prev {
            // rtt increases => needs to slow down by increasing delaying duration
            1.0 / (1.0 - 0.75 * w.powi(4))
        } else {
            // rtt decreases => speeds up by decreasing delaying duration
            1.0 / (1.5 - 0.5 * w.powf(0.25))
        };
        let sleep_duration = elapsed_scale * delaying_gain;
        self.sleep_duration = base_delay + Duration::from_micros(sleep_duration as u64);
    }
}

impl Default for CongestionControlState {
    fn default() -> Self {
        Self::new()
    }
}
