use crate::util::get_home_path;
use chrono::prelude::*;
use std::{fs, process::exit};
use tokio::time::{sleep, Duration};
mod alarm;
mod spotify;
mod util;

const ALARMS_FILE_NAME: &str = "alarms.txt";

#[tokio::main]
async fn main() {
    let mut alarms_file = get_home_path().unwrap();
    alarms_file.push(ALARMS_FILE_NAME);

    // check that the file exists
    if let Err(_) = fs::metadata(&alarms_file) {
        eprintln!(
            "\n{} didn't exist. Please populate it.\nEx: Time Days Desc\n6:00 M,T,W,Th,F,S,Su My first alarm",
            alarms_file.to_str().unwrap()
        );

        fs::write(alarms_file, "").expect("Unable to write file");
        exit(1);
    }

    let mut first = true;
    let mut alarms = vec![];
    loop {
        // get the alarms from the file
        let f = fs::read_to_string(&alarms_file).expect(
            format!(
                "There was a problem reading {}",
                alarms_file.to_str().unwrap()
            )
            .as_str(),
        );
        let my_alarms = alarm::get_alarms(f.as_str()).unwrap();

        // figure out which alarm should be next -- specifically which alarms should run today and
        // which alarms have already run (merge current state with new state)
        let time = Local::now();
        alarms = alarm::get_valid_alarms(my_alarms, alarms, time);
        if first {
            first = false;
            for a in alarms.iter() {
                println!("{:?}", a);
            }
            println!();
        }

        // check if any alarms need to be playing
        for a in alarms.iter_mut() {
            if a.should_play(time) {
                println!("> {:?}", a);
                println!("@ {:?}", time);
                a.played = true;
                spotify::play_alarm().await;
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
}
