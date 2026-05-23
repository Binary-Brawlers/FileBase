use crate::config::Config;

pub fn connect(config: &Config) -> anyhow::Result<redis::Client> {
    Ok(redis::Client::open(config.redis_url.as_str())?)
}
