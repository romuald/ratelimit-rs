use std::{env, fs, io::Error};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct RLConfig {
    pub hits: u32,
    pub seconds: f64,

    pub cleanup_interval: u32,
}

#[derive(Deserialize, Debug)]
pub struct MCacheConfig {
    pub enabled: bool,
    pub listen: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct HandlersConfig {
    pub memcache: MCacheConfig,
}

#[derive(Deserialize, Debug)]
pub struct Configuration {
   pub ratelimit: RLConfig,
   pub handlers: HandlersConfig,
}


impl Configuration {
    pub fn from_argv()  -> Result<Configuration, Error> {
        let args: Vec<String> = env::args().collect();
        let filename = match args.get(1) {
            Some(name) => name,
            None => "development.toml",
        };

        let conf = fs::read_to_string(filename)?;
        let res = toml::from_str(&conf)?;
        Ok(res)
    }
}
