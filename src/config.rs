use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Conf {
    pub languages: HashMap<String, Vec<String>>,
}

pub fn get(name: &str) -> Result<Conf, std::io::Error> {
    let toml_string = std::fs::read_to_string(name)?;
    let conf: Conf = toml::from_str(&toml_string)?;
    Ok(conf)
}
