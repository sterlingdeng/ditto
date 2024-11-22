use std::future::Future;
use std::pin::{pin, Pin};
use std::task::{Context, Poll};
use std::time::Instant;

use pin_project::pin_project;
use thiserror::Error;
use tracing::{error, info};
use ts_core::TrafficShaper;

pub mod models;

pub struct Simulation {
    manifest: models::Manifest,
    epoch: Instant,
    ts: TrafficShaper,
}

#[derive(Error, Debug)]
pub enum SimulationError {
    #[error("System error: {0}")]
    SystemError(#[from] Box<dyn std::error::Error + Sync + Send>),
}

impl Simulation {
    pub fn new(manifest: models::Manifest, epoch: Instant) -> Self {
        let ts_config = manifest.config.clone().into();
        let ts = TrafficShaper::new(ts_config);
        Self {
            manifest,
            epoch,
            ts,
        }
    }
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        info!("starting simulation");
        let res = self.start_inner().await;
        info!("cleaning up");
        if let Err(e) = self.ts.cleanup() {
            error!("error during simulation cleanup: {}", e);
        }
        res.map_err(|err| err.into())
    }

    async fn start_inner(&mut self) -> Result<(), SimulationError> {
        self.ts
            .enable()
            .map_err(|err| SimulationError::SystemError(err.into()))?;

        self.epoch = Instant::now();

        let mut d = Driver::new(&self.manifest.events, &self.ts, self.epoch.clone());

        let res = pin!(d).await;
        res
    }
}

#[pin_project]
struct Driver<'a> {
    pos: usize,
    events: &'a Vec<models::Events>,
    epoch: Instant,
    traffic_shaper: &'a TrafficShaper,
    #[pin]
    sleep: tokio::time::Sleep,
}

impl<'a> Driver<'a> {
    fn new(
        events: &'a Vec<models::Events>,
        traffic_shaper: &'a TrafficShaper,
        epoch: Instant,
    ) -> Self {
        Self {
            pos: 0,
            events,
            epoch,
            traffic_shaper,
            sleep: tokio::time::sleep(std::time::Duration::ZERO),
        }
    }
}

impl<'a> Future for Driver<'a> {
    type Output = Result<(), SimulationError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        let now = Instant::now();

        let event = &this.events[*this.pos];
        let expiry = *this.epoch + event.time;
        if now >= expiry {
            info!(
                "applying event: {:?} {}/{}",
                event,
                *this.pos + 1,
                this.events.len()
            );

            this.traffic_shaper
                .apply(event.clone().into())
                .map_err(|err| SimulationError::SystemError(err.into()))?;

            *this.pos += 1;
        }

        if *this.pos == this.events.len() {
            Poll::Ready(Ok(()))
        } else {
            let deadline = tokio::time::Instant::from_std(
                this.epoch.checked_add(this.events[*this.pos].time).unwrap(),
            );
            this.sleep.as_mut().reset(deadline);
            let _ = this.sleep.poll(cx);
            Poll::Pending
        }
    }
}
