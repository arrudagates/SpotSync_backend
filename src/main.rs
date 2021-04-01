use tide::prelude::*;
use tide::Request;

extern crate rspotify;
use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::util::*;

use dotenv::dotenv;
use std::env;
use serde_json::{Result, Value};

#[async_std::main]
async fn main() -> tide::Result<()> {

    let mut app = tide::new();
    app.at("/get_url").get(get_url);
    app.at("/auth").post(auth);
    app.at("/me").get(me);
    app.at("/current_playing").get(current_playing);
    app.at("/start_playback").post(start_playback);
    app.listen("127.0.0.1:8000").await?;
    Ok(())
}

#[derive(Deserialize)]
struct Playback {
    uri: String,
    position_ms: Option<u32>
}

async fn get_oauth() -> SpotifyOAuth {
   dotenv().ok();

    let oauth = SpotifyOAuth::default()
        .client_id(env::var("ID").expect("not found").as_str())
        .client_secret(env::var("SECRET").expect("not found").as_str())
        .redirect_uri("http://localhost:8000/redirect")
        .scope("app-remote-control")
        .scope("streaming")
        .scope("user-read-playback-state")
        .scope("user-modify-playback-state")
        .scope("user-read-currently-playing")
        .build();
    oauth
}

async fn get_url(mut req: Request<()>) -> tide::Result {
    Ok(json!({"status": "ok", "url": SpotifyOAuth::get_authorize_url(&mut get_oauth().await, None, None)}).into())
}

#[derive(Deserialize)]
struct Url {
    url: Option<String>
}

async fn auth(mut req: Request<()>) -> tide::Result {
    let Url { url } = req.body_json().await.unwrap();

    let mut new_url = url.clone().unwrap();

    let token = match SpotifyOAuth::parse_response_code(&mut get_oauth().await, &mut new_url) {
        Some(c) => {
            match SpotifyOAuth::get_access_token_without_cache(&mut get_oauth().await, c.as_str()).await {
                Some(t) => Some(t.access_token),
                None => None
            }
        }
        None => None
    };

      if let Some(token) = token {
          Ok(json!({"status": "ok", "token": token}).into())

   }
   else {
       Ok(json!({"status": "err", "error": "Failed to authenticate"}).into())
   }
}

async fn me(mut req: Request<()>) -> tide::Result {
    let token = req.header("Authorization").unwrap();
    Ok(format!("{:#?}", Spotify::default().access_token(token.as_str()).me().await.unwrap()).into())
}

async fn current_playing(mut req: Request<()>) -> tide::Result {
    let token = req.header("Authorization").unwrap();
    let playing = Spotify::default().access_token(token.as_str()).current_playing(None, None).await;
    match playing {
        Ok(data) => {
            match data {
                Some(d) => {
                    let uri;
                    let ctx = d.context;
                    if let Some(con) = ctx {
                        uri = con.uri;
                    }
                    else { uri = String::from("none") }
                    let response = json!({
                        "uri": uri.as_str(),
                        "timestamp": d.timestamp,
                        "progress_ms": d.progress_ms.unwrap_or(0),
                        "is_playing": d.is_playing,

                    });
                   Ok(response.into())
                },
                None => Ok("Nothing playing".into())
            }
        },
        Err(e) => Ok(json!({"status": "err", "error": "Failed to get current_playing"}).into())
    }
}

async fn start_playback(mut req: Request<()>) -> tide::Result {
    let token = req.header("Authorization").unwrap();
    let spotify = Spotify::default().access_token(token.as_str());
    let device: Option<rspotify::model::device::Device> = match spotify.current_playback(None, None).await {
        Ok(data) => {
            match data {
                Some(d) => Some(d.device),
                None => None
            }
        },
        Err(e) => None
    };
    if let Some(device) = device {
        let Playback { uri, position_ms } = req.body_json().await?;
       match spotify.start_playback(Some(device.id), None, Some(vec![uri]), None, position_ms).await {
           Ok(data) => Ok(json!({"status": "ok"}).into()),
           Err(e) => Ok(json!({"status": "err", "error": "Failed to start_playback"}).into())
       }
       
    }
    else {
        Ok(json!({"status": "err", "error": "No device active"}).into())
    }
}
