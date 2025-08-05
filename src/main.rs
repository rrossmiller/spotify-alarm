use librespot::{
    connect::{ConnectConfig, LoadRequest, LoadRequestOptions, Spirc},
    core::{
        authentication::Credentials, cache::Cache, config::SessionConfig, session::Session, Error,
        SpotifyId,
    },
    metadata::{Metadata, Playlist, Track},
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::{self, MixerConfig},
        player::Player,
    },
};
use log::LevelFilter;
use rand::seq::IteratorRandom;

const CACHE: &str = ".cache";
const CACHE_FILES: &str = ".cache/files";

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut rng = rand::rng();
    env_logger::builder()
        .filter_module("librespot", LevelFilter::Debug)
        .init();

    let session_config = SessionConfig::default();
    let player_config = PlayerConfig::default();
    let audio_format = AudioFormat::default();
    let connect_config = ConnectConfig::default();
    let mixer_config = MixerConfig::default();
    let request_options = LoadRequestOptions::default();

    let sink_builder = audio_backend::find(None).unwrap();
    let mixer_builder = mixer::find(None).unwrap();

    let cache = Cache::new(Some(CACHE), Some(CACHE), Some(CACHE_FILES), None)?;
    let credentials = cache
        .credentials()
        .ok_or(Error::unavailable("credentials not cached"))
        .or_else(|_| {
            librespot::oauth::OAuthClientBuilder::new(
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

    let (spirc, spirc_task) =
        Spirc::new(connect_config, session.clone(), credentials, player, mixer).await?;

    // get playlist
    let plist_uri = SpotifyId::from_uri("spotify:playlist:2aBMj4vGrpxavecIWQtcc4").unwrap();
    let plist = Playlist::get(&session, &plist_uri).await.unwrap();
    println!("{:?}", plist);
    for track_id in plist.tracks() {
        let plist_track = Track::get(&session, track_id).await.unwrap();
        println!("track: {} ", plist_track.name);
    }

    // these calls can be seen as "queued"
    spirc.activate()?;

    // spirc.load(LoadRequest::from_tracks(
    //     plist.tracks().map(|e| e.to_uri().unwrap()).collect(),
    //     request_options,
    // ));

    // spirc.load(LoadRequest::from_context_uri(
    //     format!("spotify:user:{}:collection", session.username()),
    //     request_options,
    // ))?;
    let track = *plist.tracks().choose(&mut rng).unwrap();
    spirc
        .load(LoadRequest::from_context_uri(
            track.to_uri().unwrap(),
            request_options,
        ))
        .unwrap();
    spirc.play()?;

    // starting the connect device and processing the previously "queued" calls
    spirc_task.await;

    Ok(())
}
