use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant};

static CLOCK_STARTED: Signal<CriticalSectionRawMutex, Clock> = Signal::new();

#[derive(Copy, Clone)]
pub struct Clock {
    instant: Instant,
    latency: Duration,
}

impl Clock {
    pub fn start(latency: Duration) {
        CLOCK_STARTED.signal(Self {
            instant: Instant::now(),
            latency,
        });
    }

    pub async fn started() -> Self {
        CLOCK_STARTED.wait().await
    }

    pub fn elapsed(&self) -> Duration {
        self.instant
            .elapsed()
            .checked_sub(self.latency)
            .unwrap_or(Duration::MIN)
    }
}
