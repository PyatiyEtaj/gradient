use std::process::Command;

use chrono::{
    NaiveTime,
    format::{Parsed, StrftimeItems, parse},
};

use crate::config::Config;

#[derive(Debug)]
pub struct TimeWallpapper {
    pub time: NaiveTime,
    pub wallpapper: String,
}

impl TimeWallpapper {
    pub fn new<S: AsRef<str>>(
        time: S,
        wallpapper: S,
    ) -> Result<TimeWallpapper, chrono::ParseError> {
        let time = Self::parsing_date(time)?;
        Ok(TimeWallpapper {
            time,
            wallpapper: wallpapper.as_ref().to_string(),
        })
    }

    fn parsing_date<S: AsRef<str>>(date: S) -> std::result::Result<NaiveTime, chrono::ParseError> {
        let fmt = StrftimeItems::new("%H:%M").parse()?;

        let mut parsed = Parsed::new();
        parse(&mut parsed, date.as_ref(), fmt.as_slice().iter())?;
        let parsed_dt = parsed.to_naive_time()?;

        Ok(parsed_dt)
    }
}

#[derive(Debug)]
pub enum ChangeWallpapper {
    EveryMin {
        every: u16,
        wallpappers: Vec<String>,
    },
    AtTime {
        tw: Vec<TimeWallpapper>,
    },
}

impl ChangeWallpapper {
    pub fn new_at_time(config: &Config) -> Result<ChangeWallpapper, chrono::ParseError> {
        let mut wallpappers: Vec<TimeWallpapper> = vec![];

        for cfg in &config.plan {
            let tw = TimeWallpapper::new(&cfg.time, &cfg.wallpapper)?;
            wallpappers.push(tw);
        }

        wallpappers.sort_by_key(|tw| tw.time);

        Ok(ChangeWallpapper::AtTime { tw: wallpappers })
    }

    pub fn wallpapper(&self, from_time: NaiveTime) -> Option<(&TimeWallpapper, u32)> {
        if let ChangeWallpapper::AtTime { tw } = &self {
            if let Some(current) = tw.iter().rfind(|x| x.time < from_time).or(tw.last()) {
                if let Some(next) = tw.iter().find(|x| x.time > from_time).or(tw.first()) {
                    let wait_to_next = if from_time < next.time {
                        next.time
                            .signed_duration_since(from_time)
                            .num_milliseconds()
                    } else {
                        24 * 60 * 60 * 1000
                            - from_time
                                .signed_duration_since(next.time)
                                .num_milliseconds()
                    };

                    return Some((current, wait_to_next as u32));
                }
            }
        }

        None
    }

    pub fn hyprpapper_set_wallpapper<S: AsRef<str>>(&self, wallpapper: S) -> bool {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "hyprctl hyprpaper wallpaper \",{}\"",
                wallpapper.as_ref()
            ))
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    println!("[INFO] {:?}", out);
                    return true;
                }
                eprintln!("[ERROR] {:?}", out);
                false
            }
            Err(err) => {
                eprintln!("[ERROR] {:?}", err);
                false
            }
        }
    }
}
