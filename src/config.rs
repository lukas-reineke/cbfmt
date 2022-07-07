use config::{Config, ConfigError};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Conf {
    pub languages: HashMap<String, Vec<String>>,
}

pub fn get(name: &str) -> Result<Conf, ConfigError> {
    let settings = Config::builder()
        .add_source(config::File::with_name(name))
        .build()?;

    return settings.try_deserialize::<Conf>();
}
