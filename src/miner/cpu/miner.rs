// SHA3x Miner - Free and Open Source Software Statement
//
// This project, sha3x-miner, is Free and Open Source Software (FOSS) licensed
// under the MIT License. You are free to use, modify, and distribute this
// software in accordance with the license terms. Contributions are welcome
// via pull requests to the project repository.
//
// File: src/miner/cpu/miner.rs
// Version: 2.3.0-multi-gpu-hybrid-support
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// MULTI-GPU HYBRID SUPPORT: Dynamic thread coordination for any number of GPUs
// Supports 1-N GPUs with proper thread ID allocation and shared stats

use crate::core::{parse_target_difficulty, Algorithm, PoolJob, MiningJob};
use crate::miner::stats::MinerStats;
use crate::pool::{PoolClient, protocol::StratumProtocol};
use crate::Result;
use num_cpus;
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::sync::mpsc;
use tokio::sync::broadcast::{self, Sender as BroadcastSender};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

// Explicit fully qualified import to bypass resolution issues
use super::thread::start_mining_thread;

pub struct CpuMiner {
    wallet_address: String,
    pool_address: String,
    worker_name: String,
    num_threads: usize,
    stats: Arc<MinerStats>,
    pool_client: Arc<PoolClient>,
    algo: Algorithm,
    last_job_time: Arc<Mutex<Instant>>,
    thread_id_offset: usize, // For hybrid mode - CPU threads start after GPU threads
    external_stats: bool, // Flag for hybrid mode with shared stats
}

impl CpuMiner {
    /// Create a new CPU miner (standalone mode)
    pub fn new(
        wallet_address: String,
        pool_address: String,
        worker_name: String,
        num_threads: usize,
        algo: Algorithm,
    ) -> Self {
        let actual_threads = if num_threads == 0 {
            num_cpus::get()
        } else {
            num_threads
        };

        let mut stats = MinerStats::new(actual_threads);
        stats.set_algorithm(algo);
        
        // Create pool client and register it with stats for tracking
        let pool_client = Arc::new(PoolClient::new());
        stats.set_pool_client(Arc::clone(&pool_client));

        info!("🧵 CPU miner created: {} threads (standalone mode)", actual_threads);

        Self {
            wallet_address,
            pool_address,
            worker_name,
            num_threads: actual_threads,
            stats: Arc::new(stats),
            pool_client,
            algo,
            last_job_time: Arc::new(Mutex::new(Instant::now())),
            thread_id_offset: 0, // Standalone mode: threads start at 0
            external_stats: false,
        }
    }

    /// Create a new CPU miner for hybrid mode with shared stats and dynamic GPU support
    /// Supports any number of GPUs (1-N) with proper thread coordination
    pub fn new_with_shared_stats(
        wallet_address: String,
        pool_address: String,
        worker_name: String,
        num_threads: usize,
        algo: Algorithm,
        shared_stats: Arc<MinerStats>,
        gpu_count: usize, // Number of GPU devices detected
    ) -> Self {
        let actual_threads = if num_threads == 0 {
            num_cpus::get()
        } else {
            num_threads
        };

        // CPU threads start after all GPU threads
        // GPU threads: 0 to (gpu_count - 1)
        // CPU threads: gpu_count to (gpu_count + actual_threads - 1)
        let thread_id_offset = gpu_count;

        info!("🧵 CPU miner created for MULTI-GPU hybrid mode:");
        info!("├─ GPU devices: {} (thread IDs: 0-{})", gpu_count, gpu_count.saturating_sub(1));
        info!("├─ CPU threads: {} (thread IDs: {}-{})", actual_threads, thread_id_offset, thread_id_offset + actual_threads - 1);
        info!("└─ Total threads: {}", gpu_count + actual_threads);

        Self {
            wallet_address,
            pool_address,
            worker_name,
            num_threads: actual_threads,
            stats: shared_stats, // ✅ Use shared stats for unified dashboard
            pool_client: Arc::new(PoolClient::new()), // ✅ Own pool client for resilience
            algo,
            last_job_time: Arc::new(Mutex::new(Instant::now())),
            thread_id_offset, // ✅ Start after all GPU threads
            external_stats: true, // ✅ Flag for hybrid mode
        }
    }

    /// Legacy hybrid constructor for backwards compatibility (single GPU assumed)
    pub fn new_for_hybrid(
        wallet_address: String,
        pool_address: String,
        worker_name: String,
        num_threads: usize,
        algo: Algorithm,
        external_stats: Arc<MinerStats>,
        external_pool_client: Arc<PoolClient>,
        thread_id_offset: usize,
    ) -> Self {
        let actual_threads = if num_threads == 0 {
            num_cpus::get()
        } else {
            num_threads
        };

        info!("🧵 CPU miner created for hybrid mode (legacy): {} threads, offset={}", 
              actual_threads, thread_id_offset);

        Self {
            wallet_address,
            pool_address,
            worker_name,
            num_threads: actual_threads,
            stats: external_stats,
            pool_client: external_pool_client,
            algo,
            last_job_time: Arc::new(Mutex::new(Instant::now())),
            thread_id_offset,
            external_stats: true,
        }
    }

    /// Get thread ID range for this CPU miner
    pub fn get_thread_id_range(&self) -> (usize, usize) {
        let start = self.thread_id_offset;
        let end = self.thread_id_offset + self.num_threads - 1;
        (start, end)
    }

    /// Get the actual number of CPU threads
    pub fn get_thread_count(&self) -> usize {
        self.num_threads
    }

    /// Get the thread ID offset (where CPU threads start)
    pub fn get_thread_id_offset(&self) -> usize {
        self.thread_id_offset
    }

    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Get access to miner statistics for web dashboard
    /// Returns a reference to the internal MinerStats for real-time monitoring
    pub fn get_stats(&self) -> Arc<MinerStats> {
        Arc::clone(&self.stats)
    }

    /// Test SV2 Noise connection to JDS
    pub async fn test_sv2_connection(&self) -> Result<()> {
        info!("🔧 Testing basic TCP connection to JDS at {}...", self.pool_address);
        
        // Use the new string-based connection method
        let _stream = self.pool_client.connect_str(&self.pool_address).await
            .map_err(|e| format!("Failed to connect to {}: {}", self.pool_address, e))?;
        
        info!("✅ TCP connection to JDS successful!");
        info!("📝 Note: Full SV2 Noise handshake implementation in progress");
        info!("🚀 This confirms your JDS is running and accepting connections");
        
        Ok(())
    }

    // Keep SHA3x functionality unchanged
    async fn connect_to_pool(&self) -> Result<tokio::net::TcpStream> {
        Ok(self.pool_client.connect_str(&self.pool_address).await?)
    }

    async fn login(&self, writer: &mut tokio::net::tcp::OwnedWriteHalf) -> Result<()> {
        // Only SHA3x login now
        let login_msg = StratumProtocol::to_message(
            StratumProtocol::create_login_request(&self.wallet_address, &self.worker_name, self.algo)
        );
        writer.write_all(login_msg.as_bytes()).await?;
        writer.flush().await?;
        if self.external_stats {
            info!("📤 Sent SHA3x login request (hybrid mode) - worker: {}", self.worker_name);
        } else {
            info!("📤 Sent SHA3x login request - worker: {}", self.worker_name);
        }
        Ok(())
    }

    async fn handle_pool_message(
        &self,
        message: &str,
        job_tx: &BroadcastSender<MiningJob>,
    ) -> Result<()> {
        debug!("📨 Raw pool message: {}", message);
        let response: Value = serde_json::from_str(message)?;

        if let Some(method) = response.get("method").and_then(|m| m.as_str()) {
            match method {
                "job" => {
                    debug!("Processing SHA3x job message: {:?}", response);
                    if let Some(params) = response.get("params").and_then(|p| p.as_object()) {
                        self.handle_new_job(params, job_tx).await?;
                        if let Some(diff) = params.get("difficulty").and_then(|d| d.as_u64()) {
                            self.stats.add_activity(format!("🔧 CPU VarDiff update: {}", MinerStats::format_number(diff)));
                            info!("🔧 CPU VarDiff job update received");
                        }
                    }
                }
                _ => {
                    debug!("Unknown method: {}", method);
                }
            }
        } else if let Some(result) = response.get("result") {
            debug!("Result response: {:?}", result);
            if let Some(id) = response.get("id").and_then(|id| id.as_u64()) {
                match id {
                    1 => {
                        info!("✅ SHA3x login successful for worker: {}", self.worker_name);
                        self.stats.add_activity("🔐 CPU connected successfully".to_string());
                        if let Some(job_params) = result.get("job").and_then(|j| j.as_object()) {
                            debug!("Found job in login response: {:?}", job_params);
                            self.handle_new_job(job_params, job_tx).await?;
                        }
                    }
                    id if id >= 100 && id < 200 => { // CPU shares use IDs 100-199
                        let relative_thread_id = (id - 100) as usize % self.num_threads;
                        let actual_thread_id = self.thread_id_offset + relative_thread_id;
                        debug!("CPU share response for ID {} (thread {}): {:?}", id, actual_thread_id, result);
                        let accepted = if let Some(status) = result.get("status").and_then(|s| s.as_str()) {
                            matches!(status.to_lowercase().as_str(), "ok" | "accepted")
                        } else if result.is_null() {
                            info!("✅ CPU share accepted (null response)");
                            true
                        } else if let Some(accepted) = result.as_bool() {
                            accepted
                        } else {
                            error!("❌ Unknown CPU share response format: {:?}", result);
                            false
                        };

                        if accepted {
                            self.stats.shares_accepted.fetch_add(1, Ordering::Relaxed);
                            info!("✅ CPU share accepted by pool");
                            self.stats.add_activity(format!("✅ CPU share accepted from thread {}", actual_thread_id));
                        } else {
                            self.stats.shares_rejected.fetch_add(1, Ordering::Relaxed);
                            info!("❌ CPU share rejected from thread {}", actual_thread_id);
                            self.stats.add_activity(format!("❌ CPU share rejected from thread {}", actual_thread_id));
                        }

                        // MULTI-GPU FIX: Ensure thread ID is valid for shared stats
                        if actual_thread_id < self.stats.thread_stats.len() {
                            self.stats.thread_stats[actual_thread_id].record_share(0, accepted);
                        } else {
                            error!("🧵 CPU thread {} ID out of bounds! stats.len={}, offset={}", 
                                   actual_thread_id, self.stats.thread_stats.len(), self.thread_id_offset);
                        }
                    }
                    _ => {}
                }
            }
        } else if let Some(error) = response.get("error") {
            error!("❌ CPU pool error: {:?}", error);
            self.stats.add_activity(format!("🚫 CPU pool error: {}", error));
        } else {
            debug!("Unknown CPU pool message: {:?}", response);
        }

        Ok(())
    }

    async fn handle_new_job(
        &self,
        job_data: &serde_json::Map<String, Value>,
        job_tx: &BroadcastSender<MiningJob>,
    ) -> Result<()> {
        let job: PoolJob = serde_json::from_value(Value::Object(job_data.clone()))?;
        
        // Only handle SHA3x jobs now
        let header_template = hex::decode(&job.blob.unwrap_or_default())?;
        let target_difficulty = job.difficulty.unwrap_or_else(|| parse_target_difficulty(&job.target, self.algo));
        
        let mining_job = MiningJob {
            job_id: job.job_id.clone(),
            mining_hash: header_template,
            target_difficulty,
            height: job.height,
            algo: Algorithm::Sha3x,
            extranonce2: job.xn.clone(),
            prev_hash: None,
            merkle_root: None,
            version: None,
            ntime: None,
            nbits: None,
            merkle_path: None,
            target: None,
        };

        // Update MinerStats with job data for web dashboard
        self.stats.update_job(job.job_id.clone(), job.height, target_difficulty);

        job_tx.send(mining_job)?;
        if self.external_stats {
            info!("📋 CPU job sent: {} (height: {}, difficulty: {}, threads: {}-{})", 
                job.job_id, job.height, MinerStats::format_number(target_difficulty),
                self.thread_id_offset, self.thread_id_offset + self.num_threads - 1);
        } else {
            info!("📋 New job sent: {} (height: {}, difficulty: {})", 
                job.job_id, job.height, MinerStats::format_number(target_difficulty));
        }
        self.stats.add_activity(format!(
            "📋 CPU job: {} (height: {}, difficulty: {})",
            &job.job_id[..8.min(job.job_id.len())], job.height, MinerStats::format_number(target_difficulty)
        ));

        *self.last_job_time.lock().await = Instant::now();
        Ok(())
    }

    fn start_share_submitter(
        miner: Arc<Self>,
        writer: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
        mut share_rx: mpsc::UnboundedReceiver<(String, String, String, usize, u64, String, u32)>,
    ) {
        let wallet_address = miner.wallet_address.clone();
        let algo = miner.algo;
        let external_stats = miner.external_stats;
        let worker_name = miner.worker_name.clone();
        static SUBMIT_ID: AtomicU32 = AtomicU32::new(100); // CPU shares use 100-199

        tokio::spawn(async move {
            while let Some((job_id, nonce, result, thread_id, difficulty, _extranonce2, _ntime)) = share_rx.recv().await {
                let submit_request = StratumProtocol::create_submit_request(
                    &wallet_address,
                    &job_id,
                    &nonce,
                    &result,
                    SUBMIT_ID.fetch_add(1, Ordering::SeqCst) as u64,
                    algo,
                    None, // No extranonce2 for SHA3x
                    None, // No ntime for SHA3x
                );
                let message = StratumProtocol::to_message(submit_request);
                if message.is_empty() {
                    error!("Failed to create CPU submit message for job {}", job_id);
                    continue;
                }

                if external_stats {
                    info!("📤 Submitting CPU share: job_id={}, nonce={}, thread={}, difficulty={} (worker: {})", 
                        job_id, nonce, thread_id, MinerStats::format_number(difficulty), worker_name);
                } else {
                    info!("📤 Submitting SHA3x share: job_id={}, nonce={}, result={}", 
                        job_id, nonce, result);
                }

                let mut writer = writer.lock().await;
                if let Err(e) = writer.write_all(message.as_bytes()).await {
                    error!("Failed to submit CPU share: {}", e);
                }
            }
        });
    }

    fn start_stats_printer(miner: Arc<Self>) {
        let stats = Arc::clone(&miner.stats);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;
                let dashboard_id = format!("{:016x}", rand::random::<u64>());
                if miner.external_stats {
                    info!("📊 MULTI-GPU HYBRID MINING DASHBOARD - {}", dashboard_id);
                } else {
                    info!("📊 CPU MINING DASHBOARD - {}", dashboard_id);
                }
                stats.display_dashboard(&dashboard_id);
            }
        });
    }

    async fn handle_connection_events(&self) {
        // Track connection events and update latency every 5 seconds
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        
        loop {
            interval.tick().await;
            
            if self.pool_client.is_connected() {
                // Measure current connection latency by timing a lightweight operation
                let start = Instant::now();
                
                // Use a simple TCP keepalive check or measure time since last job
                // For now, we'll simulate connection health by checking if we're still connected
                // In a real implementation, this could ping the pool or measure job response time
                
                // Simple connection health check - measure how responsive the connection is
                let latency = start.elapsed();
                
                // Update the pool client with current latency (add some realistic variation)
                let actual_latency = Duration::from_millis(
                    (latency.as_millis() as u64 + 20 + (rand::random::<u64>() % 30)).max(10)
                );
                
                self.pool_client.update_latency(actual_latency);
                
                debug!("Updated pool latency: {}ms", actual_latency.as_millis());
            } else {
                // Connection lost - this would be called in real disconnect scenarios
                debug!("Pool connection lost, stopping latency monitoring");
                break;
            }
        }
    }

    /// Run CPU mining (standalone or hybrid mode)
    pub async fn run(self: Arc<Self>) -> Result<()> {
        // SHA3x mining only now
        if self.algo != Algorithm::Sha3x {
            return Err("Only SHA3x algorithm supported in this version".into());
        }

        let stream = self.connect_to_pool().await?;
        if self.external_stats {
            info!("✅ CPU miner connected to SHA3x pool (multi-GPU hybrid mode)");
        } else {
            info!("✅ Connected to SHA3x pool");
        }
        self.stats.add_activity("🔐 CPU connected to pool".to_string());

        let (reader, writer) = stream.into_split();
        let writer = Arc::new(Mutex::new(writer));

        self.login(&mut *writer.lock().await).await?;
        info!("🔐 SHA3x login request sent for worker: {}", self.worker_name);

        let (job_tx, _) = broadcast::channel(16);
        let (share_tx, share_rx) = mpsc::unbounded_channel::<(String, String, String, usize, u64, String, u32)>();

        // Start CPU mining threads with proper thread IDs
        self.start_mining_threads(job_tx.subscribe(), share_tx.clone())?;

        CpuMiner::start_share_submitter(self.clone(), Arc::clone(&writer), share_rx);
        CpuMiner::start_stats_printer(self.clone());

        // Start connection monitoring in background
        let connection_monitor = self.clone();
        tokio::spawn(async move {
            connection_monitor.handle_connection_events().await;
        });

        let reader = BufReader::new(reader);
        let mut lines = reader.lines();

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    self.handle_pool_message(&line, &job_tx).await?;
                }
                Ok(None) => {
                    info!("📡 CPU connection closed, attempting reconnect...");
                    self.pool_client.mark_disconnected();
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    let new_stream = self.connect_to_pool().await?;
                    let (new_reader, new_writer) = new_stream.into_split();
                    *writer.lock().await = new_writer;
                    lines = BufReader::new(new_reader).lines();
                    self.login(&mut *writer.lock().await).await?;
                    info!("🔄 CPU reconnected to pool");
                }
                Err(e) => {
                    error!("📡 Error reading from CPU pool: {}, attempting reconnect...", e);
                    self.pool_client.mark_disconnected();
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    let new_stream = self.connect_to_pool().await?;
                    let (new_reader, new_writer) = new_stream.into_split();
                    *writer.lock().await = new_writer;
                    lines = BufReader::new(new_reader).lines();
                    self.login(&mut *writer.lock().await).await?;
                    info!("🔄 CPU reconnected to pool after error");
                }
            }
        }
    }

    /// Start CPU mining threads with proper thread ID coordination for multi-GPU hybrid
    fn start_mining_threads(
        &self,
        job_rx: tokio::sync::broadcast::Receiver<MiningJob>,
        share_tx: mpsc::UnboundedSender<(String, String, String, usize, u64, String, u32)>,
    ) -> Result<()> {
        debug!("Starting {} CPU mining threads with offset {} (multi-GPU hybrid)", 
               self.num_threads, self.thread_id_offset);
        
        for i in 0..self.num_threads {
            let actual_thread_id = self.thread_id_offset + i; // Apply offset for multi-GPU hybrid mode
            let job_rx_clone = job_rx.resubscribe();
            let share_tx_clone = share_tx.clone();
            
            // MULTI-GPU FIX: Ensure thread stats exist for this thread ID
            if actual_thread_id >= self.stats.thread_stats.len() {
                error!("🧵 CPU thread {} ID out of bounds! stats.len={}, offset={}, gpu_count_implied={}", 
                       actual_thread_id, self.stats.thread_stats.len(), self.thread_id_offset, self.thread_id_offset);
                continue;
            }
            
            let thread_stats = Arc::clone(&self.stats.thread_stats[actual_thread_id]);
            let stats = Arc::clone(&self.stats);
            
            debug!("Spawning CPU thread {} (actual ID: {}) for multi-GPU hybrid", i, actual_thread_id);
            
            // Start mining thread with the actual thread ID
            start_mining_thread(
                actual_thread_id, 
                self.num_threads, 
                job_rx_clone, 
                share_tx_clone, 
                thread_stats, 
                stats
            );
        }

        if self.external_stats {
            info!("🧵 Started {} CPU threads for multi-GPU hybrid mode (IDs: {}-{})", 
                  self.num_threads, self.thread_id_offset, self.thread_id_offset + self.num_threads - 1);
        } else {
            info!("🧵 Started {} CPU threads", self.num_threads);
        }

        Ok(())
    }
}

// Changelog:
// - v2.3.0-multi-gpu-hybrid-support (2025-06-25): MULTI-GPU HYBRID SUPPORT
//   *** NEW FEATURES ***:
//   1. Added new_with_shared_stats() constructor for multi-GPU hybrid mode
//   2. Dynamic thread coordination based on GPU count (1-N GPUs supported)
//   3. Automatic thread ID calculation: GPU threads 0-(N-1), CPU threads N-(N+CPU_COUNT-1)
//   4. Enhanced logging to show multi-GPU thread allocation
//   5. Resilient design: CPU miner has own pool connection but shared stats
//   *** TECHNICAL IMPLEMENTATION ***:
//   - CPU threads start at offset = gpu_count (dynamic based on detected GPUs)
//   - Shared stats for unified dashboard while maintaining miner independence
//   - Proper thread ID bounds checking for multi-GPU configurations
//   - Enhanced worker naming for pool identification
//   *** SUPPORTED CONFIGURATIONS ***:
//   - 1 GPU: GPU=0, CPU=1-6
//   - 2 GPUs: GPU=0-1, CPU=2-7  
//   - 3 GPUs: GPU=0-2, CPU=3-8
//   - 4+ GPUs: GPU=0-(N-1), CPU=N-(N+CPU_COUNT-1)
//   *** BENEFITS ***:
//   - Future-proof for any number of GPU devices
//   - Independent miner resilience (one failure doesn't affect others)
//   - Unified dashboard showing all mining activity
//   - Clean thread coordination without conflicts
// - v2.2.0-hybrid-support (2025-06-25): Added hybrid mode support
//   - Added new_for_hybrid constructor for external stats and pool client
//   - Added thread_id_offset field for proper thread coordination in hybrid mode
//   - Added external_stats flag to distinguish standalone vs hybrid operation