use tracing_subscriber::EnvFilter;

pub fn init(service: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("{service}=info,tower_http=info")));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
