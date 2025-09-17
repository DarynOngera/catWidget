#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui::{FontDefinitions, FontFamily};

// use futures::StreamExt; // Removed - not needed after simplifying stream handling
use rand::seq::SliceRandom;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;

fn decode_animated_gif(gif_data: &[u8]) -> Result<Vec<(egui::ColorImage, Duration)>, Box<dyn std::error::Error>> {
    use std::io::Cursor;
    
    let cursor = Cursor::new(gif_data);
    let decoder = GifDecoder::new(cursor)?;
    let frames = decoder.into_frames();
    
    let mut gif_frames = Vec::new();
    
    for frame_result in frames {
        let frame = frame_result?;
        let (delay_num, delay_den) = frame.delay().numer_denom_ms();
        let delay_ms = if delay_den == 0 { 100 } else { delay_num / delay_den };
        let duration = Duration::from_millis(delay_ms.max(50) as u64); // Minimum 50ms per frame
        
        let buffer = frame.buffer();
        let (width, height) = buffer.dimensions();
        
        // Convert to RGBA
        let rgba_data: Vec<u8> = buffer
            .pixels()
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect();
        
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [width as usize, height as usize],
            &rgba_data,
        );
        
        gif_frames.push((color_image, duration));
    }
    
    if gif_frames.is_empty() {
        return Err("No frames found in GIF".into());
    }
    
    Ok(gif_frames)
}

mod optimizations;
use optimizations::*;

// New structs for GIF support
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CatImage {
    id: String,
    url: String,
    width: Option<u32>,
    height: Option<u32>,
    #[serde(default)]
    breeds: Vec<Breed>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GiphyResponse {
    data: Vec<GiphyGif>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GiphyGif {
    id: String,
    title: String,
    images: GiphyImages,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GiphyImages {
    fixed_height: GiphyImageData,
    original: GiphyImageData,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GiphyImageData {
    url: String,
    width: String,
    height: String,
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Breed {
    #[serde(default)]
    id: Option<String>,
    name: String,
    #[serde(default)]
    temperament: Option<String>,
    #[serde(default)]
    life_span: Option<String>,
    #[serde(default)]
    alt_names: Option<String>,
    #[serde(default)]
    wikipedia_url: Option<String>,
    #[serde(default)]
    origin: Option<String>,
    #[serde(default)]
    weight_imperial: Option<String>,
    #[serde(default)]
    experimental: Option<u8>,
    #[serde(default)]
    hairless: Option<u8>,
    #[serde(default)]
    natural: Option<u8>,
    #[serde(default)]
    rare: Option<u8>,
    #[serde(default)]
    rex: Option<u8>,
    #[serde(default)]
    suppressed_tail: Option<u8>,
    #[serde(default)]
    short_legs: Option<u8>,
    #[serde(default)]
    hypoallergenic: Option<u8>,
    #[serde(default)]
    adaptability: Option<u8>,
    #[serde(default)]
    affection_level: Option<u8>,
    #[serde(default)]
    country_code: Option<String>,
    #[serde(default)]
    child_friendly: Option<u8>,
    #[serde(default)]
    dog_friendly: Option<u8>,
    #[serde(default)]
    energy_level: Option<u8>,
    #[serde(default)]
    grooming: Option<u8>,
    #[serde(default)]
    health_issues: Option<u8>,
    #[serde(default)]
    intelligence: Option<u8>,
    #[serde(default)]
    shedding_level: Option<u8>,
    #[serde(default)]
    social_needs: Option<u8>,
    #[serde(default)]
    stranger_friendly: Option<u8>,
    #[serde(default)]
    vocalisation: Option<u8>,
    #[serde(default)]
    bidirectional: Option<u8>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Clone)]
enum AppMessage {
    CatDataLoaded { image_id: Option<String>, image_url: String, image_data: Vec<u8>, breed_info: Option<String> },
    LoadError(String),
    CacheStatsUpdated(u64, u64, u64, u64), // hits, misses, disk_hits, network_requests
    ConnectionSpeedChanged(String),
    PreloadedCat { image_id: Option<String>, image_url: String, image_data: Vec<u8>, breed_info: Option<String> },
    GifDataLoaded { gif_id: String, gif_url: String, gif_data: Vec<u8>, gif_title: String },
    GifLoadError(String),
    PreloadingComplete,
}

struct HeartAnimation {
    start_time: Instant,
    show_heart_animation: bool,
}

impl HeartAnimation {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            show_heart_animation: false,
        }
    }

    fn trigger(&mut self) {
        self.show_heart_animation = true;
        self.start_time = Instant::now();
    }

    fn is_active(&self) -> bool {
        self.start_time.elapsed() < Duration::from_millis(1000)
    }
    
    fn progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed().as_millis() as f32;
        let duration = 1000.0; // 1 second animation
        (elapsed / duration).min(1.0)
    }
}

struct CatLoveApp {
    // Optimized HTTP client and caching
    optimized_client: Arc<OptimizedHttpClient>,
    image_cache: Arc<ImageCache>,
    prefetch_manager: Arc<PrefetchManager>,
    batch_manager: Arc<BatchOperationManager>,
    
    // Original fields
    cat_image_url: String,
    current_image_id: Option<String>,
    current_message: String,
    current_fact: String,
    current_breed_info: Option<String>,
    current_love_note: String,
    texture: Option<egui::TextureHandle>,
    loading: bool,
    error_message: Option<String>,
    last_update: Instant,
    auto_refresh_interval: Duration,
    heart_animation: HeartAnimation,
    button_clicks: u32,
    easter_egg_shown: bool,
    rt: Arc<tokio::runtime::Runtime>,
    message_receiver: Arc<Mutex<mpsc::UnboundedReceiver<AppMessage>>>,
    message_sender: mpsc::UnboundedSender<AppMessage>,
    
    // Performance monitoring
    cache_stats: (u64, u64, u64, u64),
    connection_speed: String,
    api_key: String,
    
    // Preloading queue
    preloaded_cats: std::collections::VecDeque<(String, Option<String>, String, Vec<u8>, Option<String>)>,
    preloading: bool,
    
    // GIF support
    show_gifs: bool,
    current_gif_url: String,
    current_gif_title: String,
    gif_texture: Option<egui::TextureHandle>,
    giphy_api_key: String,
    
    // Animated GIF support
    gif_frames: Vec<(egui::ColorImage, Duration)>,
    gif_current_frame: usize,
    gif_last_frame_time: Instant,
    gif_playing: bool,
}

impl CatLoveApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Setup custom fonts
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "SueEllenFrancisco-Regular".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/SueEllenFrancisco-Regular.ttf")),
        );
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "SueEllenFrancisco-Regular".to_owned());
        cc.egui_ctx.set_fonts(fonts);
        
        let rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime")
        );
        
        // Initialize optimized components
        let optimized_client = Arc::new(OptimizedHttpClient::new().expect("Failed to create optimized client"));
        let image_cache = Arc::new(ImageCache::new().expect("Failed to create image cache"));
        let prefetch_manager = Arc::new(PrefetchManager::new());
        let batch_manager = Arc::new(BatchOperationManager::new());
        
        let (sender, receiver) = mpsc::unbounded_channel();
        let api_key = "live_2k3LOZrZ7VN7DbYU6NHQJeufnNWxXdJ7s2o3FJVW7CmVAL0wC8VXffZlcHhYze6j".to_string();
        
        let mut app = Self {
            optimized_client,
            image_cache,
            prefetch_manager,
            batch_manager,
            cat_image_url: String::new(),
            current_image_id: None,
            current_message: String::new(),
            current_fact: String::new(),
            current_breed_info: None,
            current_love_note: String::new(),
            texture: None,
            loading: false,
            error_message: None,
            last_update: Instant::now(),
            auto_refresh_interval: Duration::from_secs(3600), // Reduced from 1 hour to 30 seconds for demo
            heart_animation: HeartAnimation::new(),
            button_clicks: 0,
            easter_egg_shown: false,
            rt,
            message_receiver: Arc::new(Mutex::new(receiver)),
            message_sender: sender,
            cache_stats: (0, 0, 0, 0),
            connection_speed: "Medium".to_string(),
            api_key,
            preloaded_cats: std::collections::VecDeque::new(),
            // GIF support
            show_gifs: false,
            current_gif_url: String::new(),
            current_gif_title: String::new(),
            gif_texture: None,
            giphy_api_key: "b7EqESYjABoMKQ7kjqqxtZnelV17hKK1".to_string(),
            
            // Animated GIF support
            gif_frames: Vec::new(),
            gif_current_frame: 0,
            gif_last_frame_time: Instant::now(),
            gif_playing: false,
            preloading: false,
        };
        
        // Load initial content and start cache warmup
        app.refresh_content();
        
        // Start preloading 5 images immediately
        app.start_preloading();
        
        // Start background prefetching to warm up cache
        let prefetch_client = Arc::clone(&app.optimized_client);
        let prefetch_api_key = app.api_key.clone();
        let prefetch_manager = Arc::clone(&app.prefetch_manager);
        let rt_clone = Arc::clone(&app.rt);
        
        rt_clone.spawn(async move {
            // Wait a bit for initial load, then start prefetching
            tokio::time::sleep(Duration::from_secs(2)).await;
            prefetch_manager.start_prefetching(prefetch_client, prefetch_api_key).await;
        });
        
        app
    }

    fn love_notes() -> Vec<String> {
        vec![
            // Sweet and romantic
            "Every morning I wake up grateful that you're in my life ".to_string(),
            "You know that feeling when you see a perfect sunset? That's how I feel every time I see you".to_string(),
            "You make ordinary moments feel like magic".to_string(),
            "I could listen to you talk about anything for hours and never get bored (I just zone out sometimes)".to_string(),
            
            // Playful and fun
            "You're like catnip (weed) for my soul, completely addicted".to_string(),
            "If I were a cat, I'd spend all 9 lives with you...cliche I know".to_string(),
            "You're the reason I believe in fairy tales (and also why I'm not afraid of commitment)".to_string(),
            "You're my favorite notification, my best plot twist, my happy ending..I mean this metaphorically, My notifications are turned off lol".to_string(),
            
            // Deep and meaningful  
            "In a world full of temporary things, you're my constant".to_string(),
            "You see the best in me even when I can't see it myself".to_string(),
            "You make me want to be the person you already think I am".to_string(),
            "I love how we can be completely ourselves together, weird parts and all".to_string(),
            
            // Quirky and personal
            "You're like that perfect song that gets stuck in my head, but I never want it to stop playing".to_string(),
            "I love how you remember the little things I forget I even told you (Thiss!!)".to_string(),
            "You're my favorite person to do absolutely nothing with".to_string(),
            "Even when you're grumpy, you're still the cutest thing I've ever seen..haha".to_string(),
            "Sometimes you are a pain in the ass...Love you regardless".to_string(),
            
            // Grateful and appreciative
            "Thank you for loving all my broken pieces back together".to_string(),
            "You turn my anxiety into butterflies and my fears into adventures".to_string(),
            "I love how you make me feel like I'm exactly where I'm supposed to be".to_string(),
            "Every day with you feels like I'm getting away with something wonderful".to_string(),
        ]
    }

    fn get_random_message(&self) -> String {
        if self.button_clicks >= 10 && !self.easter_egg_shown {
            return "I love you".to_string();
        }
        
        let notes = Self::love_notes();
        notes.choose(&mut rand::thread_rng())
            .unwrap_or(&notes[0])
            .clone()
    }

    fn refresh_content(&mut self) {
        // Check if we have preloaded cats first
        if let Some((image_url, image_id, current_message, image_data, breed_info)) = self.preloaded_cats.pop_front() {
            // Use preloaded data immediately
            self.cat_image_url = image_url;
            self.current_image_id = image_id;
            self.current_message = current_message.clone();
            // Use the preloaded message instead of generating a new one
            self.current_breed_info = breed_info.clone();
            self.current_love_note = current_message.clone();
            // Keep current_fact for backward compatibility
            self.current_fact = if let Some(breed) = breed_info {
                format!("{}\n\n💕 {}", breed, current_message)
            } else {
                current_message
            };
            
            // Load image into texture
            match image::load_from_memory(&image_data) {
                Ok(dynamic_image) => {
                    let rgba_image = dynamic_image.to_rgba8();
                    let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                    let pixels = rgba_image.as_flat_samples();
                    
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    // We'll set this in the UI update loop since we need the context
                    let _ = self.message_sender.send(AppMessage::CatDataLoaded {
                        image_id: self.current_image_id.clone(),
                        image_url: self.cat_image_url.clone(),
                        image_data,
                        breed_info: Some(self.current_fact.clone()),
                    });
                }
                Err(e) => {
                    println!("Failed to decode preloaded image: {}", e);
                    self.fetch_new_cat();
                }
            }
            
            // Ensure we keep preloading
            if self.preloaded_cats.len() < 3 && !self.preloading {
                self.start_preloading();
            }
            
            self.loading = false;
            self.error_message = None;
            self.last_update = Instant::now();
            
            if self.button_clicks >= 10 && !self.easter_egg_shown {
                self.easter_egg_shown = true;
            }
        } else {
            // Fall back to regular fetch
            self.fetch_new_cat();
        }
    }
    
    fn fetch_new_cat(&mut self) {
        self.loading = true;
        self.error_message = None;
        
        let optimized_client = Arc::clone(&self.optimized_client);
        let image_cache = Arc::clone(&self.image_cache);
        let prefetch_manager = Arc::clone(&self.prefetch_manager);
        let sender = self.message_sender.clone();
        let rt = Arc::clone(&self.rt);
        let api_key = self.api_key.clone();
        
        // Spawn async task to fetch cat data with optimizations
        rt.spawn(async move {
            let start_time = std::time::Instant::now();
            println!("🚀 Starting cat data fetch with optimized client...");
            
            match Self::fetch_cat_data_optimized(optimized_client.clone(), image_cache, api_key.clone()).await {
                Ok((image_id, image_url, image_data, breed_info)) => {
                    println!("✅ Successfully fetched cat data: {} bytes", image_data.len());
                    let _ = sender.send(AppMessage::CatDataLoaded {
                        image_id,
                        image_url,
                        image_data,
                        breed_info,
                    });
                }
                Err(e) => {
                    println!("❌ Failed to fetch cat data: {}", e);
                    let _ = sender.send(AppMessage::LoadError(format!("Failed to load cat: {}", e)));
                }
            }
            
            // Update connection speed based on response time
            let request_duration = start_time.elapsed();
            optimized_client.update_connection_speed(request_duration).await;
            
            // Start prefetching in background
            prefetch_manager.start_prefetching(optimized_client, api_key).await;
        });
        
        // Update cache stats periodically
        let stats = self.image_cache.get_stats();
        if stats != self.cache_stats {
            let _ = self.message_sender.send(AppMessage::CacheStatsUpdated(stats.0, stats.1, stats.2, stats.3));
        }
        
        // Update message immediately
        self.current_message = self.get_random_message();
        self.last_update = Instant::now();
        
        if self.button_clicks >= 10 && !self.easter_egg_shown {
            self.easter_egg_shown = true;
        }
    }
    
    fn start_preloading(&mut self) {
        if self.preloading {
            return;
        }
        
        self.preloading = true;
        let optimized_client = Arc::clone(&self.optimized_client);
        let image_cache = Arc::clone(&self.image_cache);
        let sender = self.message_sender.clone();
        let rt = Arc::clone(&self.rt);
        let api_key = self.api_key.clone();
        
        rt.spawn(async move {
            println!("🔄 Starting preload of 5 cat images...");
            
            for i in 0..5 {
                match Self::fetch_cat_data_optimized(optimized_client.clone(), image_cache.clone(), api_key.clone()).await {
                    Ok((image_id, image_url, image_data, breed_info)) => {
                        println!("✅ Preloaded cat {} of 5: {} bytes", i + 1, image_data.len());
                        let _ = sender.send(AppMessage::PreloadedCat {
                            image_id,
                            image_url,
                            image_data,
                            breed_info,
                        });
                    }
                    Err(e) => {
                        println!("❌ Failed to preload cat {}: {}", i + 1, e);
                    }
                }
                
                // Small delay between requests to be nice to the API
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            
            let _ = sender.send(AppMessage::PreloadingComplete);
        });
    }

    async fn fetch_cat_data_optimized(
        client: Arc<OptimizedHttpClient>, 
        image_cache: Arc<ImageCache>, 
        api_key: String
    ) -> Result<(Option<String>, String, Vec<u8>, Option<String>), Box<dyn std::error::Error + Send + Sync>> {
        println!("🔍 Fetching cat API data with key: {}...", &api_key[..20]);
        
        // Use optimized API call with compression and adaptive quality
        let mut response = client.fetch_cat_api_with_compression(&api_key).await?;
        println!("📡 API response status: {}", response.status());
        
        let mut cat_images: Vec<CatImage> = response.json().await?;
        println!("📊 Received {} cat images", cat_images.len());
        
        // If no breed images found, try regular search
        if cat_images.is_empty() {
            println!("🔄 No breed images found, trying regular search...");
            response = client.fetch_cat_api_with_compression(&api_key).await?;
            cat_images = response.json().await?;
            println!("📊 Regular search returned {} images", cat_images.len());
        }
        
        if let Some(cat_image) = cat_images.first() {
            println!("🖼️ Downloading image from: {}", cat_image.url);
            // Use cached image download with optimizations
            let image_data = image_cache.get_or_fetch(&cat_image.url, &client).await?;
            println!("💾 Image downloaded: {} bytes", image_data.len());
            
            // Extract breed information with enhanced display
            let breed_info = if let Some(breed) = cat_image.breeds.first() {
                let mut info = format!("🐱 Beautiful {} Cat! 💜", breed.name);
                if let Some(temperament) = &breed.temperament {
                    info.push_str(&format!("\n✨ Personality: {}", temperament));
                }
                if let Some(origin) = &breed.origin {
                    info.push_str(&format!("\n🌍 From: {}", origin));
                }
                if let Some(description) = &breed.description {
                    info.push_str(&format!("\n💕 Description: {}", description));
                }
                Some(info)
            } else {
                Some("🐱 A mysterious and beautiful cat, just like you! 💜".to_string())
            };
            
            Ok((Some(cat_image.id.clone()), cat_image.url.clone(), image_data, breed_info))
        } else {
            Err("No cat images found".into())
        }
    }

    fn process_messages(&mut self, ctx: &egui::Context) {
        if let Ok(mut receiver) = self.message_receiver.try_lock() {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    AppMessage::CatDataLoaded { image_id, image_url, image_data, breed_info } => {
                        self.current_image_id = image_id;
                        self.cat_image_url = image_url;
                        // Store breed info and love note separately
                        let love_note = self.get_random_message();
                        self.current_breed_info = breed_info.clone();
                        self.current_love_note = love_note.clone();
                        // Keep current_fact for backward compatibility
                        self.current_fact = if let Some(breed) = breed_info {
                            format!("{}\n\n💕 {}", breed, love_note)
                        } else {
                            love_note
                        };
                        
                        // Load image into texture with better error handling
                        match image::load_from_memory(&image_data) {
                            Ok(dynamic_image) => {
                                let rgba_image = dynamic_image.to_rgba8();
                                let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                                let pixels = rgba_image.as_flat_samples();
                                
                                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                                self.texture = Some(ctx.load_texture("cat_image", color_image, egui::TextureOptions::default()));
                                println!("Successfully loaded cat image: {}x{}", size[0], size[1]);
                            }
                            Err(e) => {
                                println!("Failed to decode image: {}", e);
                                self.error_message = Some(format!("Image decode error: {}", e));
                            }
                        }
                        
                        self.loading = false;
                        self.error_message = None;
                        
                        // Check if this image is already favorited
                    }
                    AppMessage::LoadError(error) => {
                        self.error_message = Some(error);
                        self.loading = false;
                        // Use fallback data
                        self.current_breed_info = None;
                        self.current_love_note = "Oops, the kitties are napping! But you're still amazing!".to_string();
                        self.current_fact = "Oops, the kitties are napping! But you're still amazing! 💜".to_string();
                    }
                    AppMessage::CacheStatsUpdated(hits, misses, disk_hits, network_requests) => {
                        self.cache_stats = (hits, misses, disk_hits, network_requests);
                        println!("Cache stats - Hits: {}, Misses: {}, Disk hits: {}, Network: {}", hits, misses, disk_hits, network_requests);
                    }
                    AppMessage::ConnectionSpeedChanged(speed) => {
                        self.connection_speed = speed;
                        println!("Connection speed changed to: {}", self.connection_speed);
                    }
                    AppMessage::PreloadedCat { image_id, image_url, image_data, breed_info } => {
                        // Generate a random message for this preloaded cat
                        let message = self.get_random_message();
                        
                        // Store in preload queue
                        self.preloaded_cats.push_back((
                            image_url,
                            image_id,
                            message,
                            image_data,
                            breed_info,
                        ));
                        
                        println!("📦 Added preloaded cat to queue. Queue size: {}", self.preloaded_cats.len());
                    }
                    AppMessage::PreloadingComplete => {
                        self.preloading = false;
                        println!("✅ Preloading complete. {} cats ready.", self.preloaded_cats.len());
                    }
                    AppMessage::GifDataLoaded { gif_id: _, gif_url, gif_data, gif_title } => {
                        self.current_gif_url = gif_url;
                        self.current_gif_title = gif_title;
                        
                        // Decode animated GIF frames
                        match decode_animated_gif(&gif_data) {
                            Ok(frames) => {
                                self.gif_frames = frames;
                                self.gif_current_frame = 0;
                                self.gif_last_frame_time = Instant::now();
                                self.gif_playing = true;
                                
                                // Create initial texture from first frame
                                if let Some((first_frame, _)) = self.gif_frames.first() {
                                    let texture = ctx.load_texture("cat_gif", first_frame.clone(), egui::TextureOptions::default());
                                    self.gif_texture = Some(texture);
                                }
                                
                                //println!("✅ Successfully loaded animated GIF: {} ({} frames)", self.current_gif_title, self.gif_frames.len());
                            }
                            Err(e) => {
                                println!("❌ Failed to decode animated GIF: {}", e);
                                // Fallback to static image
                                match image::load_from_memory(&gif_data) {
                                    Ok(dynamic_image) => {
                                        let rgba_image = dynamic_image.to_rgba8();
                                        let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                                        let pixels = rgba_image.as_flat_samples();
                                        
                                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                                        let texture = ctx.load_texture("cat_gif", color_image, egui::TextureOptions::default());
                                        self.gif_texture = Some(texture);
                                        
                                        println!("⚠️ Loaded GIF as static image: {}", self.current_gif_title);
                                    }
                                    Err(e2) => {
                                        println!("❌ Failed to process GIF image: {}", e2);
                                        self.error_message = Some(format!("Failed to load GIF: {}", e2));
                                    }
                                }
                            }
                        }
                        self.loading = false;
                    }
                    AppMessage::GifLoadError(error) => {
                        self.error_message = Some(error);
                        self.loading = false;
                        println!("❌ GIF load error occurred");
                    }    
                }
                ctx.request_repaint();
            }
        }
    }


    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process incoming messages from async tasks
        self.process_messages(ctx);
        
        // Auto-refresh check
        if self.last_update.elapsed() >= self.auto_refresh_interval {
            self.refresh_content();
        }
        
        // Handle GIF animation
        if self.show_gifs && self.gif_playing && !self.gif_frames.is_empty() {
            let now = Instant::now();
            if let Some((_, duration)) = self.gif_frames.get(self.gif_current_frame) {
                if now.duration_since(self.gif_last_frame_time) >= *duration {
                    self.gif_current_frame = (self.gif_current_frame + 1) % self.gif_frames.len();
                    self.gif_last_frame_time = now;
                    
                    // Update texture with new frame
                    if let Some((frame, _)) = self.gif_frames.get(self.gif_current_frame) {
                        let texture = ctx.load_texture("cat_gif", frame.clone(), egui::TextureOptions::default());
                        self.gif_texture = Some(texture);
                    }
                }
            }
        }

        // Request repaint for animations
        if self.heart_animation.is_active() || (self.show_gifs && self.gif_playing) {
            ctx.request_repaint();
        }

        // Simple, clean background
        let bg_color = egui::Color32::from_rgb(250, 245, 255); // Very light lavender
        
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(bg_color))
            .show(ctx, |ui| {
            let available_size = ui.available_size();
            let text_size = 18.0;
            let img_width = 300.0;
            let img_height = 200.0;
            let padding = 20.0;
            
            // Colors
            let card_bg = egui::Color32::WHITE;
            let primary_color = egui::Color32::from_rgb(139, 69, 234);
            let text_color = egui::Color32::from_rgb(50, 50, 50);
            let border_color = egui::Color32::from_rgb(200, 200, 200);
            
            // Responsive padding
            let padding = if available_size.x < 500.0 { 
                16.0 
            } else { 
                32.0 
            };
            
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.add_space(padding);
                    
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(400.0);
                        
                        // Title
                        ui.label(egui::RichText::new("💜 Cat Love Notes")
                            .size(28.0)
                            .color(primary_color)
                            .strong());
                        
                        ui.add_space(padding);
                        
                        // Main card
                        egui::Frame::none()
                            .fill(card_bg)
                            .rounding(12.0)
                            .stroke(egui::Stroke::new(1.0, border_color))
                            .inner_margin(padding)
                            .show(ui, |ui| {
                                // Main cat display view
                                ui.vertical_centered(|ui| {
                                    // Content type toggle
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::SelectableLabel::new(!self.show_gifs, "📸 Photos")).clicked() {
                                            self.show_gifs = false;
                                        }
                                        if ui.add(egui::SelectableLabel::new(self.show_gifs, "🎬 GIFs")).clicked() {
                                            self.show_gifs = true;
                                        }
                                    });
                                    
                                    ui.add_space(padding / 2.0);
                                    
                                    // Display content based on mode
                                    if self.show_gifs {
                                        // GIF mode
                                        if let Some(gif_texture) = &self.gif_texture {
                                            ui.add(egui::Image::new(gif_texture).max_size(egui::Vec2::new(img_width, img_height)));
                                            
                                            ui.add_space(padding / 4.0);
                                            
                                            // GIF title
                                            if !self.current_gif_title.is_empty() {
                                                ui.label(egui::RichText::new(&format!("🎬 {}", self.current_gif_title))
                                                    .size(text_size + 1.0)
                                                    .color(text_color)
                                                    .strong());
                                                
                                                ui.add_space(padding / 6.0);
                                                
                                                // Clickable GIF URL with helpful message
                                                if !self.current_gif_url.is_empty() {
                                                    ui.label(egui::RichText::new("Click below to view the full GIF in your browser:")
                                                        .size(text_size - 2.0)
                                                        .color(egui::Color32::from_rgb(150, 150, 150)));
                                                    
                                                    if ui.link(egui::RichText::new("Open GIF in Browser")
                                                        .size(text_size)
                                                        .color(egui::Color32::from_rgb(100, 150, 255))
                                                        .underline()).clicked() {
                                                        if let Err(e) = webbrowser::open(&self.current_gif_url) {
                                                            println!("Failed to open URL: {}", e);
                                                        }
                                                    }
                                                }
                                            }
                                        } else if self.loading {
                                            ui.spinner();
                                            ui.label("Loading adorable cat GIF...");
                                        } else if let Some(error) = &self.error_message {
                                            ui.label(egui::RichText::new(&format!("😿 {}", error))
                                                .size(text_size)
                                                .color(text_color));
                                        } else {
                                            ui.label("Click 'Show Me More' to load a cat GIF!");
                                        }
                                    } else {
                                        // Photo mode
                                        if let Some(texture) = &self.texture {
                                            ui.add(egui::Image::new(texture).max_size(egui::Vec2::new(img_width, img_height)));
                                        } else if self.loading {
                                            ui.spinner();
                                            ui.label("Loading adorable cat...");
                                        } else if let Some(error) = &self.error_message {
                                            ui.label(egui::RichText::new(&format!("😿 {}", error))
                                                .size(text_size)
                                                .color(text_color));
                                        }
                                        
                                        ui.add_space(padding / 2.0);
                                        
                                        // Cat breed information (if available)
                                        if let Some(breed_info) = &self.current_breed_info {
                                            ui.label(egui::RichText::new(breed_info)
                                                .size(text_size + 2.0)
                                                .color(egui::Color32::from_rgb(180, 180, 180)));
                                            
                                            ui.add_space(padding / 3.0);
                                        }
                                        
                                        // Love note with distinctive styling
                                        if !self.current_love_note.is_empty() {
                                            ui.label(egui::RichText::new(&format!("💕 {}", self.current_love_note))
                                                .size(text_size + 2.0)
                                                .color(egui::Color32::from_rgb(255, 150, 180)));
                                        }
                                    }
                                });
                                
                                // Simple buttons
                                ui.horizontal(|ui| {
                                    if ui.add(egui::Button::new("💜 Show Me More")
                                        .fill(primary_color)
                                        .rounding(8.0))
                                        .clicked() {
                                        self.button_clicks += 1;
                                        self.heart_animation.trigger();
                                        
                                        if self.show_gifs {
                                            self.loading = true;
                                            self.error_message = None;
                                            let optimized_client = Arc::clone(&self.optimized_client);
                                            let sender = self.message_sender.clone();
                                            let rt = Arc::clone(&self.rt);
                                            let api_key = self.giphy_api_key.clone();
                                            
                                            rt.spawn(async move {
                                                println!("🎬 Fetching cat GIF from Giphy API...");
                                                
                                                let url = format!(
                                                    "https://api.giphy.com/v1/gifs/search?api_key={}&q=cat&limit=1&offset={}&rating=g&lang=en&bundle=messaging_non_clips",
                                                    api_key,
                                                    rand::random::<u32>() % 100
                                                );
                                                
                                                match optimized_client.client.get(&url).send().await {
                                                    Ok(response) => {
                                                        if response.status().is_success() {
                                                            match response.json::<GiphyResponse>().await {
                                                                Ok(giphy_response) => {
                                                                    if let Some(gif) = giphy_response.data.first() {
                                                                        let gif_url = &gif.images.fixed_height.url;
                                                                        println!("🎬 Found GIF: {} - {}", gif.title, gif_url);
                                                                        
                                                                        match optimized_client.client.get(gif_url).send().await {
                                                                            Ok(gif_response) => {
                                                                                match gif_response.bytes().await {
                                                                                    Ok(gif_data) => {
                                                                                        let _ = sender.send(AppMessage::GifDataLoaded {
                                                                                            gif_id: gif.id.clone(),
                                                                                            gif_url: gif_url.clone(),
                                                                                            gif_data: gif_data.to_vec(),
                                                                                            gif_title: gif.title.clone(),
                                                                                        });
                                                                                    }
                                                                                    Err(e) => {
                                                                                        let _ = sender.send(AppMessage::GifLoadError(format!("Failed to download GIF: {}", e)));
                                                                                    }
                                                                                }
                                                                            }
                                                                            Err(e) => {
                                                                                let _ = sender.send(AppMessage::GifLoadError(format!("Failed to fetch GIF: {}", e)));
                                                                            }
                                                                        }
                                                                    } else {
                                                                        let _ = sender.send(AppMessage::GifLoadError("No GIFs found".to_string()));
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    let _ = sender.send(AppMessage::GifLoadError(format!("Failed to parse Giphy response: {}", e)));
                                                                }
                                                            }
                                                        } else {
                                                            let _ = sender.send(AppMessage::GifLoadError(format!("Giphy API error: {}", response.status())));
                                                        }
                                                    }
                                                    Err(e) => {
                                                        let _ = sender.send(AppMessage::GifLoadError(format!("Network error: {}", e)));
                                                    }
                                                }
                                            });
                                        } else {
                                            self.refresh_content();
                                        }
                                    }
                                    
                                    if ui.add(egui::Button::new("♥")
                                        .fill(egui::Color32::from_rgb(220, 38, 127))
                                        .rounding(8.0))
                                        .clicked() {
                                        self.heart_animation.trigger();
                                    }
                                });
                                
                                ui.add_space(padding / 2.0);
                                
                                
                            });
                        
                        ui.add_space(padding);
                    });
                });
            
            // Draw heart animation
            // Draw heart animation
            if self.heart_animation.is_active() {
                let progress = self.heart_animation.progress();
                let alpha = (255.0 * (1.0 - progress)) as u8;
                let scale = 1.0 + progress * 2.0;
                
                ui.allocate_ui_at_rect(
                    egui::Rect::from_center_size(
                        ui.available_rect_before_wrap().center(),
                        egui::Vec2::splat(50.0 * scale)
                    ),
                    |ui| {
                        ui.label(egui::RichText::new("💖")
                            .size(30.0 * scale)
                            .color(egui::Color32::from_rgba_unmultiplied(255, 100, 150, alpha)));
                    }
                );
            }
        });
    }
}

impl eframe::App for CatLoveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update(ctx, _frame);
    }
}

fn main() -> Result<(), eframe::Error> {
    // Load icon data - resize to standard sizes for better compatibility
    let icon_data = include_bytes!("../assets/icon.ico");
    let icon = match image::load_from_memory(icon_data) {
        Ok(img) => {
            // Resize to 32x32 for better compatibility across platforms
            let resized = img.resize_exact(32, 32, image::imageops::FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let width = rgba.width();
            let height = rgba.height();
            println!("✅ Successfully loaded icon: {}x{}", width, height);
            Some(egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            })
        }
        Err(e) => {
            println!("❌ Failed to load icon: {}", e);
            // Create a simple cat-themed fallback icon
            let mut rgba_data = vec![0u8; 32 * 32 * 4];
            for y in 0..32 {
                for x in 0..32 {
                    let idx = (y * 32 + x) * 4;
                    // Create a simple purple gradient with cat ears shape
                    if (x < 8 && y < 8) || (x > 23 && y < 8) || (x > 8 && x < 24 && y > 8) {
                        rgba_data[idx] = 150;     // R
                        rgba_data[idx + 1] = 100; // G
                        rgba_data[idx + 2] = 200; // B
                        rgba_data[idx + 3] = 255; // A
                    } else {
                        rgba_data[idx + 3] = 0; // Transparent
                    }
                }
            }
            Some(egui::IconData {
                rgba: rgba_data,
                width: 32,
                height: 32,
            })
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 600.0])
            .with_min_inner_size([320.0, 480.0])
            .with_max_inner_size([420.0, 600.0])
            .with_resizable(true)
            .with_decorations(true)
            .with_icon(icon.unwrap()),
        ..Default::default()
    };
    
    eframe::run_native(
        "CuteCats",
        options,
        Box::new(|cc| Box::new(CatLoveApp::new(cc))),
    )
}
