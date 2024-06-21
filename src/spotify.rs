use librespot::connect::spirc::Spirc;
use librespot::core::cache::Cache;
use librespot::discovery::DeviceType;
use librespot::playback::mixer::softmixer::SoftMixer;
use librespot::playback::mixer::Mixer;
use rand::seq::SliceRandom;
use tokio::join;

use std::env;

use librespot::core::authentication::Credentials;
use librespot::core::config::{ConnectConfig, SessionConfig};
use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::metadata::{Metadata, Playlist, Track};
use librespot::playback::audio_backend;
use librespot::playback::config::{AudioFormat, PlayerConfig};
use librespot::playback::mixer::{MixerConfig, NoOpVolume};
use librespot::playback::player::{Player, PlayerEvent};

use crate::util::get_home_path;

const CREDS_PATH: &str = "creds";
const VOL_PATH: &str = "vol";
const AUDIO_PATH: &str = "audio";
pub async fn play_alarm() {
    let home_path = get_home_path().expect("Unable to get your home directory");
    let cache = Cache::new(
        Some(format!("{}/{}", home_path.to_str().unwrap(), CREDS_PATH)),
        Some(format!("{}/{}", home_path.to_str().unwrap(), VOL_PATH)),
        Some(format!("{}/{}", home_path.to_str().unwrap(), AUDIO_PATH)),
        None,
    )
    .unwrap();

    let credentials = match cache.credentials() {
        Some(c) => {
            println!("using saved credentials");
            c
        }
        None => {
            let args: Vec<_> = env::args().collect();
            if args.len() != 3 {
                eprintln!("Usage: {} USERNAME PASSWORD", args[0]);
                return;
            }
            let cred = Credentials::with_password(&args[1], &args[2]);

            cache.save_credentials(&cred);
            cred
        }
    };

    let mut rng = rand::thread_rng();
    let session_config = SessionConfig::default();
    let player_config = PlayerConfig::default();
    let audio_format = AudioFormat::default();
    let backend = audio_backend::find(None).unwrap();
    let connect_config = ConnectConfig {
        name: "PiAlarm".to_string(),
        device_type: DeviceType::default(),
        initial_volume: Some(100),
        has_volume_ctrl: false,
        autoplay: false,
    };

    println!("Connecting ..");
    let (session, _) = Session::connect(session_config, credentials, None, false)
        .await
        .unwrap();

    let (mut player, mut player_event) = Player::new(
        player_config,
        session.clone(),
        Box::new(NoOpVolume),
        move || backend(None, audio_format),
    );

    // pick a random track from the alarm playlist
    let plist = "spotify:playlist:2aBMj4vGrpxavecIWQtcc4"; // alarm
    let plist_uri = SpotifyId::from_uri(plist).unwrap();

    let plist = Playlist::get(&session, plist_uri).await.unwrap();
    let track = *plist.tracks.choose(&mut rng).unwrap();
    let print_track = Track::get(&session, track).await.unwrap();
    println!("{}", print_track.name);

    // https://open.spotify.com/track/5PbMSJZcNA3p2LZv7C56cm?si=d83209b036a64047
    // let track = SpotifyId::from_base62("5PbMSJZcNA3p2LZv7C56cm").unwrap(); // 4 seconds
    //https://open.spotify.com/track/6UCFZ9ZOFRxK8oak7MdPZu?si=e14c5c002f064429
    // let track = SpotifyId::from_base62("6UCFZ9ZOFRxK8oak7MdPZu").unwrap(); // 20 something seconds
    // let print_track = Track::get(&session, track).await.unwrap();
    // println!(">>{}", print_track.name);

    // play the track
    player.load(track, true, 0);
    let (spirc, spirc_task) = Spirc::new(
        connect_config,
        session.clone(),
        player,
        Box::new(SoftMixer::open(MixerConfig::default())),
    );

    join!(spirc_task, async {
        println!("Playing...");
        spirc.play();

        while let Some(event) = player_event.recv().await {
            match event {
                // end the alarm if the track stops
                // the app will start looking for the next alarm
                PlayerEvent::EndOfTrack { .. }
                | PlayerEvent::Paused { .. }
                | PlayerEvent::Stopped { .. } => spirc.shutdown(),
                _ => {}
            }
        }
        println!("Done...");
    });
}
