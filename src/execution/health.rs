use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Default, Debug, Clone)]
pub struct HealthPollUpdate {
    pub health_json: Option<String>,
    pub positions_json: Option<String>,
    pub api_offline: bool,
    pub api_degraded: bool,
    pub postgres_available: bool,
}

pub struct HealthPoller {
    api_base: String,
    cancel: Arc<AtomicBool>,
    interval: Duration,
}

impl HealthPoller {
    pub fn new(api_base: &str) -> Self {
        Self {
            api_base: api_base.to_string(),
            cancel: Arc::new(AtomicBool::new(false)),
            interval: Duration::from_secs(2),
        }
    }

    pub fn with_interval(api_base: &str, interval: Duration) -> Self {
        Self {
            api_base: api_base.to_string(),
            cancel: Arc::new(AtomicBool::new(false)),
            interval,
        }
    }

    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }

    pub async fn poll_step(client: &reqwest::Client, api_base: &str) -> HealthPollUpdate {
        let health_url = format!("{api_base}/api/v1/execution/health");
        let positions_url = format!("{api_base}/api/v1/execution/positions");

        let health_resp = client.get(&health_url).send().await;
        let mut update = HealthPollUpdate::default();

        match health_resp {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    if let Ok(text) = resp.text().await {
                        update.health_json = Some(text);
                        update.postgres_available = true;
                    }
                } else if status.as_u16() == 503 {
                    update.api_degraded = true;
                    update.postgres_available = false;
                } else {
                    update.api_offline = true;
                    update.postgres_available = false;
                }
            }
            Err(_) => {
                update.api_offline = true;
                update.postgres_available = false;
            }
        }

        let pos_resp = client.get(&positions_url).send().await;
        if let Ok(resp) = pos_resp {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    update.positions_json = Some(text);
                }
            }
        }

        update
    }

    pub async fn run_loop<F>(&self, cancel: Arc<AtomicBool>, interval: Duration, on_update: F)
    where
        F: Fn(HealthPollUpdate) + Send + Sync + 'static,
    {
        let client = reqwest::Client::new();
        let mut timer = tokio::time::interval(interval);

        while !cancel.load(Ordering::Relaxed) {
            timer.tick().await;
            if cancel.load(Ordering::Relaxed) {
                break;
            }

            let update = Self::poll_step(&client, &self.api_base).await;
            on_update(update);
        }
    }

    pub fn start<F>(self: &Arc<Self>, on_update: F)
    where
        F: Fn(HealthPollUpdate) + Send + Sync + 'static,
    {
        let poller = self.clone();
        let cancel = self.cancel.clone();
        let interval = self.interval;
        let on_update = Arc::new(on_update);

        std::thread::Builder::new()
            .name("health-poller".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(_) => return,
                };

                rt.block_on(async move {
                    poller
                        .run_loop(cancel, interval, move |u| on_update(u))
                        .await;
                });
            })
            .ok();
    }

    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
