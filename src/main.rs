use std::time::Duration;

use chrono::{Local, NaiveTime};
use futures_util::StreamExt;
use gradient::{config::Config, structs::ChangeWallpapper};
use zbus::{Connection, Proxy, names::MemberName, proxy::SignalStream};

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
    let config = match Config::new() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("[ERROR] {:?}", err);
            return;
        }
    };

    let change_wallpapper = match ChangeWallpapper::new_at_time(&config) {
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
