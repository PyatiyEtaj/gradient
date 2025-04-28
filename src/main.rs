use std::{fmt::Debug, process::Command, time::Duration, vec};

use chrono::{
    Local, NaiveTime,
    format::{Parsed, StrftimeItems, parse},
};
use futures_util::StreamExt;
use zbus::{Connection, Proxy, names::MemberName, proxy::SignalStream};

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
enum ChangeWallpapper {
    EveryMin {
        every: u16,
        wallpappers: Vec<String>,
    },
    AtTime {
        tw: Vec<TimeWallpapper>,
    },
}

impl ChangeWallpapper {
    pub fn new_at_time() -> Result<ChangeWallpapper, chrono::ParseError> {
        let t2 = TimeWallpapper::new("08:00", "/home/me/wallpappers/light_wall.png")?;
        let t1 = TimeWallpapper::new("17:00", "/home/me/wallpappers/dark_wall.png")?;
        let mut wallpappers = vec![t1, t2];

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
                println!("[INFO] {:?}", out)
            }
            Err(err) => {
                eprintln!("[ERROR] {:?}", err)
            }
        }
    }
}

async fn init_sleep_signal_stream() -> Result<SignalStream<'static>, zbus::Error> {
    let connection = Connection::system().await?;

    let p = Proxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await?;

    let stream = p.receive_signal("PrepareForSleep").await?;

    Ok(stream)
}

async fn init_and_run_change_wallpapper_loop() {
    let change_wallpapper = match ChangeWallpapper::new_at_time() {
        Ok(cw) => cw,
        Err(err) => {
            eprintln!("[ERROR] {}", err);
            return;
        }
    };

    loop {
        let mut sleep_ms = 1000 * 60; // default 1 min
        let now = Local::now().time();

        if let Some((tw, sleep_for_next)) = change_wallpapper.wallpapper(now) {
            println!("[INFO] current_wallpapper = {:?}", tw);
            sleep_ms = sleep_for_next;
            change_wallpapper.hyprpapper_set_wallpapper(&tw.wallpapper);
        }

        println!(
            "[INFO] main loop sleep for {:?}",
            NaiveTime::from_num_seconds_from_midnight_opt(sleep_ms / 1000, 0).unwrap()
        );

        tokio::time::sleep(Duration::from_millis(sleep_ms as u64)).await;
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> zbus::Result<()> {
    tokio::time::sleep(Duration::from_secs(2)).await;

    let mut main_loop = tokio::spawn(async {
        init_and_run_change_wallpapper_loop().await;
    });

    let mut stream = init_sleep_signal_stream().await?;

    let unknown = MemberName::from_static_str("unknown")?;

    while let Some(signal) = stream.next().await {
        let body: (bool,) = signal.body().deserialize()?;

        println!(
            "[INFO] get signal name:'{:?}' body:{}",
            signal.header().member().unwrap_or(&unknown).to_string(),
            body.0
        );

        if body.0 {
            if !main_loop.is_finished() {
                main_loop.abort();
                println!("[INFO] main loop aborated");
            }
        } else {
            if !main_loop.is_finished() {
                main_loop.abort();
                println!("[INFO] main loop aborated");
            }

            main_loop = tokio::spawn(async {
                init_and_run_change_wallpapper_loop().await;
            });

            println!("[INFO] main loop initialized again");
        }
    }

    Ok(())
}
