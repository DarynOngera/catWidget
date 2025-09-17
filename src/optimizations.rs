use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering}};
use std::time::{Duration, Instant};
use std::num::NonZeroUsize;
use reqwest::Client;
use lru::LruCache;
use dashmap::DashMap;
use tokio::sync::{broadcast, Mutex};
// use futures::StreamExt; // Removed - not needed after simplifying stream handling
use anyhow::{Result, Error};

// Image Cache with LRU memory cache and disk persistence
pub struct ImageCache {
    memory_cache: Arc<Mutex<LruCache<String, Vec<u8>>>>,
    disk_cache_dir: PathBuf,
    cache_stats: CacheStats,
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub disk_hits: AtomicU64,
    pub network_requests: AtomicU64,
}

impl ImageCache {
    pub fn new() -> Result<Self> {
        let cache_size = NonZeroUsize::new(50).unwrap();
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join("cat-love-notes")
            .join("images");
        
        std::fs::create_dir_all(&cache_dir)?;
        
        Ok(Self {
            memory_cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
            disk_cache_dir: cache_dir,
            cache_stats: CacheStats::default(),
        })
    }
    
    pub async fn get_or_fetch(&self, url: &str, client: &OptimizedHttpClient) -> Result<Vec<u8>> {
        let cache_key = format!("{:x}", md5::compute(url));
        println!("🔍 Cache lookup for key: {}", &cache_key[..8]);
        
        // Check memory cache first
        {
            let mut cache = self.memory_cache.lock().await;
            if let Some(data) = cache.get(&cache_key) {
                println!("⚡ Memory cache HIT for {}", &cache_key[..8]);
                self.cache_stats.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(data.clone());
            }
            println!("❌ Memory cache MISS for {}", &cache_key[..8]);
        }
        
        // Check disk cache
        let cache_file = self.disk_cache_dir.join(&cache_key);
        if cache_file.exists() {
            println!("💽 Disk cache file exists for {}", &cache_key[..8]);
            if let Ok(data) = tokio::fs::read(&cache_file).await {
                println!("⚡ Disk cache HIT for {}", &cache_key[..8]);
                // Store back in memory cache
                let mut cache = self.memory_cache.lock().await;
                cache.put(cache_key.clone(), data.clone());
                self.cache_stats.disk_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(data);
            }
        } else {
            println!("❌ Disk cache MISS for {}", &cache_key[..8]);
        }
        
        // Fetch from network as last resort
        println!("🌐 Fetching from network: {}", url);
        self.cache_stats.misses.fetch_add(1, Ordering::Relaxed);
        self.cache_stats.network_requests.fetch_add(1, Ordering::Relaxed);
        
        let data = client.download_image_optimized(url).await?;
        println!("📥 Downloaded {} bytes from network", data.len());
        
        // Store in both caches
        {
            let mut cache = self.memory_cache.lock().await;
            cache.put(cache_key.clone(), data.clone());
        }
        
        // Store on disk asynchronously
        let cache_file_clone = cache_file.clone();
        let data_clone = data.clone();
        tokio::spawn(async move {
            let _ = tokio::fs::write(cache_file_clone, data_clone).await;
        });
        
        Ok(data)
    }
    
    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        (
            self.cache_stats.hits.load(Ordering::Relaxed),
            self.cache_stats.misses.load(Ordering::Relaxed),
            self.cache_stats.disk_hits.load(Ordering::Relaxed),
            self.cache_stats.network_requests.load(Ordering::Relaxed),
        )
    }
}

// Optimized HTTP Client with compression and connection pooling
pub struct OptimizedHttpClient {
    pub client: Client,
    connection_speed: Arc<Mutex<ConnectionSpeed>>,
    request_deduplicator: RequestDeduplicator,
    circuit_breaker: CircuitBreaker,
}

#[derive(Clone, Debug)]
pub enum ConnectionSpeed {
    Slow,    // > 2 seconds per request
    Medium,  // 0.5-2 seconds
    Fast,    // < 0.5 seconds
}

impl OptimizedHttpClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(8)
            .tcp_keepalive(Duration::from_secs(30))
            .http1_only()
            .user_agent("CatLoveNotes/1.0 (Optimized)")
            .build()?;
        
        Ok(Self {
            client,
            connection_speed: Arc::new(Mutex::new(ConnectionSpeed::Medium)),
            request_deduplicator: RequestDeduplicator::new(),
            circuit_breaker: CircuitBreaker::new(),
        })
    }
    
    pub async fn download_image_optimized(&self, url: &str) -> Result<Vec<u8>> {
        // Check circuit breaker
        if self.circuit_breaker.is_open() {
            return Err(Error::msg("Circuit breaker open - too many failures"));
        }
        
        let start_time = Instant::now();
        
        // Use request deduplication
        let result = self.request_deduplicator.deduplicated_fetch(&self.client, url.to_string()).await;
        
        // Update connection speed based on response time
        let request_duration = start_time.elapsed();
        self.update_connection_speed(request_duration).await;
        
        match &result {
            Ok(_) => self.circuit_breaker.record_success(),
            Err(_) => self.circuit_breaker.record_failure(),
        }
        
        result
    }
    
    pub async fn fetch_cat_api_with_compression(&self, api_key: &str) -> Result<reqwest::Response> {
        println!("🌐 Making API request to Cat API...");
        
        let mut request = self.client
            .get("https://api.thecatapi.com/v1/images/search")
            .header("x-api-key", api_key)
            .header("Accept-Encoding", "gzip, br, deflate")
            .header("Cache-Control", "max-age=300")
            .query(&[("has_breeds", "1"), ("limit", "1")]);
        
        // Add adaptive quality based on connection speed
        let speed = self.connection_speed.lock().await.clone();
        println!("📶 Connection speed: {:?}", speed);
        
        match speed {
            ConnectionSpeed::Slow => {
                request = request.query(&[("size", "small")]);
                println!("📉 Using small image size for slow connection");
            }
            ConnectionSpeed::Medium => {
                request = request.query(&[("size", "med")]);
                println!("📊 Using medium image size for medium connection");
            }
            ConnectionSpeed::Fast => {
                println!("📈 Using full quality for fast connection");
                // Use full quality
            }
        }
        
        println!("🚀 Sending request to: https://api.thecatapi.com/v1/images/search");
        let response = request.send().await?;
        println!("📨 Received response with status: {}", response.status());
        Ok(response)
    }
    
    pub async fn update_connection_speed(&self, duration: Duration) {
        let mut speed = self.connection_speed.lock().await;
        *speed = match duration.as_secs_f32() {
            t if t > 2.0 => ConnectionSpeed::Slow,
            t if t > 0.5 => ConnectionSpeed::Medium,
            _ => ConnectionSpeed::Fast,
        };
    }
    
    pub async fn get_connection_speed(&self) -> ConnectionSpeed {
        self.connection_speed.lock().await.clone()
    }
}

// Request Deduplication to prevent duplicate API calls
pub struct RequestDeduplicator {
    pending_requests: Arc<DashMap<String, broadcast::Sender<Result<Vec<u8>, String>>>>,
}

impl RequestDeduplicator {
    pub fn new() -> Self {
        Self {
            pending_requests: Arc::new(DashMap::new()),
        }
    }
    
    pub async fn deduplicated_fetch(&self, client: &Client, url: String) -> Result<Vec<u8>> {
        // Check if same request is already in flight
        if let Some(sender) = self.pending_requests.get(&url) {
            let mut receiver = sender.subscribe();
            return match receiver.recv().await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(Error::msg(e)),
                Err(_) => Err(Error::msg("Request cancelled")),
            };
        }
        
        // Create new request
        let (tx, _rx) = broadcast::channel(16);
        self.pending_requests.insert(url.clone(), tx.clone());
        
        // Make the actual request with streaming
        let result = self.stream_download(client, &url).await;
        
        // Broadcast result to all waiting requests
        let broadcast_result = match &result {
            Ok(data) => Ok(data.clone()),
            Err(e) => Err(e.to_string()),
        };
        let _ = tx.send(broadcast_result);
        
        // Clean up
        self.pending_requests.remove(&url);
        
        result
    }
    
    async fn stream_download(&self, client: &Client, url: &str) -> Result<Vec<u8>> {
        let response = client
            .get(url)
            .header("Accept", "image/webp,image/jpeg,image/png,*/*;q=0.8")
            .send()
            .await?;
        
        let content_length = response.content_length().unwrap_or(0);
        let mut buffer = if content_length > 0 {
            Vec::with_capacity(content_length as usize)
        } else {
            Vec::new()
        };
        
        let bytes = response.bytes().await?;
        buffer.extend_from_slice(&bytes);
            
        // Yield control for UI responsiveness
        tokio::task::yield_now().await;
        
        Ok(buffer)
    }
}

// Circuit Breaker to handle API failures gracefully
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    last_failure: AtomicU64,
    state: AtomicU8, // 0=Closed, 1=Open, 2=HalfOpen
    failure_threshold: u32,
    timeout_duration: Duration,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            last_failure: AtomicU64::new(0),
            state: AtomicU8::new(0), // Closed
            failure_threshold: 5,
            timeout_duration: Duration::from_secs(60),
        }
    }
    
    pub fn is_open(&self) -> bool {
        let state = self.state.load(Ordering::Relaxed);
        if state == 1 { // Open
            let last_failure = self.last_failure.load(Ordering::Relaxed);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            if now - last_failure > self.timeout_duration.as_secs() {
                // Transition to half-open
                self.state.store(2, Ordering::Relaxed);
                false
            } else {
                true
            }
        } else {
            false
        }
    }
    
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.state.store(0, Ordering::Relaxed); // Closed
    }
    
    pub fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        self.last_failure.store(now, Ordering::Relaxed);
        
        if failures >= self.failure_threshold {
            self.state.store(1, Ordering::Relaxed); // Open
        }
    }
}

// Prefetch Manager for intelligent background loading
pub struct PrefetchManager {
    prefetch_queue: Arc<Mutex<VecDeque<String>>>,
    prefetch_cache: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    is_prefetching: Arc<Mutex<bool>>,
}

impl PrefetchManager {
    pub fn new() -> Self {
        Self {
            prefetch_queue: Arc::new(Mutex::new(VecDeque::new())),
            prefetch_cache: Arc::new(Mutex::new(HashMap::new())),
            is_prefetching: Arc::new(Mutex::new(false)),
        }
    }
    
    pub async fn start_prefetching(&self, client: Arc<OptimizedHttpClient>, api_key: String) {
        let mut is_prefetching = self.is_prefetching.lock().await;
        if *is_prefetching {
            return; // Already prefetching
        }
        *is_prefetching = true;
        drop(is_prefetching);
        
        let prefetch_cache = Arc::clone(&self.prefetch_cache);
        let is_prefetching = Arc::clone(&self.is_prefetching);
        
        tokio::spawn(async move {
            // Prefetch 3 images in background
            for _ in 0..3 {
                if let Ok(response) = client.fetch_cat_api_with_compression(&api_key).await {
                    if let Ok(cat_images) = response.json::<Vec<crate::CatImage>>().await {
                        if let Some(cat) = cat_images.first() {
                            if let Ok(img_data) = client.download_image_optimized(&cat.url).await {
                                let mut cache = prefetch_cache.lock().await;
                                cache.insert(cat.url.clone(), img_data);
                                
                                // Limit cache size
                                if cache.len() > 10 {
                                    let oldest_key = cache.keys().next().cloned();
                                    if let Some(key) = oldest_key {
                                        cache.remove(&key);
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Wait between prefetch requests
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            
            let mut is_prefetching = is_prefetching.lock().await;
            *is_prefetching = false;
        });
    }
    
    pub async fn get_prefetched(&self, url: &str) -> Option<Vec<u8>> {
        let mut cache = self.prefetch_cache.lock().await;
        cache.remove(url)
    }
}

// Batch operations for favorites
pub struct BatchOperationManager {
    pending_operations: Arc<Mutex<Vec<FavoriteOperation>>>,
    batch_timer: Arc<Mutex<Option<Instant>>>,
    batch_interval: Duration,
}

#[derive(Clone, Debug)]
pub enum FavoriteOperation {
    Add(String, String), // image_id, user_id
    Remove(String),      // favorite_id
}

impl BatchOperationManager {
    pub fn new() -> Self {
        Self {
            pending_operations: Arc::new(Mutex::new(Vec::new())),
            batch_timer: Arc::new(Mutex::new(None)),
            batch_interval: Duration::from_millis(500), // Batch operations for 500ms
        }
    }
    
    pub async fn add_operation(&self, operation: FavoriteOperation, client: Arc<OptimizedHttpClient>, api_key: String) {
        let mut operations = self.pending_operations.lock().await;
        operations.push(operation);
        
        let mut timer = self.batch_timer.lock().await;
        if timer.is_none() {
            *timer = Some(Instant::now());
            
            // Schedule batch execution
            let pending_ops = Arc::clone(&self.pending_operations);
            let batch_timer = Arc::clone(&self.batch_timer);
            let interval = self.batch_interval;
            
            tokio::spawn(async move {
                tokio::time::sleep(interval).await;
                
                let operations_to_execute = {
                    let mut ops = pending_ops.lock().await;
                    let mut timer = batch_timer.lock().await;
                    *timer = None;
                    std::mem::take(&mut *ops)
                };
                
                if !operations_to_execute.is_empty() {
                    Self::execute_batch(operations_to_execute, client, api_key).await;
                }
            });
        }
    }
    
    async fn execute_batch(operations: Vec<FavoriteOperation>, client: Arc<OptimizedHttpClient>, api_key: String) {
        let futures: Vec<_> = operations.into_iter().map(|op| {
            let client = &client.client;
            let api_key = &api_key;
            
            async move {
                match op {
                    FavoriteOperation::Add(image_id, user_id) => {
                        let create_favorite = serde_json::json!({ "image_id": image_id, "sub_id": user_id });
                        client
                            .post("https://api.thecatapi.com/v1/favourites")
                            .header("x-api-key", api_key)
                            .header("content-type", "application/json")
                            .json(&create_favorite)
                            .send()
                            .await
                    }
                    FavoriteOperation::Remove(fav_id) => {
                        client
                            .delete(&format!("https://api.thecatapi.com/v1/favourites/{}", fav_id))
                            .header("x-api-key", api_key)
                            .send()
                            .await
                    }
                }
            }
        }).collect();
        
        // Execute all requests concurrently
        let results = futures::future::join_all(futures).await;
        
        // Log results
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(response) if response.status().is_success() => {
                    println!("Batch operation {} completed successfully", i);
                }
                Ok(response) => {
                    println!("Batch operation {} failed with status: {}", i, response.status());
                }
                Err(e) => {
                    println!("Batch operation {} failed with error: {}", i, e);
                }
            }
        }
    }
}
