#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui::{FontDefinitions, FontFamily};

// use futures::StreamExt; // Removed - not needed after simplifying stream handling
use rand::seq::SliceRandom;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

mod optimizations;
use optimizations::*;


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CatImage {
    id: Option<String>,
    url: String,
    #[serde(default)]
    breeds: Vec<Breed>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Favorite {
    id: Option<u64>,
    image_id: String,
    sub_id: Option<String>,
    created_at: Option<String>,
    image: Option<CatImage>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CreateFavorite {
    image_id: String,
    sub_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Breed {
    name: String,
    temperament: Option<String>,
    origin: Option<String>,
    description: Option<String>,
}

#[derive(Clone)]
enum AppMessage {
    CatDataLoaded { image_id: Option<String>, image_url: String, image_data: Vec<u8>, breed_info: Option<String> },
    LoadError(String),
    FavoriteAdded(String),
    FavoriteRemoved,
    FavoritesLoaded(Vec<Favorite>),
    FavoriteImageLoaded(String, Vec<u8>),
    CacheStatsUpdated(u64, u64, u64, u64), // hits, misses, disk_hits, network_requests
    ConnectionSpeedChanged(String),
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
        self.show_heart_animation
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
    is_favorited: bool,
    current_favorite_id: Option<String>,
    show_favorites: bool,
    favorites: Vec<Favorite>,
    user_id: String,
    favorite_textures: std::collections::HashMap<String, egui::TextureHandle>,
    
    // Performance monitoring
    cache_stats: (u64, u64, u64, u64),
    connection_speed: String,
    api_key: String,
}

impl CatLoveApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Setup custom fonts
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "Pacifico".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/Pacifico.ttf")),
        );
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "Pacifico".to_owned());
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
            texture: None,
            loading: false,
            error_message: None,
            last_update: Instant::now(),
            auto_refresh_interval: Duration::from_secs(30), // Reduced from 1 hour to 30 seconds for demo
            heart_animation: HeartAnimation::new(),
            button_clicks: 0,
            easter_egg_shown: false,
            rt,
            message_receiver: Arc::new(Mutex::new(receiver)),
            message_sender: sender,
            is_favorited: false,
            current_favorite_id: None,
            show_favorites: false,
            favorites: Vec::new(),
            user_id: "love_user_123".to_string(),
            favorite_textures: std::collections::HashMap::new(),
            cache_stats: (0, 0, 0, 0),
            connection_speed: "Medium".to_string(),
            api_key,
        };
        
        // Load initial content and start cache warmup
        app.refresh_content();
        
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
            "You know that feeling when you see a perfect sunset? That's how I feel every time I see you 🌅".to_string(),
            "You make ordinary moments feel like magic ✨".to_string(),
            "I could listen to you talk about anything for hours and never get bored (I just zone out sometimes)�".to_string(),
            
            // Playful and fun
            "You're like catnip (weed) for my soul, completely addicted ��".to_string(),
            "If I were a cat, I'd spend all 9 lives with you...cliche I know 🐾".to_string(),
            "You're the reason I believe in fairy tales (and also why I'm not afraid of commitment) �".to_string(),
            "You're my favorite notification, my best plot twist, my happy ending..I mean this metaphorically, My notifications are turned off lol 📱�".to_string(),
            
            // Deep and meaningful  
            "In a world full of temporary things, you're my constant 🌟".to_string(),
            "You see the best in me even when I can't see it myself 👀💜".to_string(),
            "You make me want to be the person you already think I am �".to_string(),
            "I love how we can be completely ourselves together, weird parts and all 🤪".to_string(),
            
            // Quirky and personal
            "You're like that perfect song that gets stuck in my head, but I never want it to stop playing 🎵".to_string(),
            "I love how you remember the little things I forget I even told you (Thiss!!) 🧠�".to_string(),
            "You're my favorite person to do absolutely nothing with 🛋️".to_string(),
            "Even when you're grumpy, you're still the cutest thing I've ever seen..haha �💜".to_string(),
            "Sometimes you are a pain in the ass...Love you regardless 🤪".to_string(),
            
            // Grateful and appreciative
            "Thank you for loving all my broken pieces back together 🧩".to_string(),
            "You turn my anxiety into butterflies and my fears into adventures 🦋".to_string(),
            "I love how you make me feel like I'm exactly where I'm supposed to be 🗺️".to_string(),
            "Every day with you feels like I'm getting away with something wonderful �".to_string(),
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
            
            Ok((cat_image.id.clone(), cat_image.url.clone(), image_data, breed_info))
        } else {
            Err("No cat images found".into())
        }
    }

    fn add_to_favorites(&self) {
        if let Some(ref image_id) = self.current_image_id {
            let batch_manager = Arc::clone(&self.batch_manager);
            let optimized_client = Arc::clone(&self.optimized_client);
            let sender = self.message_sender.clone();
            let rt = Arc::clone(&self.rt);
            let image_id = image_id.clone();
            let user_id = self.user_id.clone();
            let api_key = self.api_key.clone();
            
            rt.spawn(async move {
                println!("Adding favorite for image: {} using batch operations", image_id);
                
                // Use batch manager for optimized operations
                let operation = FavoriteOperation::Add(image_id, user_id);
                batch_manager.add_operation(operation, optimized_client, api_key).await;
                
                // For immediate UI feedback, send success message
                // In a real implementation, you'd wait for the batch result
                let _ = sender.send(AppMessage::FavoriteAdded("batch_pending".to_string()));
            });
        } else {
            println!("No current image ID to favorite");
        }
    }
    
    fn remove_from_favorites(&self) {
        if let Some(ref favorite_id) = self.current_favorite_id {
            let batch_manager = Arc::clone(&self.batch_manager);
            let optimized_client = Arc::clone(&self.optimized_client);
            let sender = self.message_sender.clone();
            let rt = Arc::clone(&self.rt);
            let favorite_id = favorite_id.clone();
            let api_key = self.api_key.clone();
            
            rt.spawn(async move {
                println!("Removing favorite: {} using batch operations", favorite_id);
                
                // Use batch manager for optimized operations
                let operation = FavoriteOperation::Remove(favorite_id);
                batch_manager.add_operation(operation, optimized_client, api_key).await;
                
                // For immediate UI feedback
                let _ = sender.send(AppMessage::FavoriteRemoved);
            });
        }
    }
    
    fn check_if_favorited(&self, image_id: String) {
        let optimized_client = Arc::clone(&self.optimized_client);
        let sender = self.message_sender.clone();
        let rt = Arc::clone(&self.rt);
        let user_id = self.user_id.clone();
        let api_key = self.api_key.clone();
        
        rt.spawn(async move {
            match optimized_client.client
                .get("https://api.thecatapi.com/v1/favourites")
                .header("x-api-key", &api_key)
                .header("Accept-Encoding", "gzip, br, deflate")
                .query(&[("sub_id", &user_id), ("image_id", &image_id)])
                .send()
                .await
            {
                Ok(response) => {
                    if let Ok(favorites) = response.json::<Vec<Favorite>>().await {
                        if let Some(favorite) = favorites.first() {
                            if let Some(id) = &favorite.id {
                                let _ = sender.send(AppMessage::FavoriteAdded(id.to_string()));
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to check favorites: {}", e);
                }
            }
        });
    }
    
    fn load_favorites(&self) {
        let optimized_client = Arc::clone(&self.optimized_client);
        let sender = self.message_sender.clone();
        let rt = Arc::clone(&self.rt);
        let user_id = self.user_id.clone();
        let api_key = self.api_key.clone();
        println!("Loading favorites for user: {}", user_id);
        rt.spawn(async move {
            match optimized_client.client
                .get("https://api.thecatapi.com/v1/favourites")
                .header("x-api-key", &api_key)
                .header("Accept-Encoding", "gzip, br, deflate")
                .query(&[("sub_id", user_id.as_str()), ("attach_image", "1"), ("limit", "20")])
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();
                    println!("Load favorites API response status: {}", status);
                    if status.is_success() {
                        match response.json::<Vec<Favorite>>().await {
                            Ok(favorites) => {
                                println!("Successfully loaded {} favorites", favorites.len());
                                let _ = sender.send(AppMessage::FavoritesLoaded(favorites));
                            }
                            Err(e) => {
                                println!("Failed to parse favorites response: {}", e);
                            }
                        }
                    } else {
                        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        println!("Load favorites API error {}: {}", status, error_text);
                    }
                }
                Err(e) => {
                    println!("Failed to load favorites (network error): {}", e);
                }
            }
        });
    }

    fn load_favorite_image(&self, url: String, image_id: String) {
        let image_cache = Arc::clone(&self.image_cache);
        let optimized_client = Arc::clone(&self.optimized_client);
        let sender = self.message_sender.clone();
        let rt = Arc::clone(&self.rt);
        rt.spawn(async move {
            match image_cache.get_or_fetch(&url, &optimized_client).await {
                Ok(image_data) => {
                    let _ = sender.send(AppMessage::FavoriteImageLoaded(image_id, image_data));
                }
                Err(e) => {
                    println!("Failed to fetch favorite image: {}", e);
                }
            }
        });
    }

    fn process_messages(&mut self, ctx: &egui::Context) {
        if let Ok(mut receiver) = self.message_receiver.try_lock() {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    AppMessage::CatDataLoaded { image_id, image_url, image_data, breed_info } => {
                        self.current_image_id = image_id;
                        self.cat_image_url = image_url;
                        self.is_favorited = false;
                        self.current_favorite_id = None;
                        self.current_fact = breed_info.unwrap_or_else(|| 
                            "This adorable kitty is just as cute as you! 💜".to_string()
                        );
                        
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
                        if let Some(ref image_id) = self.current_image_id {
                            self.check_if_favorited(image_id.clone());
                        }
                    }
                    AppMessage::LoadError(error) => {
                        self.error_message = Some(error);
                        self.loading = false;
                        // Use fallback data
                        self.current_fact = "Oops, the kitties are napping! But you're still amazing! 💜".to_string();
                    }
                    AppMessage::FavoriteAdded(favorite_id) => {
                        self.is_favorited = true;
                        self.current_favorite_id = Some(favorite_id);
                        self.heart_animation.trigger();
                    }
                    AppMessage::FavoriteRemoved => {
                        self.is_favorited = false;
                        self.current_favorite_id = None;
                    }
                    AppMessage::FavoritesLoaded(favorites) => {
                        self.favorites = favorites;
                        // Load images for favorites
                        for favorite in &self.favorites {
                            if let Some(image) = &favorite.image {
                                if let Some(image_id) = &image.id {
                                    if !self.favorite_textures.contains_key(image_id) {
                                        self.load_favorite_image(image.url.clone(), image_id.clone());
                                    }
                                }
                            }
                        }
                    }
                    AppMessage::FavoriteImageLoaded(image_id, image_data) => {
                        match image::load_from_memory(&image_data) {
                            Ok(dynamic_image) => {
                                let rgba_image = dynamic_image.to_rgba8();
                                let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                                let pixels = rgba_image.as_flat_samples();
                                
                                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                                let texture = ctx.load_texture(&format!("favorite_{}", image_id), color_image, egui::TextureOptions::default());
                                self.favorite_textures.insert(image_id, texture);
                                println!("Successfully loaded favorite image texture from cache");
                            }
                            Err(e) => {
                                println!("Failed to decode favorite image: {}", e);
                            }
                        }
                    }
                    AppMessage::CacheStatsUpdated(hits, misses, disk_hits, network_requests) => {
                        self.cache_stats = (hits, misses, disk_hits, network_requests);
                        println!("Cache stats - Hits: {}, Misses: {}, Disk hits: {}, Network: {}", hits, misses, disk_hits, network_requests);
                    }
                    AppMessage::ConnectionSpeedChanged(speed) => {
                        self.connection_speed = speed;
                        println!("Connection speed changed to: {}", self.connection_speed);
                    }
                }
                ctx.request_repaint();
            }
        }
    }

    fn setup_professional_theme(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        
        // Romantic gradient color palette - inspired by sunset and roses
        let is_dark_mode = ctx.style().visuals.dark_mode;
        
        let (primary_gradient_start, primary_gradient_end, secondary_rose, accent_gold, 
             background_gradient_start, background_gradient_end, surface_glass, 
             text_romantic, text_soft, border_shimmer) = if is_dark_mode {
            // Dark romantic theme - midnight romance
            (
                egui::Color32::from_rgb(139, 69, 234),      // Deep purple start
                egui::Color32::from_rgb(219, 39, 119),      // Rose pink end
                egui::Color32::from_rgb(244, 114, 182),     // Soft rose
                egui::Color32::from_rgb(251, 191, 36),      // Warm gold
                egui::Color32::from_rgb(15, 23, 42),        // Deep navy start
                egui::Color32::from_rgb(30, 41, 59),        // Slate end
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8), // Glass effect
                egui::Color32::from_rgb(253, 244, 255),     // Romantic white
                egui::Color32::from_rgb(219, 234, 254),     // Soft blue
                egui::Color32::from_rgb(168, 85, 247),      // Shimmer purple
            )
        } else {
            // Light romantic theme - dreamy pastels
            (
                egui::Color32::from_rgb(251, 113, 133),     // Rose start
                egui::Color32::from_rgb(147, 51, 234),      // Purple end
                egui::Color32::from_rgb(252, 165, 165),     // Blush pink
                egui::Color32::from_rgb(251, 191, 36),      // Golden accent
                egui::Color32::from_rgb(255, 241, 242),     // Soft rose background
                egui::Color32::from_rgb(243, 232, 255),     // Lavender background
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200), // Frosted glass
                egui::Color32::from_rgb(75, 85, 99),        // Charcoal text
                egui::Color32::from_rgb(107, 114, 128),     // Soft gray
                egui::Color32::from_rgb(219, 39, 119),      // Rose shimmer
            )
        };
        
        // Dreamy gradient background - ensure all backgrounds match
        style.visuals.window_fill = background_gradient_start;
        style.visuals.panel_fill = background_gradient_start;
        style.visuals.extreme_bg_color = background_gradient_start;
        style.visuals.faint_bg_color = background_gradient_start;
        style.visuals.code_bg_color = background_gradient_start;
        
        // Romantic button styling with gradients
        style.visuals.widgets.inactive.bg_fill = primary_gradient_start;
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(0.0, egui::Color32::WHITE);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(20.0);
        style.visuals.widgets.inactive.expansion = 0.0;
        
        style.visuals.widgets.hovered.bg_fill = primary_gradient_end;
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(0.0, egui::Color32::WHITE);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(20.0);
        style.visuals.widgets.hovered.expansion = 3.0;
        
        style.visuals.widgets.active.bg_fill = secondary_rose;
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(0.0, egui::Color32::WHITE);
        style.visuals.widgets.active.rounding = egui::Rounding::same(20.0);
        style.visuals.widgets.active.expansion = -2.0;
        
        // Elegant text styling
        style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(0.0, text_romantic);
        
        // Luxurious spacing - remove all margins to eliminate solid color bars
        style.spacing.item_spacing = egui::vec2(20.0, 24.0);
        style.spacing.window_margin = egui::style::Margin::same(0.0);
        style.spacing.button_padding = egui::vec2(32.0, 20.0);
        style.spacing.indent = 32.0;
        style.spacing.menu_margin = egui::style::Margin::same(0.0);
        
        // Dreamy window styling with romantic shadows
        style.visuals.window_rounding = egui::Rounding::same(28.0);
        style.visuals.window_shadow = egui::epaint::Shadow {
            extrusion: 20.0,
            color: egui::Color32::from_rgba_unmultiplied(219, 39, 119, 25),
        };
        
        // Glass morphism cards
        style.visuals.widgets.noninteractive.bg_fill = surface_glass;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(20.0);
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.5, border_shimmer);
        
        ctx.set_style(style);
    }
    
    fn get_responsive_sizes(&self, available_size: egui::Vec2) -> (f32, f32, f32, f32) {
        let is_compact = available_size.x < 500.0 || available_size.y < 600.0;
        
        if is_compact {
            // Compact layout
            (
                available_size.x.min(280.0), // image width
                200.0,                        // image height
                14.0,                         // title size
                12.0,                         // text size
            )
        } else {
            // Standard layout
            (
                available_size.x.min(360.0), // image width
                270.0,                        // image height
                18.0,                         // title size
                14.0,                         // text size
            )
        }
    }

    fn draw_heart_animation(&mut self, ui: &mut egui::Ui) {
        if !self.heart_animation.is_active() {
            return;
        }

        let time = ui.input(|i| i.time) as f32;
        let window_rect = ui.max_rect();
        
        // Create multiple floating hearts with romantic colors
        let heart_colors = [
            egui::Color32::from_rgba_unmultiplied(251, 113, 133, 180), // Rose pink
            egui::Color32::from_rgba_unmultiplied(236, 72, 153, 160),  // Deep pink
            egui::Color32::from_rgba_unmultiplied(251, 146, 60, 140),  // Warm orange
            egui::Color32::from_rgba_unmultiplied(168, 85, 247, 120),  // Purple
            egui::Color32::from_rgba_unmultiplied(251, 191, 36, 100),  // Gold
        ];
        
        for i in 0..12 {
            let phase = (i as f32 * 0.8) + time * 1.5;
            let x_offset = (phase * 0.7).sin() * 80.0;
            let y_progress = (time * 0.8 + i as f32 * 0.3) % 4.0;
            
            let start_x = window_rect.min.x + (i as f32 * window_rect.width() / 12.0) + x_offset;
            let start_y = window_rect.max.y + 50.0 - (y_progress * window_rect.height() * 0.4);
            
            let pos = egui::Pos2::new(start_x, start_y);
            
            // Only draw if heart is within visible area
            if pos.y > window_rect.min.y - 50.0 && pos.y < window_rect.max.y + 50.0 {
                let color = heart_colors[i % heart_colors.len()];
                let size_variation = 1.0 + (phase * 2.0).sin() * 0.3;
                let base_size = 16.0 + (i % 3) as f32 * 4.0;
                let heart_size = base_size * size_variation;
                
                // Draw romantic glowing heart with shadow
                let shadow_pos = pos + egui::Vec2::new(2.0, 2.0);
                ui.painter().text(
                    shadow_pos,
                    egui::Align2::CENTER_CENTER,
                    "♥",
                    egui::FontId::new(heart_size, egui::FontFamily::Proportional),
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 30)
                );
                
                ui.painter().text(
                    pos,
                    egui::Align2::CENTER_CENTER,
                    "♥",
                    egui::FontId::new(heart_size, egui::FontFamily::Proportional),
                    color
                );
                
                // Add sparkle effects around hearts
                if i % 3 == 0 {
                    let sparkle_offset = egui::Vec2::new(
                        (phase * 3.0).cos() * 25.0,
                        (phase * 2.5).sin() * 20.0
                    );
                    let sparkle_pos = pos + sparkle_offset;
                    ui.painter().text(
                        sparkle_pos,
                        egui::Align2::CENTER_CENTER,
                        "✧",
                        egui::FontId::new(8.0, egui::FontFamily::Proportional),
                        egui::Color32::from_rgba_unmultiplied(251, 191, 36, 120)
                    );
                }
            }
        }
        
        // Auto-hide animation after duration
        if time - self.heart_animation.start_time.elapsed().as_secs_f32() > 4.0 {
            self.heart_animation.show_heart_animation = false;
        }
    }
    
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process incoming messages from async tasks
        self.process_messages(ctx);
        
        // Auto-refresh check
        if self.last_update.elapsed() >= self.auto_refresh_interval {
            self.refresh_content();
        }
        
        // Request repaint for animations
        if self.heart_animation.is_active() {
            ctx.request_repaint();
        }

        // Simple, clean background
        let bg_color = egui::Color32::from_rgb(250, 245, 255); // Very light lavender
        
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(bg_color))
            .show(ctx, |ui| {
            let available_size = ui.available_size();
            let text_size = 14.0;
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
                            .size(24.0)
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
                                if self.show_favorites {
                                    // Favorites view
                                    ui.vertical_centered(|ui| {
                                        ui.label(egui::RichText::new("💜 Your Favorite Cats")
                                            .size(18.0)
                                            .color(text_color)
                                            .strong());
                                        
                                        ui.add_space(padding);
                                        
                                        if self.favorites.is_empty() {
                                            ui.label(egui::RichText::new("No favorites yet! Click the heart on any cat to save it.")
                                                .size(text_size)
                                                .color(text_color));
                                        } else {
                                            ui.label(egui::RichText::new(&format!("You have {} favorite cats", self.favorites.len()))
                                                .size(text_size)
                                                .color(text_color));
                                        }
                                    });
                                } else {
                                    // Main cat view
                                    ui.vertical_centered(|ui| {
                                        // Cat image
                                        if let Some(texture) = &self.texture {
                                            ui.add(egui::Image::from_texture(texture)
                                                .fit_to_exact_size(egui::Vec2::new(img_width, img_height))
                                                .rounding(8.0));
                                        } else if self.loading {
                                            let rect = ui.allocate_response(
                                                egui::Vec2::new(img_width, img_height),
                                                egui::Sense::hover()
                                            ).rect;
                                            ui.painter().rect_filled(rect, 8.0, egui::Color32::LIGHT_GRAY);
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                "Loading...",
                                                egui::FontId::proportional(16.0),
                                                text_color
                                            );
                                        } else {
                                            let rect = ui.allocate_response(
                                                egui::Vec2::new(img_width, img_height),
                                                egui::Sense::hover()
                                            ).rect;
                                            ui.painter().rect_filled(rect, 8.0, egui::Color32::LIGHT_GRAY);
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                "🐱\nClick 'Show Me Love' to see cats!",
                                                egui::FontId::proportional(16.0),
                                                text_color
                                            );
                                        }
                                    });
                                }
                                
                                ui.add_space(padding);
                                
                                // Love message
                                ui.label(egui::RichText::new(&self.current_message)
                                    .size(16.0)
                                    .color(primary_color)
                                    .strong());
                                
                                ui.add_space(padding / 2.0);
                                
                                // Cat fact
                                ui.label(egui::RichText::new(&self.current_fact)
                                    .size(text_size)
                                    .color(text_color));
                                
                                ui.add_space(padding);
                                
                                let button_width = if available_size.x < 400.0 { 
                                    ui.available_width() - 32.0 
                                } else { 
                                    240.0 
                                };
                                
                                // Simple buttons
                                ui.horizontal(|ui| {
                                    if ui.add(egui::Button::new("💜 Show Me Love")
                                        .fill(primary_color)
                                        .rounding(8.0))
                                        .clicked() {
                                        self.button_clicks += 1;
                                        self.heart_animation.trigger();
                                        self.refresh_content();
                                    }
                                    
                                    let heart_text = if self.is_favorited { "♥" } else { "♡" };
                                    if ui.add(egui::Button::new(heart_text)
                                        .fill(if self.is_favorited { egui::Color32::from_rgb(220, 38, 127) } else { egui::Color32::GRAY })
                                        .rounding(8.0))
                                        .clicked() {
                                        if self.is_favorited {
                                            self.remove_from_favorites();
                                        } else {
                                            self.add_to_favorites();
                                        }
                                        self.heart_animation.trigger();
                                    }
                                });
                                
                                ui.add_space(padding / 2.0);
                                
                                // Navigation
                                let favorites_text = if self.show_favorites { "← Back to Cats" } else { "♥ My Favorites" };
                                if ui.add(egui::Button::new(favorites_text)
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::new(1.0, primary_color))
                                    .rounding(8.0))
                                    .clicked() {
                                    self.show_favorites = !self.show_favorites;
                                    if self.show_favorites {
                                        self.load_favorites();
                                    } else if self.texture.is_none() && !self.loading {
                                        self.refresh_content();
                                    }
                                }
                                
                                // Easter egg progress
                                if self.button_clicks < 10 && self.button_clicks > 0 {
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new(&format!("♥ {} clicks until surprise!", 10 - self.button_clicks))
                                        .size(12.0)
                                        .color(primary_color)
                                        .italics());
                                }
                            });
                        
                        ui.add_space(padding);
                    });
                });
            
            // Draw heart animation
            self.draw_heart_animation(ui);
        });
    }
}

impl eframe::App for CatLoveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update(ctx, _frame);
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 600.0])
            .with_min_inner_size([320.0, 480.0])
            .with_resizable(true)
            .with_decorations(true),
        ..Default::default()
    };
    
    eframe::run_native(
        "catWidget",
        options,
        Box::new(|cc| Box::new(CatLoveApp::new(cc))),
    )
}
