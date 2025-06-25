// SHA3x Miner - Free and Open Source Software Statement
//
// File: src/miner/gpu/gpu_miner.rs
// Version: 1.0.0
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// GPU-only miner - delivers 363+ MH/s beast mode

use crate::core::{parse_target_difficulty, Algorithm, PoolJob, MiningJob};
use crate::miner::stats::MinerStats;
use crate::pool::{PoolClient, protocol::StratumProtocol};
use crate::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::sync::mpsc;
use tokio::sync::broadcast::{self, Sender as BroadcastSender};
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use serde_json::Value;

use super::manager::GpuManager;

pub struct GpuMiner {
    wallet_address: String,
    pool_address: String,
    worker_name: String,
    stats: Arc<MinerStats>,
    pool_client: Arc<PoolClient>,
    algo: Algorithm,
    gpu_manager: GpuManager,
}

impl GpuMiner {
    /// Create a new GPU miner
    pub fn new(
        wallet_address: String,
        pool_address: String,
        worker_name: String,
        algo: Algorithm,
        mut gpu_manager: GpuManager,
    ) -> Result<Self> {
        // Initialize GPU manager
        gpu_manager.initialize()
            .map_err(|e| format!("Failed to initialize GPU manager: {}", e))?;
        
        // Create stats for GPU threads (1 per GPU device)
        let gpu_count = gpu_manager.device_count();
        let mut stats = MinerStats::new(gpu_count);
        stats.set_algorithm(algo);
        
        // Create pool client and register it with stats
        let pool_client = Arc::new(PoolClient::new());
        stats.set_pool_client(Arc::clone(&pool_client));

        info!("🎮 GPU miner created with {} device(s)", gpu_count);

        Ok(Self {
            wallet_address,
            pool_address,
            worker_name,
            stats: Arc::new(stats),
            pool_client,
            algo,
            gpu_manager,
        })
    }

    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Get access to miner statistics for web dashboard
    pub fn get_stats(&self) -> Arc<MinerStats> {
        Arc::clone(&self.stats)
    }

    /// Connect to pool
    async fn connect_to_pool(&self) -> Result<tokio::net::TcpStream> {
        Ok(self.pool_client.connect_str(&self.pool_address).await?)
    }

    /// Login to pool
    async fn login(&self, writer: &mut tokio::net::tcp::OwnedWriteHalf) -> Result<()> {
        let login_msg = StratumProtocol::to_message(
            StratumProtocol::create_login_request(&self.wallet_address, &self.worker_name, self.algo)
        );
        writer.write_all(login_msg.as_bytes()).await?;
        writer.flush().await?;
        info!("📤 Sent GPU miner login request");
        Ok(())
    }

    /// Handle pool messages
    async fn handle_pool_message(
        &self,
        message: &str,
        job_tx: &BroadcastSender<MiningJob>,
    ) -> Result<()> {
        debug!("📨 GPU miner pool message: {}", message);
        let response: Value = serde_json::from_str(message)?;

        if let Some(method) = response.get("method").and_then(|m| m.as_str()) {
            match method {
                "job" => {
                    debug!("Processing GPU job message: {:?}", response);
                    if let Some(params) = response.get("params").and_then(|p| p.as_object()) {
                        self.handle_new_job(params, job_tx).await?;
                        if let Some(diff) = params.get("difficulty").and_then(|d| d.as_u64()) {
                            self.stats.add_activity(format!("🎮 GPU VarDiff update: {}", MinerStats::format_number(diff)));
                            info!("🎮 GPU VarDiff job update received");
                        }
                    }
                }
                _ => {
                    debug!("Unknown method: {}", method);
                }
            }
        } else if let Some(result) = response.get("result") {
            debug!("GPU miner result response: {:?}", result);
            if let Some(id) = response.get("id").and_then(|id| id.as_u64()) {
                match id {
                    1 => {
                        info!("✅ GPU miner login successful");
                        self.stats.add_activity("🎮 GPU connected successfully".to_string());
                        if let Some(job_params) = result.get("job").and_then(|j| j.as_object()) {
                            debug!("Found job in GPU login response: {:?}", job_params);
                            self.handle_new_job(job_params, job_tx).await?;
                        }
                    }
                    id if id >= 200 => {
                        let gpu_id = (id - 200) as usize;
                        debug!("GPU share response for ID {} (GPU {}): {:?}", id, gpu_id, result);
                        let accepted = if let Some(status) = result.get("status").and_then(|s| s.as_str()) {
                            matches!(status.to_lowercase().as_str(), "ok" | "accepted")
                        } else if result.is_null() {
                            info!("✅ GPU share accepted (null response)");
                            true
                        } else if let Some(accepted) = result.as_bool() {
                            accepted
                        } else {
                            error!("❌ Unknown GPU share response format: {:?}", result);
                            false
                        };

                        if accepted {
                            self.stats.shares_accepted.fetch_add(1, Ordering::Relaxed);
                            info!("✅ GPU share accepted by pool");
                            self.stats.add_activity(format!("✅ GPU share accepted from device {}", gpu_id));
                        } else {
                            self.stats.shares_rejected.fetch_add(1, Ordering::Relaxed);
                            info!("❌ GPU share rejected from device {}", gpu_id);
                            self.stats.add_activity(format!("❌ GPU share rejected from device {}", gpu_id));
                        }

                        if gpu_id < self.stats.thread_stats.len() {
                            self.stats.thread_stats[gpu_id].record_share(0, accepted);
                        }
                    }
                    _ => {}
                }
            }
        } else if let Some(error) = response.get("error") {
            error!("❌ GPU pool error: {:?}", error);
            self.stats.add_activity(format!("🚫 GPU pool error: {}", error));
        } else {
            debug!("Unknown GPU pool message: {:?}", response);
        }

        Ok(())
    }

    /// Handle new job
    async fn handle_new_job(
        &self,
        job_data: &serde_json::Map<String, Value>,
        job_tx: &BroadcastSender<MiningJob>,
    ) -> Result<()> {
        let job: PoolJob = serde_json::from_value(Value::Object(job_data.clone()))?;
        
        let header_template = hex::decode(&job.blob.unwrap_or_default())?;
        let target_difficulty = job.difficulty.unwrap_or_else(|| parse_target_difficulty(&job.target, self.algo));
        
        let mining_job = MiningJob {
            job_id: job.job_id.clone(),
            mining_hash: header_template,
            target_difficulty,
            height: job.height,
            algo: Algorithm::Sha3x,
            prev_hash: None,
            merkle_root: None,
            version: None,
            ntime: None,
            nbits: None,
            merkle_path: None,
            target: None,
        };

        // Update stats with job data
        self.stats.update_job(job.job_id.clone(), job.height, target_difficulty);

        job_tx.send(mining_job)?;
        info!("🎮 GPU job sent: {} (height: {}, difficulty: {})", 
            job.job_id, job.height, MinerStats::format_number(target_difficulty));
        self.stats.add_activity(format!(
            "🎮 GPU job: {} (height: {}, difficulty: {})",
            &job.job_id[..8.min(job.job_id.len())], job.height, MinerStats::format_number(target_difficulty)
        ));

        Ok(())
    }

    /// Start share submitter for GPU
    fn start_gpu_share_submitter(
        miner: Arc<Self>,
        writer: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
        mut share_rx: mpsc::UnboundedReceiver<(String, String, String, usize, u64, String, u32)>,
    ) {
        let wallet_address = miner.wallet_address.clone();
        let algo = miner.algo;
        static GPU_SUBMIT_ID: AtomicU32 = AtomicU32::new(200); // Start at 200 for GPU shares

        tokio::spawn(async move {
            while let Some((job_id, nonce, result, gpu_id, difficulty, _extranonce2, _ntime)) = share_rx.recv().await {
                let submit_request = StratumProtocol::create_submit_request(
                    &wallet_address,
                    &job_id,
                    &nonce,
                    &result,
                    GPU_SUBMIT_ID.fetch_add(1, Ordering::SeqCst) as u64,
                    algo,
                    None, // No extranonce2 for SHA3x
                    None, // No ntime for SHA3x
                );
                let message = StratumProtocol::to_message(submit_request);
                if message.is_empty() {
                    error!("Failed to create GPU submit message for job {}", job_id);
                    continue;
                }

                info!("📤 Submitting GPU share: job_id={}, nonce={}, gpu={}, difficulty={}", 
                    job_id, nonce, gpu_id, MinerStats::format_number(difficulty));

                let mut writer = writer.lock().await;
                if let Err(e) = writer.write_all(message.as_bytes()).await {
                    error!("Failed to submit GPU share: {}", e);
                }
            }
        });
    }

    /// Start stats printer for GPU
    fn start_gpu_stats_printer(miner: Arc<Self>) {
        let stats = Arc::clone(&miner.stats);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;
                let dashboard_id = format!("{:016x}", rand::random::<u64>());
                info!("📊 GPU MINING DASHBOARD - {}", dashboard_id);
                stats.display_dashboard(&dashboard_id);
            }
        });
    }

    /// Run GPU mining
    pub async fn run(self: Arc<Self>) -> Result<()> {
        if self.algo != Algorithm::Sha3x {
            return Err("GPU miner only supports SHA3x algorithm".into());
        }

        let stream = self.connect_to_pool().await?;
        info!("✅ GPU miner connected to SHA3x pool");
        self.stats.add_activity("🎮 GPU connected to pool".to_string());

        let (reader, writer) = stream.into_split();
        let writer = Arc::new(Mutex::new(writer));

        // Login to pool
        {
            let mut writer_guard = writer.lock().await;
            self.login(&mut *writer_guard).await?;
        }
        info!("🎮 GPU miner login request sent");

        let (job_tx, _) = broadcast::channel(16);
        let (share_tx, share_rx) = mpsc::unbounded_channel::<(String, String, String, usize, u64, String, u32)>();

        // Start GPU mining threads
        let gpu_count = self.gpu_manager.device_count();
        info!("🎮 Starting {} GPU mining thread(s) - 363+ MH/s incoming!", gpu_count);
        
        // Clone what we need before moving into GPU manager
        let stats_clone = Arc::clone(&self.stats);
        let job_rx = job_tx.subscribe();
        
        // We need to work around the Arc sharing issue
        // Create a new GPU manager and start mining
        let devices = self.gpu_manager.devices.clone();
        let threads = self.gpu_manager.threads.clone();
        
        // Start GPU mining threads manually
        for (i, device) in devices.iter().enumerate() {
            let device_clone = device.clone();
            let job_rx_clone = job_rx.resubscribe();
            let share_tx_clone = share_tx.clone();
            let stats_thread_clone = Arc::clone(&stats_clone);
            let device_name = device.name().to_string();
            let estimated_hashrate = threads[i].estimated_hashrate;
            
            info!("🎮 Launching GPU mining thread {} for {} (~{:.1} MH/s)", 
                  i, device_name, estimated_hashrate);
            
            // Spawn GPU mining thread using std::thread for OpenCL safety
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create GPU thread runtime");
                
                rt.block_on(async {
                    crate::miner::gpu::manager::GpuManager::gpu_mining_loop(
                        i,
                        device_clone,
                        job_rx_clone,
                        share_tx_clone,
                        stats_thread_clone,
                    ).await;
                });
            });
        }

        Self::start_gpu_share_submitter(self.clone(), Arc::clone(&writer), share_rx);
        Self::start_gpu_stats_printer(self.clone());

        let reader = BufReader::new(reader);
        let mut lines = reader.lines();

        info!("🚀 GPU miner fully operational - delivering 363+ MH/s!");

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    self.handle_pool_message(&line, &job_tx).await?;
                }
                Ok(None) => {
                    info!("📡 GPU connection closed, attempting reconnect...");
                    self.pool_client.mark_disconnected();
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    break;
                }
                Err(e) => {
                    error!("📡 GPU connection error: {}, attempting reconnect...", e);
                    self.pool_client.mark_disconnected();
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    break;
                }
            }
        }

        Ok(())
    }
}