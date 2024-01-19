#![allow(unused_parens)]
use core::time::Duration;
use std::sync::Arc;

use bytes::Bytes;
use chrono::DateTime;
use clap::Parser;
use hass_rs::client;
use hass_rs::MqttEvent;
use influxdb::InfluxDbWriteable;
use influxdb::Query;
use influxdb::Timestamp;
use parking_lot::RwLock;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::task;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

mod filter;
use filter::DefaultFilter;
use filter::EntityFilter;
mod metadata;
use metadata::MetadataTree;
mod value;
use value::Value;

#[derive(Debug, Parser)]
struct Config {
	#[clap(env)]
	hass_host: String,
	#[clap(env, long, default_value_t = 80)]
	hass_port: u16,
	#[clap(env)]
	hass_token: String,
	#[clap(env, long)]
	mqtt_host: String,
	#[clap(env, long, default_value_t = 1883)]
	mqtt_port: u16,
	#[clap(env, long)]
	mqtt_topic: String,
	#[clap(long, env, default_value_t = DefaultFilter::Allow)]
	default_filter: DefaultFilter,
	/// If the default is "allow", any entities that match the filter will be denied; if the
	/// default is "deny", any entities that match the filter will be allowed.
	#[clap(long, env, default_value = "[]")]
	entity_filter: EntityFilter,
	#[clap(short = 'j', long, env, default_value_t = 1)]
	workers: u8,
	#[command(flatten)]
	influxdb: influxdb_config::InfluxDbConfig
}

impl Config {
	async fn mqtt(&self) -> Result<(rumqttc::AsyncClient, rumqttc::EventLoop), rumqttc::ClientError> {
		let mut opts = rumqttc::MqttOptions::new("collector", &self.mqtt_host, self.mqtt_port);
		opts.set_keep_alive(Duration::from_secs(30));
		opts.set_manual_acks(false);

		let (client, stream) = rumqttc::AsyncClient::new(opts, self.workers as usize * 2);
		client.subscribe(&self.mqtt_topic, rumqttc::QoS::AtMostOnce).await?;

		Ok((client, stream))
	}

	fn spawn_workers(&self, metadata: Arc<RwLock<MetadataTree>>) -> (async_channel::Sender<Bytes>, Vec<task::JoinHandle<()>>) {
		let (tx, rx) = async_channel::bounded(self.workers as usize * 2);
		let mut workers = Vec::with_capacity(self.workers as usize);
		for i in 0..self.workers {
			let default_filter = self.default_filter;
			let filter = self.entity_filter.clone();
			let influx = self.influxdb.client();
			let rx: async_channel::Receiver<Bytes> = rx.clone();
			let metadata = metadata.clone();
			workers.push(task::spawn(async move {
				while let Ok(ev) = rx.recv().await {
					let ev: MqttEvent = match serde_json::from_slice(ev.as_ref()) {
						Ok(v) => v,
						Err(e) => {
							error!(error=?e, event=?std::str::from_utf8(ev.as_ref()), "Failed to deserialize event");
							continue;
						}
					};

					if ((default_filter == DefaultFilter::Deny && !filter.matches_event(&ev)) || (default_filter == DefaultFilter::Allow && filter.matches_event(&ev))) {
						debug!(entity_id = ev.event_data.entity_id, "Did not match filter");
						continue;
					}

					let Some(state) = ev.event_data.new_state.as_ref() else {
						warn!(entity_id = ev.event_data.entity_id, "New state missing");
						continue;
					};

					let Some((kind, _)) = state.entity_id.split_once('.') else {
						warn!(entity_id = state.entity_id, "Invalid entity ID");
						continue;
					};

					let Ok(ts) = DateTime::parse_from_rfc3339(&state.last_updated) else {
						error!(ts = state.last_updated, "Failed to parse timestamp");
						continue;
					};

					let point = {
						let metadata = metadata.read();
						let Some(meta) = metadata.find(&state.entity_id) else {
							warn!(entity_id = state.entity_id, "Metadata not found");
							continue;
						};

						let Ok(value) = state.state.parse::<Value>() else {
							warn!(entity_id=meta.entity.entity_id, value=state.state, "Failed to parse numerical value from state");
							continue;
						};

						let mut point = Timestamp::from(ts)
							.into_query(format!("hass:{kind}"))
							.add_field("value", value.0)
							.add_tag("entity.id", meta.entity.entity_id.as_str())
							.add_tag("device.name", meta.device.name.as_str());

						if let Some(name) = meta.entity.name.as_ref().or(meta.entity.original_name.as_ref()) {
							point = point.add_tag("entity.name", name.as_str());
						}

						if let Some(area) = meta.area.as_ref() {
							point = point.add_tag("device.area", area.name.as_str());
						}

						if let Some(class) = state.attributes.get("device_class").and_then(|v| v.as_str()) {
							point = point.add_tag("device.class", class);
						}

						point
					};

					match point.build() {
						Ok(v) => info!(point=%v.get(), "Built datapoint"),
						Err(e) => {
							error!(error=?e, entity_id=state.entity_id, "Failed to build datapoint");
							continue;
						}
					}

					if let Err(e) = influx.query(point).await {
						error!(error=?e, state.entity_id, "Failed to write data");
					}
				}

				info!(worker = i, "Channel closed; shutting down worker");
			}));
		}

		(tx, workers)
	}
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
	let mut config = Config::parse();
	tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).compact().init();

	let mut client = client::connect(&config.hass_host, config.hass_port).await.unwrap();
	client.auth_with_longlivedtoken(std::mem::take(&mut config.hass_token)).await.unwrap();

	let meta = Arc::new(RwLock::new(MetadataTree::load(&mut client).await.unwrap()));
	let meta_handle = {
		let meta = meta.clone();
		task::spawn(async move {
			let mut tick = tokio::time::interval(Duration::from_secs(10));
			let mut fail_count = 0;
			loop {
				if(fail_count > 4) {
					break;
				}
				tick.tick().await;
				let new_meta = match MetadataTree::load(&mut client).await {
					Ok(v) => v,
					Err(e) => {
						error!(error=?e, fail_count, "Failed to get metadata");
						fail_count += 1;
						continue;
					}
				};
				*meta.write() = new_meta;
			}
			error!("Metadata failure count limit reached");
		})
	};

	let (tx, workers) = config.spawn_workers(meta);

	let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();

	let (client, mut mqtt_stream) = config.mqtt().await.unwrap();
	let mqtt_handler = task::spawn(async move {
		loop {
			tokio::select! {
				_ = &mut shutdown_rx => break,
				ev = mqtt_stream.poll() => {
					let ev = match ev {
						Ok(v) => v,
						Err(e) => {
							error!(error=?e, "Error receiving from MQTT event loop");
							continue;
						}
					};
					let rumqttc::Event::Incoming(ev) = ev else {
						continue;
					};
					let rumqttc::Packet::Publish(ev) = ev else {
						continue;
					};
					if let Err(e) = tx.send(ev.payload).await {
						error!(error=?e, "Send channel has closed; terminating");
						break;
					}
				}
			}
		}
	});

	let mut sigterm = signal(SignalKind::terminate()).unwrap();
	let mut sigint = signal(SignalKind::interrupt()).unwrap();
	let mut sigquit = signal(SignalKind::quit()).unwrap();
	tokio::select! {
		_ = sigterm.recv() => info!("SIGTERM received; shutting down"),
		_ = sigint.recv() => info!("SIGINT received; shutting down"),
		_ = sigquit.recv() => info!("SIGQUIT received; suhtting down"),
		_ = mqtt_handler => warn!("MQTT event loop handler terminated; shutting down"),
		_ = meta_handle => warn!("Metadata loop terminated; shutting down")
	};

	if let Err(e) = client.unsubscribe(&config.mqtt_topic).await {
		error!(error=?e, "Failed to unsubscribe from MQTT topic");
	}
	shutdown_tx.send(()).unwrap();
	futures::future::join_all(workers).await;
}
