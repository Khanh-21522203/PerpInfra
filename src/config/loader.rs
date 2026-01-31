use crate::config::fees::FeeConfig;
use crate::config::market::MarketConfig;
use crate::config::risk::RiskConfig;
use crate::config::*;
use crate::error::{Error, Result};
use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub market: MarketConfig,
    pub risk: RiskConfig,
    pub fees: FeeConfig,
    pub funding: FundingConfig,
    pub kafka: KafkaConfig,
    pub price_sources: Vec<crate::price_infra::PriceSourceConfig>,
}

#[derive(Debug, Deserialize)]
pub struct KafkaConfig {
    pub brokers: String,
    pub topic: String,
    pub group_id: String,
}

impl AppConfig {
    pub fn load(env: &str) -> Result<Self> {
        let config = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", env)).required(false))
            .add_source(Environment::with_prefix("PERPINFRA"))
            .build()
            .map_err(|e| Error::ConfigError(e.to_string()))?;

        config.try_deserialize()
            .map_err(|e| Error::ConfigError(e.to_string()))
    }
}