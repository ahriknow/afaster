use afast::{AFastSerialize, Tag, handler};

#[derive(AFastSerialize, Tag)]
#[tag("Service health status")]
struct HealthResponse {
    status: String,
    version: String,
}

#[handler(desc("Health check"), no_trace)]
async fn health() -> afast::Result<HealthResponse> {
    #[cfg(feature = "log")]
    tracing::info!("Health check called");
    Ok(HealthResponse {
        status: "ok".into(),
        version: "0.0.2".into(),
    })
}
