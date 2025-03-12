use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use actix_files;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::BufReader;
use anyhow::{Context, Result};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Word {
    word: String,
    meaning: String,
}

#[derive(Debug)]
struct AppState {
    words: Vec<Word>,
}

impl AppState {
    fn new() -> Result<Self> {
        let words: Vec<Word> = serde_json::from_reader(
            BufReader::new(File::open("static/words.json").context("Failed to open words.json file")?))
        .context("Failed to parse words.json")?;
        
        Ok(AppState { words })
    }

}

#[get("/api/words")]
async fn get_words(data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(&data.words)
}

#[actix_web::main]
async fn main() -> Result<()> {
    let app_state = web::Data::new(AppState::new()?);
    
    println!("Server running at http://localhost:8080");    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(get_words)
            .service(actix_files::Files::new("/", "./static").index_file("index.html"))
    })
    .bind("127.0.0.1:8080").context("Failed to bind to address")?
    .run()
    .await.context("Server error")
}
