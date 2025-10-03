use anyhow::{Context, Result};
use web_time::Duration;

fn builder() -> reqwest::ClientBuilder {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            reqwest::Client::builder()
        } else {
            let danger_accept_invalid_certs = matches!(std::env::var("DANGER_ACCEPT_INVALID_CERTS").as_deref(), Ok("1"));
            reqwest::Client::builder()
                // NB: we can NOT set a total request timeout here: our users
                // regularly have requests that take multiple minutes, due to how
                // long LLMs take
                .connect_timeout(Duration::from_secs(10))
                .danger_accept_invalid_certs(danger_accept_invalid_certs)
                .http2_keep_alive_interval(Some(Duration::from_secs(10)))
                // To prevent stalling in python, we set the pool to 0 and idle timeout to 0.
                // See:
                // https://github.com/seanmonstar/reqwest/issues/600
                // https://github.com/denoland/deno/issues/28853
                // https://github.com/hyperium/hyper/issues/2312
                // https://github.com/Azure/azure-sdk-for-rust/pull/1550
                .pool_max_idle_per_host(0)
                .pool_idle_timeout(std::time::Duration::from_nanos(1))
        }
    }
}

fn builder_with_env(env: &std::collections::HashMap<String, String>) -> reqwest::ClientBuilder {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            // On wasm, TLS verification cannot be disabled via reqwest; ignore this setting.
            let _ = env.get("DANGER_ACCEPT_INVALID_CERTS");
            reqwest::Client::builder()
        } else {
            let danger_accept_invalid_certs = env.get("DANGER_ACCEPT_INVALID_CERTS").map(|v| v.as_str()) == Some("1");
            reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .danger_accept_invalid_certs(danger_accept_invalid_certs)
                .http2_keep_alive_interval(Some(Duration::from_secs(10)))
                .pool_max_idle_per_host(0)
                .pool_idle_timeout(std::time::Duration::from_nanos(1))
        }
    }
}

pub fn create_client() -> Result<reqwest::Client> {
    builder().build().context("Failed to create reqwest client")
}

pub fn create_client_with_env(
    env: &std::collections::HashMap<String, String>,
) -> Result<reqwest::Client> {
    builder_with_env(env)
        .build()
        .context("Failed to create reqwest client with env")
}

pub(crate) fn create_tracing_client() -> Result<reqwest::Client> {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let cb = builder();
        } else {
            let cb = builder()
                // Wait up to 30s to send traces to the backend
                .read_timeout(Duration::from_secs(30));

        }
    }

    cb.build().context("Failed to create reqwest client")
}
