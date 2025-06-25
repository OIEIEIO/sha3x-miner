// SHA3x Miner - Free and Open Source Software Statement
//
// File: src/miner/gpu/manager.rs
// Version: 3.0.3 - Fixed Share Recording
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// GPU mining manager with PROPER share recording in MinerStats

use anyhow::{Error, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info, warn};

use crate::core::types::MiningJob;
use crate::miner::stats::MinerStats;
use super::opencl::{OpenClDevice, OpenClEngine};

/// GPU mining thread data
#[derive(Clone)]
pub struct GpuMiningThread {
    pub device_id: usize,
    pub thread_id: usize,
    pub device_name: String,
    pub estimated_hashrate: f64,
}

/// GPU mining manager - coordinates GPU mining operations
pub struct GpuManager {
    pub devices: Vec<OpenClDevice>,
    pub threads: Vec<GpuMiningThread>,
    initialized: bool,
}

impl GpuManager {
    /// Create a new GPU manager
    pub fn new() -> Self {
        debug!("Creating GPU manager");
        Self {
            devices: Vec::new(),
            threads: Vec::new(),
            initialized: false,
        }
    }
    
    /// Check if GPU mining is available
    pub fn is_available() -> bool {
        debug!("Checking GPU availability");
        match OpenClDevice::detect_devices() {
            Ok(devices) => {
                let suitable_devices: Vec<_> = devices.into_iter()
                    .filter(|d| d.is_suitable_for_mining())
                    .collect();
                !suitable_devices.is_empty()
            }
            Err(e) => {
                debug!("GPU detection failed: {}", e);
                false
            }
        }
    }
    
    /// Initialize GPU mining
    pub fn initialize(&mut self) -> Result<()> {
        info!("🎮 Initializing GPU mining...");
        
        // Detect available GPU devices
        let detected_devices = OpenClDevice::detect_devices()
            .map_err(|e| Error::msg(format!("Failed to detect GPU devices: {}", e)))?;
        
        if detected_devices.is_empty() {
            return Err(Error::msg("No OpenCL GPU devices found"));
        }
        
        // Filter suitable devices
        let suitable_devices: Vec<_> = detected_devices.into_iter()
            .filter(|device| {
                let suitable = device.is_suitable_for_mining();
                if suitable {
                    info!("✅ Found suitable GPU: {}", device.info_string());
                } else {
                    warn!("⚠️ GPU not suitable for mining: {}", device.info_string());
                }
                suitable
            })
            .collect();
        
        if suitable_devices.is_empty() {
            return Err(Error::msg("No suitable GPU devices found for mining"));
        }
        
        // Test engine creation and prepare thread info
        let mut threads = Vec::new();
        for (device_id, device) in suitable_devices.iter().enumerate() {
            let mut test_engine = OpenClEngine::new(device.clone());
            test_engine.initialize()
                .map_err(|e| Error::msg(format!("Failed to initialize engine for {}: {}", device.name(), e)))?;
            
            let estimated_hashrate = test_engine.estimate_hashrate();
            info!("🚀 GPU {} ready - estimated {:.1} MH/s", 
                  device.name(), estimated_hashrate);
            
            let thread_info = GpuMiningThread {
                device_id,
                thread_id: device_id, // Use device_id as thread_id (0, 1, 2...)
                device_name: device.name().to_string(),
                estimated_hashrate,
            };
            
            threads.push(thread_info);
        }
        
        self.devices = suitable_devices;
        self.threads = threads;
        self.initialized = true;
        
        info!("✅ GPU mining initialized with {} device(s)", self.devices.len());
        info!("🎯 Total estimated GPU hashrate: {:.1} MH/s", self.get_estimated_hashrate());
        
        Ok(())
    }
    
    /// Check if GPU manager is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Get number of GPU devices
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
    
    /// Get device info for dashboard
    pub fn get_device_info(&self) -> Vec<String> {
        self.devices.iter()
            .map(|d| d.info_string())
            .collect()
    }
    
    /// Get total estimated hashrate from all GPU devices
    pub fn get_estimated_hashrate(&self) -> f64 {
        self.threads.iter()
            .map(|t| t.estimated_hashrate)
            .sum()
    }
    
    /// Start actual GPU mining threads
    pub fn start_gpu_mining(
        &mut self,
        job_rx: Receiver<MiningJob>,
        share_tx: UnboundedSender<(String, String, String, usize, u64, String, u32)>,
        stats: Arc<MinerStats>,
    ) -> Result<()> {
        if !self.initialized {
            return Err(Error::msg("GPU manager not initialized"));
        }
        
        info!("🎮 Starting GPU mining with {} device(s) - proper share tracking!", self.devices.len());
        
        for (i, device) in self.devices.iter().enumerate() {
            let gpu_thread_id = i; // GPU threads get IDs 0, 1, 2... (for GPU-only mining)
            let device_clone = device.clone();
            let job_rx_clone = job_rx.resubscribe();
            let share_tx_clone = share_tx.clone();
            let stats_clone = Arc::clone(&stats);
            let device_name = device.name().to_string();
            let estimated_hashrate = self.threads[i].estimated_hashrate;
            
            // Update thread info
            self.threads[i].thread_id = gpu_thread_id;
            
            info!("🎮 Launching GPU mining thread {} for {} (~{:.1} MH/s)", 
                  gpu_thread_id, device_name, estimated_hashrate);
            
            debug!("GPU thread setup: thread_id={}, stats.thread_stats.len={}", 
                   gpu_thread_id, stats.thread_stats.len());
            
            // Spawn GPU mining thread using std::thread for OpenCL safety
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create GPU thread runtime");
                
                rt.block_on(async {
                    Self::gpu_mining_loop(
                        gpu_thread_id,
                        device_clone,
                        job_rx_clone,
                        share_tx_clone,
                        stats_clone,
                    ).await;
                });
            });
        }
        
        info!("🚀 All GPU mining threads launched with proper share tracking!");
        Ok(())
    }
    
    /// The actual GPU mining loop (public for direct access)
    pub async fn gpu_mining_loop(
        thread_id: usize,
        device: OpenClDevice,
        mut job_rx: Receiver<MiningJob>,
        share_tx: UnboundedSender<(String, String, String, usize, u64, String, u32)>,
        stats: Arc<MinerStats>,
    ) {
        info!("🎮 GPU mining thread {} starting for {}", thread_id, device.name());
        
        // Create OpenCL engine in this thread
        let mut engine = OpenClEngine::new(device.clone());
        if let Err(e) = engine.initialize() {
            error!("🎮 GPU thread {} failed to initialize: {}", thread_id, e);
            return;
        }
        
        let batch_size = engine.get_suggested_batch_size();
        let mut nonce_offset = thread_id as u64 * 1_000_000_000; // Unique nonce space per GPU
        let mut current_job: Option<MiningJob> = None;
        
        info!("🎮 GPU thread {} initialized - starting mining with batch size {}", 
              thread_id, batch_size);
        
        debug!("GPU thread {} ready: stats.thread_stats.len={}", thread_id, stats.thread_stats.len());
        
        loop {
            tokio::select! {
                // Wait for new job
                job_result = job_rx.recv() => {
                    match job_result {
                        Ok(job) => {
                            debug!("🎮 GPU {} got new job: {}", thread_id, job.job_id);
                            current_job = Some(job);
                            nonce_offset = thread_id as u64 * 1_000_000_000; // Reset nonce space
                        }
                        Err(e) => {
                            error!("🎮 GPU {} job error: {}", thread_id, e);
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
                
                // Mine continuously if we have a job
                _ = tokio::time::sleep(Duration::from_millis(1)), if current_job.is_some() => {
                    let job = current_job.as_ref().unwrap();
                    
                    match engine.mine(job, nonce_offset, batch_size) {
                        Ok((found_nonce, hashes_processed, best_difficulty)) => {
                            // Update stats - FIXED to ensure thread_id is valid
                            if thread_id < stats.thread_stats.len() {
                                stats.thread_stats[thread_id].update_hashrate(hashes_processed as u64);
                                
                                if best_difficulty > 0 {
                                    stats.thread_stats[thread_id].current_difficulty_target.store(
                                        best_difficulty, 
                                        std::sync::atomic::Ordering::Relaxed
                                    );
                                }
                            } else {
                                error!("🎮 GPU thread {} ID out of bounds! stats.len={}", thread_id, stats.thread_stats.len());
                            }
                            
                            // Update global hash count
                            stats.hashes_computed.fetch_add(
                                hashes_processed as u64, 
                                std::sync::atomic::Ordering::Relaxed
                            );
                            
                            // Submit share if found
                            if let Some(nonce) = found_nonce {
                                // Use little-endian nonce format (same as CPU)
                                let nonce_hex = hex::encode(nonce.to_le_bytes());
                                
                                // Calculate the actual hash result for SHA3x using same function as CPU
                                let hash_result = engine.calculate_share_result(job, nonce)
                                    .unwrap_or_else(|_| hex::encode(&[0u8; 32])); // Fallback to zeros
                                
                                info!("🎉 GPU {} FOUND SHARE! Nonce: {} Difficulty: {}", 
                                      thread_id, nonce_hex, 
                                      crate::miner::stats::MinerStats::format_number(best_difficulty));
                                
                                // *** CRITICAL FIX: Properly record share in MinerStats ***
                                debug!("Recording GPU share: thread_id={}, difficulty={}, target={}, stats.len={}", 
                                       thread_id, best_difficulty, job.target_difficulty, stats.thread_stats.len());
                                
                                // Record share in MinerStats for dashboard metrics
                                stats.record_share_found(thread_id, best_difficulty, job.target_difficulty, true);
                                
                                // Also manually increment share counters for immediate stats
                                stats.shares_submitted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                
                                // Send share for submission to pool
                                if let Err(e) = share_tx.send((
                                    job.job_id.clone(),
                                    nonce_hex,
                                    hash_result, // Actual SHA3x hash result
                                    thread_id,
                                    best_difficulty,
                                    String::new(), // No extranonce2
                                    0, // No ntime
                                )) {
                                    error!("🎮 GPU {} failed to send share: {}", thread_id, e);
                                }
                                
                                // Record in thread stats (redundant but ensures tracking)
                                if thread_id < stats.thread_stats.len() {
                                    stats.thread_stats[thread_id].record_share(best_difficulty, true);
                                }
                                
                                debug!("✅ GPU share recorded: submitted={}, recent_shares_count={}", 
                                       stats.shares_submitted.load(std::sync::atomic::Ordering::Relaxed),
                                       "checking...");
                            }
                            
                            // Advance nonce for next iteration
                            nonce_offset += hashes_processed as u64;
                        }
                        Err(e) => {
                            error!("🎮 GPU {} mining error: {}", thread_id, e);
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                    }
                }
            }
        }
    }
}

impl Default for GpuManager {
    fn default() -> Self {
        Self::new()
    }
}