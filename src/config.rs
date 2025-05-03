use std::{fs::File, io::Read, path::Path};

#[derive(Debug)]
pub enum ConfigError {
    CantGetHomeDir,
    CantOpenConfig(String),
    CantRead(String),
    CantParse(String),
}

#[derive(serde::Deserialize, Debug)]
pub struct TimeWalpapperConfig {
    time: String,
    wallpapper: String,
}

impl TimeWalpapperConfig {
    pub fn time(&self) -> &str{
        self.time.as_str()
    }

    pub fn wallpapper(&self) -> &str{
        self.wallpapper.as_str()
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct Config {
    plan: Vec<TimeWalpapperConfig>,
}

impl Config {
    pub fn new() -> Result<Config, ConfigError> {
        let home = match std::env::home_dir(){
            Some(h) => h,
            None => {
                return Err(ConfigError::CantGetHomeDir);
            },
        };
        let path = Path::new(&home).join(".config/hypr/gradient.json");
        println!("[INFO] cfg from {:?}", path);
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(err) => return Err(ConfigError::CantOpenConfig(err.to_string())),
        };

        let mut json = String::new();
        match file.read_to_string(&mut json) {
            Ok(_) => {}
            Err(err) => return Err(ConfigError::CantRead(err.to_string())),
        };

        let cfg: Config = match serde_json::from_str(&json) {
            Ok(r) => r,
            Err(err) => return Err(ConfigError::CantParse(err.to_string())),
        };

        Ok(cfg)
    }

    pub fn plan(&self) -> &[TimeWalpapperConfig]{
        self.plan.as_slice()
    }
}

#[cfg(test)]
mod config_tests {
    use super::Config;

    #[test]
    fn json_parse() {
        let cfg = match Config::new() {
            Ok(cfg) => cfg,
            Err(err) => {
                assert_eq!(format!("[ERROR] {:?}", err), "");
                return;
            },
        };
        println!("{:?}", cfg);
    }
}