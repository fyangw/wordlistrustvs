use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use actix_files as fs;
use serde::{Serialize, Deserialize};
use std::sync::Mutex;
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
    current_word_index: Mutex<usize>,
    words: Vec<Word>,
}

impl AppState {
    fn new() -> Result<Self> {
        let words: Vec<Word> = serde_json::from_reader(
            BufReader::new(File::open("static/words.json").context("Failed to open words.json file")?)
        ).context("Failed to parse words.json")?;
        
        if words.is_empty() {
            anyhow::bail!("Words list is empty");
        }
        
        Ok(AppState {
            current_word_index: Mutex::new(0),
            words,
        })
    }

    // 添加用于测试的函数
    #[cfg(test)]
    fn new_with_words(words: Vec<Word>) -> Self {
        AppState {
            current_word_index: Mutex::new(0),
            words,
        }
    }

    fn current_word(&self) -> Result<Word> {
        let current_index = self.current_word_index
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire lock"))?;
        
        self.words.get(*current_index)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Word index out of bounds"))
    }

    fn next_word(&self) -> Result<()> {
        let mut current_index = self.current_word_index
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire lock"))?;
        
        *current_index = (*current_index + 1) % self.words.len();
        Ok(())
    }
}

#[get("/api/word")]
async fn get_word(data: web::Data<AppState>) -> impl Responder {
    data.current_word()
        .map(|word| HttpResponse::Ok().json(word))
        .unwrap_or_else(|_| HttpResponse::InternalServerError().json("Failed to get current word"))
}

#[post("/api/next")]
async fn next_word(data: web::Data<AppState>) -> impl Responder {
    data.next_word()
        .map(|_| HttpResponse::Ok().finish())
        .unwrap_or_else(|_| HttpResponse::InternalServerError().json("Failed to move to next word"))
}

#[actix_web::main]
async fn main() -> Result<()> {
    let app_state = web::Data::new(
        AppState::new()
            .context("Failed to initialize application state")?
    );
    
    println!("Server running at http://localhost:8080");
    println!("Loaded {} words from words.json", app_state.words.len());
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(get_word)
            .service(next_word)
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    .bind("127.0.0.1:8080").context("Failed to bind to address")?
    .run()
    .await.context("Server error")
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    fn test_words() -> Vec<Word> {
        vec![
            Word {
                word: String::from("test"),
                meaning: String::from("测试"),
            },
            Word {
                word: String::from("example"),
                meaning: String::from("例子"),
            },
        ]
    }

    #[tokio::test]
    async fn test_app_state_new_with_words() {
        let words = test_words();
        let app_state = AppState::new_with_words(words.clone());
        
        assert_eq!(app_state.words, words);
        assert_eq!(*app_state.current_word_index.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_app_state_current_word() {
        let words = test_words();
        let app_state = AppState::new_with_words(words.clone());
        
        let current_word = app_state.current_word().unwrap();
        assert_eq!(current_word, words[0]);
    }

    #[tokio::test]
    async fn test_app_state_next_word() {
        let words = test_words();
        let app_state = AppState::new_with_words(words.clone());
        
        // 测试初始状态
        assert_eq!(app_state.current_word().unwrap(), words[0]);
        
        // 测试移动到下一个单词
        app_state.next_word().unwrap();
        assert_eq!(app_state.current_word().unwrap(), words[1]);
        
        // 测试循环回到开始
        app_state.next_word().unwrap();
        assert_eq!(app_state.current_word().unwrap(), words[0]);
    }

    #[actix_web::test]
    async fn test_get_word_endpoint() {
        let words = vec![test_words()[0].clone()];
        let app_state = web::Data::new(AppState::new_with_words(words.clone()));
        
        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(get_word)
        ).await;

        let req = test::TestRequest::get().uri("/api/word").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert!(resp.status().is_success());
        
        let result: Word = test::read_body_json(resp).await;
        assert_eq!(result, words[0]);
    }

    #[actix_web::test]
    async fn test_next_word_endpoint() {
        let words = test_words();
        let app_state = web::Data::new(AppState::new_with_words(words.clone()));
        
        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .service(get_word)
                .service(next_word)
        ).await;

        // 测试初始状态
        let req = test::TestRequest::get().uri("/api/word").to_request();
        let resp = test::call_service(&app, req).await;
        let result: Word = test::read_body_json(resp).await;
        assert_eq!(result, words[0]);

        // 测试移动到下一个单词
        let req = test::TestRequest::post().uri("/api/next").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // 验证是否移动到了下一个单词
        let req = test::TestRequest::get().uri("/api/word").to_request();
        let resp = test::call_service(&app, req).await;
        let result: Word = test::read_body_json(resp).await;
        assert_eq!(result, words[1]);
    }
}