use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::TelemetryConfig;

pub fn init_telemetry(config: TelemetryConfig) -> Result<(), String> {
    let fmt_layer = tracing_subscriber::fmt::layer().with_filter(
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
    );

    tracing_subscriber::registry().with(fmt_layer).init();

    Ok(())
}
