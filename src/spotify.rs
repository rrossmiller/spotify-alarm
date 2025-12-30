use std::future::Future;
use std::sync::Arc;
use tokio::{join, sync::Mutex};

use librespot::{
    connect::{ConnectConfig, LoadRequest, LoadRequestOptions, Spirc},
    core::{
        authentication::Credentials, cache::Cache, config::SessionConfig, session::Session, Error,
        SpotifyUri,
    },
    metadata::{Metadata, Playlist},
    oauth,
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::{self, MixerConfig},
        player::{Player, PlayerEvent},
    },
};
use rand::seq::IteratorRandom;

const CACHE: &str = ".cache";
const CACHE_FILES: &str = ".cache/files";
pub async fn init() -> Result<
    (
        Session,
        Arc<Mutex<Spirc>>,
        impl Future<Output = ()>,
        Arc<Player>,
    ),
    Error,
> {
    let session_config = SessionConfig::default();
    let player_config = PlayerConfig::default();
    let audio_format = AudioFormat::default();
    let connect_config = ConnectConfig::default();
    let mixer_config = MixerConfig::default();

    let sink_builder = audio_backend::find(None).unwrap();
    let mixer_builder = mixer::find(None).unwrap();

    let cache = Cache::new(Some(CACHE), Some(CACHE), Some(CACHE_FILES), None)?;
    let credentials = cache
        .credentials()
        .ok_or(Error::unavailable("credentials not cached"))
        .or_else(|_| {
            oauth::OAuthClientBuilder::new(
                &session_config.client_id,
                "http://127.0.0.1:8898/login",
                vec!["streaming"],
            )
            .open_in_browser()
            .build()?
            .get_access_token()
            .map(|t| Credentials::with_access_token(t.access_token))
        })?;

    let session = Session::new(session_config, Some(cache));
    let mixer = mixer_builder(mixer_config)?;

    let player = Player::new(
        player_config,
        session.clone(),
        mixer.get_soft_volume(),
        move || sink_builder(None, audio_format),
    );

    let (spirc, spirc_task) = Spirc::new(
        connect_config,
        session.clone(),
        credentials,
        player.clone(),
        mixer,
    )
    .await?;

    return Ok((session, Arc::new(Mutex::new(spirc)), spirc_task, player));
}

pub async fn play() -> Result<(), Error> {
    let (session, spirc, spirc_task, player) = init().await?;

    let request_options = LoadRequestOptions::default();

    // get playlist
    // let uri = "13NGKvpadSMzN73aFnFFKT"; // 150 playlist
    let uri = "2aBMj4vGrpxavecIWQtcc4"; // alarm playlist
    let plist_uri = SpotifyUri::from_uri(&format!("spotify:playlist:{}", uri)).unwrap();
    let plist = Playlist::get(&session, &plist_uri).await.unwrap();

    // Choose a random track and get its URI (ThreadRng is not Send, so we need to drop it before awaits)
    let track_uri = {
        let mut rng = rand::rng();
        let track = plist.tracks().choose(&mut rng).unwrap();
        track.to_uri().unwrap()
    }; // RNG is dropped here

    // Lock spirc for playback control
    let spirc_guard = spirc.lock().await;

    // these calls can be seen as "queued"
    spirc_guard.activate()?;

    // set volume to max
    spirc_guard.set_volume(u16::MAX).unwrap();

    spirc_guard.load(LoadRequest::from_context_uri(track_uri, request_options))?;
    spirc_guard.play()?;

    // Release the lock immediately after issuing commands
    drop(spirc_guard);

    join!(
        // play the song
        spirc_task,
        // disconnect/return when the connect device changes
        async {
            let mut events = player.get_player_event_channel();
            while let Some(event) = events.recv().await {
                println!("EVENT: {:?}", event);
                match event {
                    // end the alarm if the track stops
                    // the app will start looking for the next alarm
                    PlayerEvent::EndOfTrack { .. }
                    | PlayerEvent::Paused { .. }
                    | PlayerEvent::Stopped { .. } => {
                        spirc.lock().await.shutdown().unwrap();
                        break;
                    }
                    _ => {}
                }
            }
            println!("Alarm Done...");
        }
    );

    Ok(())
}
