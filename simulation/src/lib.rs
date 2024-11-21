use pin_project::pin_project;
use std::error::Error;
use std::future::Future;
use std::pin::{pin, Pin};
use std::task::{Context, Poll};
use std::time::Instant;
use thiserror::Error;

use ts_core::{ApplyConfig, TrafficShaper};

pub mod models;

pub struct SimulationConfig {
    manifest_path: String,
}

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
        let res = self.start_inner().await;
        if let Err(e) = self.ts.cleanup() {
            eprintln!("error during simulation cleanup: {}", e);
        }
        res.map_err(|err| err.into())
    }

    async fn start_inner(&mut self) -> Result<(), SimulationError> {
        if false {
            self.ts
                .enable()
                .map_err(|err| SimulationError::SystemError(err.into()))?;
        }

        self.epoch = Instant::now();

        let mut d = Driver::new(&self.manifest.events, &self.ts, self.epoch.clone());

        pin!(d).await
    }
}

#[pin_project]
struct Driver<'a> {
    pos: usize,
    events: &'a Vec<models::Events>,
    epoch: Instant,
    traffic_shaper: &'a TrafficShaper,
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
        }
    }
}

impl<'a> Future for Driver<'a> {
    type Output = Result<(), SimulationError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let now = Instant::now();

        let event = &this.events[*this.pos];
        let expiry = *this.epoch + event.time;
        if now >= expiry {
            println!("handling event: {:?}", event);

            if false {
                this.traffic_shaper
                    .apply(event.clone().into())
                    .map_err(|err| SimulationError::SystemError(err.into()))?;
            }
            // It has expired
            // signal to the Sim
            // why is this mutation okay?
            *this.pos += 1;
        }

        if *this.pos == this.events.len() {
            Poll::Ready(Ok(()))
        } else {
            let deadline = tokio::time::Instant::from_std(
                this.epoch.checked_add(this.events[*this.pos].time).unwrap(),
            );

            let _ = pin!(tokio::time::sleep_until(deadline)).poll(cx);

            Poll::Pending
        }
    }
}
