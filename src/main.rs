use std::{fmt::Debug, process::Command, vec};

use chrono::{
    Local, NaiveTime,
    format::{Parsed, StrftimeItems, parse},
};

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
enum Schedule {
    EveryMin {
        every: u16,
        wallpappers: Vec<String>,
    },
    AtTime {
        tw: Vec<TimeWallpapper>,
    },
}

impl Schedule {
    pub fn from_schedule() -> Result<Schedule, chrono::ParseError> {
        let t1 = TimeWallpapper::new("19:00", "/home/me/wallpappers/dark_wall.png")?;
        let t2 = TimeWallpapper::new("6:00", "/home/me/wallpappers/light_wall.png")?;
        let mut wallpappers = vec![t1, t2];

        wallpappers.sort_by_key(|tw| tw.time);

        Ok(Schedule::AtTime { tw: wallpappers })
    }

    pub fn find_the_latest(&self) -> Option<&TimeWallpapper> {
        let now = Local::now().time();

        if let Schedule::AtTime { tw } = &self {
            if let Some(last) = tw.iter().rfind(|x| x.time < now) {
                return Some(last);
            }
        }

        None
    }

    pub fn hyprpapper_set_wallpapper<S: AsRef<str>>(&self, wallpapper: S) {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "hyprctl hyprpaper wallpaper \",{}\"",
                wallpapper.as_ref()
            ))
            .output();

        match output {
            Ok(out) => {
                println!("[SUCCESS] {:?}", out)
            }
            Err(err) => {
                eprintln!("[FAIL] {:?}", err)
            }
        }
    }
}

fn main() {
    let scheduler_r = Schedule::from_schedule();
    let scheduler = match scheduler_r {
        Ok(s) => s,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };
    
    if let Some(last) = scheduler.find_the_latest() {
        println!("{:?}", last);
        scheduler.hyprpapper_set_wallpapper(&last.wallpapper);
    }
}
