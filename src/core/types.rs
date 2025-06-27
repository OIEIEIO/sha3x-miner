// SHA3x Miner - Free and Open Source Software Statement
//
// This project, sha3x-miner, is Free and Open Source Software (FOSS) licensed
// under the MIT License. You are free to use, modify, and distribute this
// software in accordance with the license terms. Contributions are welcome
// via pull requests to the project repository.
//
// File: src/core/types.rs
// Version: 1.1.4-luckypool-xn-support
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// This file defines core data structures for the SHA3x miner, located in the
// core subdirectory. It includes types for command-line arguments, pool jobs,
// mining jobs, shares, and related protocol structures, supporting SHA3x
// (Tari), SV2 testing, GPU controls, web dashboard functionality, LuckyPool
// address formats (solo:ADDRESS, ADDRESS=DIFF, ADDRESS.WORKER), and LuckyPool
// XN (extra nonce) field support for proper nonce generation.
//
// Tree Location:
// - src/core/types.rs (core data structures)
// - Depends on: clap, serde

use clap::Parser;
use serde::{Deserialize, Serialize};

/// Mining algorithm variants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Algorithm {
    Sha3x,
    Sha256,
}

/// GPU mining settings
#[derive(Debug, Clone)]
pub struct GpuSettings {
    /// GPU mining intensity (0-100%)
    pub intensity: u8,
    /// Override for automatic batch size calculation
    pub batch_size: Option<u32>,
    /// GPU power limit (50-110%)
    pub power_limit: Option<u8>,
    /// GPU temperature limit (60-85°C)
    pub temp_limit: Option<u8>,
}

impl Default for GpuSettings {
    fn default() -> Self {
        Self {
            intensity: 100,
            batch_size: None,
            power_limit: None,
            temp_limit: None,
        }
    }
}

/// Command-line arguments for the SHA3x miner
#[derive(Parser, Debug)]
#[command(
    name = "sha3x-miner",
    author = "SHA3x Mining Team",
    version = "1.0.0",
    about = "High-performance SHA3x (Tari) CPU/GPU miner with SV2 testing capabilities",
    long_about = "SHA3x Miner is a high-performance CPU/GPU miner supporting SHA3x for the Tari blockchain.\n\
                  It supports pool mining, standalone benchmarking, SV2 protocol testing, and web dashboard.\n\n\
                  MINING: Requires wallet address, pool connection, and algorithm selection\n\
                  BENCHMARK: Tests hardware performance without pool connection\n\
                  SV2 TEST: Tests Noise protocol connection to Stratum V2 JDS\n\
                  WEB DASHBOARD: Real-time mining statistics and charts at http://localhost:8080\n\
                  GPU MINING: 300+ MH/s with OpenCL GPU acceleration\n\
                  LUCKYPOOL FORMATS: Supports solo:ADDRESS, ADDRESS=DIFF, ADDRESS.WORKER\n\
                  LUCKYPOOL XN: Supports LuckyPool extra nonce (xn) for proper nonce generation\n\n\
                  Examples:\n\
                    CPU Mining: sha3x-miner -u YOUR_TARI_WALLET -o pool.tari.com:4200 --algo sha3x --threads 6\n\
                    GPU Mining: sha3x-miner -u YOUR_TARI_WALLET -o pool.tari.com:4200 --algo sha3x --gpu-intensity 85\n\
                    Hybrid Mining: sha3x-miner -u YOUR_TARI_WALLET -o pool.tari.com:4200 --algo sha3x --threads 12 --gpu-intensity 80\n\
                    Mining with Dashboard: sha3x-miner -u YOUR_TARI_WALLET -o pool.tari.com:4200 --algo sha3x --web\n\
                    LuckyPool Solo: sha3x-miner -u solo:YOUR_WALLET -o ca.luckypool.io:6118 --algo sha3x\n\
                    LuckyPool Worker: sha3x-miner -u YOUR_WALLET.worker-name -o ca.luckypool.io:6118 --algo sha3x\n\
                    LuckyPool Diff: sha3x-miner -u YOUR_WALLET=100G -o ca.luckypool.io:6118 --algo sha3x\n\
                    LuckyPool Diff+Worker: sha3x-miner -u YOUR_WALLET=80G.rig-01 -o ca.luckypool.io:6118 --algo sha3x\n\
                    Benchmark: sha3x-miner --benchmark --algo sha3x --threads 72 --benchmark-duration 60 --benchmark-difficulty 100000\n\
                    SV2 Test: sha3x-miner --test-sv2 --pool 127.0.0.1:34254\n\n\
                  For detailed help, use: sha3x-miner --help"
)]
pub struct Args {
    /// Wallet address for receiving mining rewards (Tari: starts with '12' or '14')
    /// Supports LuckyPool formats: solo:ADDRESS, ADDRESS=DIFF, ADDRESS.WORKER, ADDRESS=DIFF.WORKER
    /// Examples: 
    ///   Standard: 125ohcEDcG8sL4DcdtqZ6YLbSgVYFJWtGeCfHmRocTcyGNYRqMYidnfs1JQPijqQvqV5SLygC5ynxZH3zED5Rr9fPAW
    ///   Solo: solo:125ohcEDcG8sL4DcdtqZ6YLbSgVYFJWtGeCfHmRocTcyGNYRqMYidnfs1JQPijqQvqV5SLygC5ynxZH3zED5Rr9fPAW
    ///   Worker: 125ohcEDcG8sL4DcdtqZ6YLbSgVYFJWtGeCfHmRocTcyGNYRqMYidnfs1JQPijqQvqV5SLygC5ynxZH3zED5Rr9fPAW.rig-01
    ///   Difficulty: 125ohcEDcG8sL4DcdtqZ6YLbSgVYFJWtGeCfHmRocTcyGNYRqMYidnfs1JQPijqQvqV5SLygC5ynxZH3zED5Rr9fPAW=100G
    #[arg(
        short = 'u', 
        long = "wallet",
        value_name = "ADDRESS",
        help = "Wallet address for mining rewards (supports LuckyPool formats)"
    )]
    pub wallet: Option<String>,

    /// Mining pool address in format hostname:port or ip:port
    /// Examples: pool.tari.com:4200, ca.luckypool.io:6118, localhost:34255, 192.168.1.100:4200, 127.0.0.1:34254 (JDS)
    #[arg(
        short = 'o', 
        long = "pool",
        value_name = "HOST:PORT",
        help = "Mining pool address (format: host:port)"
    )]
    pub pool: Option<String>,

    /// Pool password (often worker name or 'x' for no password)
    /// Examples: x, worker1, your-worker-name
    #[arg(
        short = 'p', 
        long = "password",
        value_name = "PASSWORD",
        default_value = "x",
        help = "Pool password (usually 'x' or worker identifier)"
    )]
    pub password: String,

    /// Worker name for pool identification (helps identify different mining rigs)
    /// Use descriptive names like: rig-01, office-pc, server-main, xeon-dual
    /// Avoid spaces and special characters
    #[arg(
        long, 
        default_value = "worker1",
        value_name = "NAME",
        help = "Worker identifier for pool (e.g., rig-01, office-pc)"
    )]
    pub worker: String,

    /// Number of CPU mining threads to use
    /// 0 = auto-detect (recommended), or specify exact count
    /// Typical values: 32 (high-end desktop), 64 (workstation), 72 (dual Xeon)
    #[arg(
        short, 
        long, 
        default_value = "0",
        value_name = "COUNT",
        help = "Number of CPU threads (0 = auto-detect)"
    )]
    pub threads: usize,

    /// Enable GPU mining (placeholder for future implementation)
    /// Currently not implemented - CPU mining only
    #[arg(
        short, 
        long, 
        default_value = "false",
        help = "Enable GPU mining (future feature)"
    )]
    pub gpu: bool,

    /// Enable TUI dashboard (requires 'tui' feature)
    /// Provides real-time visual dashboard with charts and statistics
    #[cfg(feature = "tui")]
    #[arg(
        long, 
        default_value = "false",
        help = "Enable TUI dashboard interface"
    )]
    pub tui: bool,

    /// Enable real-time web dashboard at http://localhost:8080
    /// Provides web-based mining statistics with live charts and graphs
    /// Includes hashrate trends, thread performance, share analytics, and efficiency metrics
    #[arg(
        long, 
        default_value = "false",
        help = "Enable real-time web dashboard with live charts at http://localhost:8080"
    )]
    pub web: bool,

    /// Run in benchmark mode (no pool connection required)
    /// Tests hardware performance and finds optimal settings
    /// Useful for: hardware testing, optimization, comparison
    #[arg(
        long, 
        default_value = "false",
        help = "Run performance benchmark (no pool required)"
    )]
    pub benchmark: bool,

    /// Benchmark duration in seconds
    /// Recommended: 30s (quick test), 60s (standard), 300s (stability test)
    /// Longer tests provide more accurate results
    #[arg(
        long, 
        default_value = "30",
        value_name = "SECONDS",
        help = "Benchmark duration in seconds [30s=quick, 60s=standard, 300s=extended]"
    )]
    pub benchmark_duration: u64,

    /// Benchmark target difficulty (affects share finding frequency)
    /// 1.0 = very easy (many shares), 0.1 = medium, 0.0001 = realistic
    #[arg(
        long, 
        default_value = "1.0",
        value_name = "DIFFICULTY",
        help = "Benchmark difficulty [1.0=easy, 0.1=medium, 0.0001=realistic]"
    )]
    pub benchmark_difficulty: f64,

    /// Mining algorithm to use
    /// Examples: sha3x (Tari)
    #[arg(
        long,
        default_value = "sha3x",
        value_name = "ALGO",
        help = "Mining algorithm (sha3x)"
    )]
    pub algo: String,

    /// Test SV2 Noise protocol connection to JDS
    /// Tests encrypted connection establishment without mining
    /// Useful for: SV2 setup verification, connection troubleshooting
    #[arg(
        long, 
        default_value = "false",
        help = "Test SV2 Noise connection to JDS"
    )]
    pub test_sv2: bool,

    // GPU Control Parameters (available in gpu and hybrid features)
    
    /// GPU mining intensity as percentage (0-100%)
    /// Controls overall GPU workload: 100% = maximum performance, 85% = balanced, 70% = power efficient
    /// Lower values reduce power consumption and heat generation
    #[cfg(any(feature = "gpu", feature = "hybrid"))]
    #[arg(
        long,
        value_name = "PERCENT", 
        default_value = "100",
        help = "GPU mining intensity (0-100%) [100=max, 85=balanced, 70=efficient]"
    )]
    pub gpu_intensity: u8,

    /// Override automatic GPU batch size calculation
    /// Larger batches = higher efficiency but more memory usage
    /// Typical ranges: 10,000 (conservative) to 1,000,000 (aggressive)
    /// RTX 4060 Ti recommended: 50,000-500,000
    #[cfg(any(feature = "gpu", feature = "hybrid"))]
    #[arg(
        long,
        value_name = "COUNT",
        help = "Override GPU batch size (1,000-1,000,000) [auto if not specified]"
    )]
    pub gpu_batch_size: Option<u32>,

    /// GPU power limit as percentage (50-110%)
    /// Reduces power consumption and heat at the cost of some performance
    /// Recommended: 80-90% for 24/7 mining, 70-80% for hot climates
    /// Requires MSI Afterburner or similar tool to be effective
    #[cfg(any(feature = "gpu", feature = "hybrid"))]
    #[arg(
        long,
        value_name = "PERCENT",
        help = "GPU power limit (50-110%) [requires MSI Afterburner/similar]"
    )]
    pub gpu_power_limit: Option<u8>,

    /// GPU temperature throttle limit in Celsius (60-85°C)
    /// Mining will be reduced if GPU temperature exceeds this limit
    /// Recommended: 75°C (balanced), 70°C (conservative), 80°C (aggressive)
    /// Helps protect hardware and maintain stability
    #[cfg(any(feature = "gpu", feature = "hybrid"))]
    #[arg(
        long,
        value_name = "CELSIUS",
        help = "GPU temperature limit (60-85°C) [75=balanced, 70=safe, 80=aggressive]"
    )]
    pub gpu_temp_limit: Option<u8>,
}

/// Raw job data received from the mining pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolJob {
    /// Unique job identifier from the pool
    pub job_id: String,
    
    /// Hex-encoded target difficulty (8 bytes, little-endian)
    pub target: String,
    
    /// Mining algorithm (sha3x for Tari)
    pub algo: String,
    
    /// Current blockchain height
    pub height: u64,
    
    /// Optional difficulty value (some pools send this directly)
    #[serde(default)]
    pub difficulty: Option<u64>,
    
    // SHA3X-specific fields
    /// Hex-encoded block header template (32 bytes when decoded, SHA3X only)
    #[serde(default)]
    pub blob: Option<String>,
    
    /// Optional seed hash (used for SHA3X algorithm verification)
    #[serde(default)]
    pub seed_hash: Option<String>,
    
    /// LuckyPool extra nonce (xn) - 2 bytes hex string (e.g., "ad49")
    /// This is the first 2 bytes of the 8-byte nonce for LuckyPool
    #[serde(default)]
    pub xn: Option<String>,
    
    // Legacy SHA-256-specific fields (kept for compatibility)
    /// Previous block hash (hex, 32 bytes, legacy)
    #[serde(default)]
    pub prev_hash: Option<String>,
    
    /// Merkle root (hex, 32 bytes, legacy)
    #[serde(default)]
    pub merkle_root: Option<String>,
    
    /// Block version (legacy)
    #[serde(default)]
    pub version: Option<u32>,
    
    /// Timestamp (legacy)
    #[serde(default)]
    pub ntime: Option<u32>,
    
    /// Difficulty bits (legacy)
    #[serde(default)]
    pub nbits: Option<u32>,
    
    /// Merkle path hashes (hex, array of 32-byte hashes, legacy)
    #[serde(default)]
    pub merkle_path: Option<Vec<String>>,
}

/// Internal representation of a mining job
#[derive(Debug, Clone)]
pub struct MiningJob {
    /// Job identifier (matches PoolJob.job_id)
    pub job_id: String,
    
    /// Decoded mining hash/header template (32 bytes for SHA3X)
    pub mining_hash: Vec<u8>,
    
    /// Target difficulty as u64 (converted from hex target)
    pub target_difficulty: u64,
    
    /// Blockchain height for this job
    pub height: u64,
    
    /// Mining algorithm
    pub algo: Algorithm,
    
    /// LuckyPool extra nonce (xn) - first 2 bytes of nonce
    /// When present, nonce format should be: [xn][6-bytes-local] = 8 bytes total
    pub extranonce2: Option<String>,
    
    // Legacy SHA-256-specific fields (kept for compatibility)
    /// Previous block hash (32 bytes, legacy)
    pub prev_hash: Option<Vec<u8>>,
    
    /// Merkle root (32 bytes, legacy)
    pub merkle_root: Option<Vec<u8>>,
    
    /// Block version (legacy)
    pub version: Option<u32>,
    
    /// Timestamp (legacy)
    pub ntime: Option<u32>,
    
    /// Difficulty bits (legacy)
    pub nbits: Option<u32>,
    
    /// Merkle path hashes (array of 32-byte hashes, legacy)
    pub merkle_path: Option<Vec<Vec<u8>>>,
    
    /// Target bytes for legacy support (32 bytes, little-endian)
    pub target: Option<[u8; 32]>,
}

/// Represents a found share ready for submission
#[derive(Debug, Clone)]
pub struct Share {
    /// Job ID this share is for
    pub job_id: String,
    
    /// Nonce value that produced valid hash (little-endian)
    pub nonce: u64,
    
    /// The resulting hash (32 bytes)
    pub hash: Vec<u8>,
    
    /// Calculated difficulty of this share
    pub difficulty: u64,
    
    /// Thread ID that found this share
    pub thread_id: usize,
    
    /// Timestamp when share was found
    pub found_at: std::time::Instant,
}

/// Pool response to a submitted share
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareResponse {
    /// Response ID (matches submission ID)
    pub id: u64,
    
    /// Result of share submission
    #[serde(default)]
    pub result: Option<ShareResult>,
    
    /// Error information if share was rejected
    #[serde(default)]
    pub error: Option<ShareError>,
}

/// Possible share submission results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ShareResult {
    /// Simple boolean result (some pools)
    Bool(bool),
    
    /// Status object result (Tari pools)
    Status { status: String },
    
    /// Null result (often means accepted)
    Null,
}

/// Share rejection error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareError {
    /// Error code
    pub code: i32,
    
    /// Error message
    pub message: String,
    
    /// Additional error data
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

/// Represents a mining target/difficulty
#[derive(Debug, Clone, Copy)]
pub struct Target {
    /// Difficulty as u64
    pub difficulty: u64,
    
    /// Raw target bytes (32 bytes for SHA3x)
    pub bits: [u8; 32],
}

/// Benchmark results for performance testing
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Total hashes computed
    pub total_hashes: u64,
    
    /// Duration of benchmark
    pub duration: std::time::Duration,
    
    /// Average hashrate (H/s)
    pub hashrate: f64,
    
    /// Peak hashrate (H/s)
    pub peak_hashrate: f64,
    
    /// Shares found during benchmark
    pub shares_found: u64,
    
    /// Thread count used
    pub thread_count: usize,
    
    /// Memory allocations (if tracked)
    pub allocations: Option<u64>,
}

impl Args {
    /// Get GPU settings from command line arguments
    #[cfg(any(feature = "gpu", feature = "hybrid"))]
    pub fn get_gpu_settings(&self) -> GpuSettings {
        GpuSettings {
            intensity: self.gpu_intensity.min(100),
            batch_size: self.gpu_batch_size.map(|b| b.max(1_000).min(1_000_000)),
            power_limit: self.gpu_power_limit.map(|p| p.max(50).min(110)),
            temp_limit: self.gpu_temp_limit.map(|t| t.max(60).min(85)),
        }
    }

    /// Validate arguments and return helpful errors (supports LuckyPool formats)
    pub fn validate(&self) -> Result<(), String> {
        // Skip validation for SV2 test mode
        if self.test_sv2 {
            return Ok(());
        }

        if !self.benchmark {
            if self.wallet.is_none() {
                return Err("Wallet address is required for mining mode. Use --wallet YOUR_ADDRESS".to_string());
            }
            if self.pool.is_none() {
                return Err("Pool address is required for mining mode. Use --pool HOST:PORT".to_string());
            }
            
            // Validate wallet address format (supports LuckyPool formats)
            if let Some(ref wallet) = self.wallet {
                match self.algo.as_str() {
                    "sha3x" => {
                        // Extract the base address for validation from various LuckyPool formats
                        let base_address = if wallet.starts_with("solo:") {
                            // Solo mining format: solo:ADDRESS
                            wallet.strip_prefix("solo:").unwrap_or(wallet)
                        } else if wallet.contains('=') {
                            // Static difficulty format: ADDRESS=DIFF or ADDRESS=DIFF.WORKER
                            wallet.split('=').next().unwrap_or(wallet)
                        } else if wallet.contains('.') {
                            // Worker name format: ADDRESS.WORKER
                            wallet.split('.').next().unwrap_or(wallet)
                        } else {
                            // Standard format: ADDRESS
                            wallet
                        };

                        // Validate the base Tari address
                        if base_address.len() < 80 {
                            return Err(format!(
                                "Tari wallet address is too short (minimum 80 characters). Found: {} chars in '{}'",
                                base_address.len(), base_address
                            ));
                        }
                        
                        if !base_address.starts_with("12") && !base_address.starts_with("14") {
                            return Err(format!(
                                "Tari wallet address must start with '12' (one-sided) or '14' (interactive). Found: '{}'",
                                base_address
                            ));
                        }
                        
                        if !base_address.chars().all(|c| "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(c)) {
                            return Err(format!(
                                "Tari wallet address contains invalid characters (must be Base58). Found: '{}'",
                                base_address
                            ));
                        }

                        // Validate specific LuckyPool format syntax if present
                        if wallet.starts_with("solo:") {
                            // Solo mining format validation
                            if wallet.len() <= 5 {  // "solo:" is 5 characters
                                return Err("Solo mining format requires address: solo:YOUR_ADDRESS".to_string());
                            }
                        } else if wallet.contains('=') {
                            // Static difficulty format validation: ADDRESS=DIFF or ADDRESS=DIFF.WORKER
                            let parts: Vec<&str> = wallet.split('=').collect();
                            if parts.len() != 2 {
                                return Err("Static difficulty format must be: ADDRESS=DIFF or ADDRESS=DIFF.WORKER".to_string());
                            }
                            
                            let diff_part = parts[1];
                            if diff_part.contains('.') {
                                // FORMAT: ADDRESS=DIFF.WORKER
                                let diff_worker_parts: Vec<&str> = diff_part.split('.').collect();
                                if diff_worker_parts.len() != 2 {
                                    return Err("Static difficulty with worker format must be: ADDRESS=DIFF.WORKER".to_string());
                                }
                                // Validate difficulty is numeric (supports G, M suffixes)
                                let diff_str = diff_worker_parts[0];
                                if !self.is_valid_difficulty_format(diff_str) {
                                    return Err(format!(
                                        "Difficulty must be numeric with optional G/M suffix in ADDRESS=DIFF.WORKER format. Found: '{}'",
                                        diff_str
                                    ));
                                }
                                // Worker name validation
                                let worker_name = diff_worker_parts[1];
                                if worker_name.is_empty() {
                                    return Err("Worker name cannot be empty in ADDRESS=DIFF.WORKER format".to_string());
                                }
                            } else {
                                // FORMAT: ADDRESS=DIFF
                                if !self.is_valid_difficulty_format(diff_part) {
                                    return Err(format!(
                                        "Difficulty must be numeric with optional G/M suffix in ADDRESS=DIFF format. Found: '{}'",
                                        diff_part
                                    ));
                                }
                            }
                        } else if wallet.contains('.') {
                            // Worker name format validation: ADDRESS.WORKER
                            let parts: Vec<&str> = wallet.split('.').collect();
                            if parts.len() != 2 {
                                return Err("Worker name format must be: ADDRESS.WORKER".to_string());
                            }
                            let worker_name = parts[1];
                            if worker_name.is_empty() {
                                return Err("Worker name cannot be empty in ADDRESS.WORKER format".to_string());
                            }
                            // Validate worker name contains only allowed characters
                            if !worker_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                                return Err(format!(
                                    "Worker name can only contain letters, numbers, hyphens, and underscores. Found: '{}'",
                                    worker_name
                                ));
                            }
                        }
                    }
                    _ => return Err("Only 'sha3x' algorithm is supported in this version".to_string()),
                }
            }
            
            // Validate pool address format
            if let Some(ref pool) = self.pool {
                if !pool.contains(':') {
                    return Err("Pool address must be in format HOST:PORT (e.g., pool.tari.com:4200)".to_string());
                }
                let parts: Vec<&str> = pool.split(':').collect();
                if parts.len() != 2 {
                    return Err("Pool address must contain exactly one colon (HOST:PORT)".to_string());
                }
                if parts[1].parse::<u16>().is_err() {
                    return Err("Pool port must be a valid number (1-65535)".to_string());
                }
            }
        }
        
        // Validate algorithm
        match self.algo.as_str() {
            "sha3x" => Ok(()),
            _ => Err("Only 'sha3x' algorithm is supported in this version".to_string()),
        }?;
        
        if self.benchmark_duration == 0 {
            return Err("Benchmark duration must be greater than 0 seconds".to_string());
        }
        
        if self.benchmark_duration > 3600 {
            return Err("Benchmark duration cannot exceed 1 hour (3600 seconds)".to_string());
        }
        
        if self.threads > 1024 {
            return Err("Thread count cannot exceed 1024".to_string());
        }

        // Validate GPU parameters if available
        #[cfg(any(feature = "gpu", feature = "hybrid"))]
        {
            if self.gpu_intensity > 100 {
                return Err("GPU intensity must be between 0-100%".to_string());
            }
            
            if let Some(batch_size) = self.gpu_batch_size {
                if batch_size < 1_000 || batch_size > 1_000_000 {
                    return Err("GPU batch size must be between 1,000 and 1,000,000".to_string());
                }
            }
            
            if let Some(power_limit) = self.gpu_power_limit {
                if power_limit < 50 || power_limit > 110 {
                    return Err("GPU power limit must be between 50-110%".to_string());
                }
            }
            
            if let Some(temp_limit) = self.gpu_temp_limit {
                if temp_limit < 60 || temp_limit > 85 {
                    return Err("GPU temperature limit must be between 60-85°C".to_string());
                }
            }
        }
        
        Ok(())
    }

    /// Helper function to validate difficulty format (supports numeric values with G, M suffixes)
    fn is_valid_difficulty_format(&self, diff_str: &str) -> bool {
        if diff_str.is_empty() {
            return false;
        }
        
        // Check if it ends with G or M (case insensitive)
        let (number_part, _suffix) = if diff_str.to_lowercase().ends_with('g') {
            (diff_str[..diff_str.len()-1].to_string(), "G")
        } else if diff_str.to_lowercase().ends_with('m') {
            (diff_str[..diff_str.len()-1].to_string(), "M")
        } else {
            (diff_str.to_string(), "")
        };
        
        // Validate the numeric part
        number_part.parse::<f64>().is_ok()
    }
}

impl Share {
    /// Create a new share
    pub fn new(
        job_id: String,
        nonce: u64,
        hash: Vec<u8>,
        difficulty: u64,
        thread_id: usize,
    ) -> Self {
        Self {
            job_id,
            nonce,
            hash,
            difficulty,
            thread_id,
            found_at: std::time::Instant::now(),
        }
    }
    
    /// Get the age of this share
    pub fn age(&self) -> std::time::Duration {
        self.found_at.elapsed()
    }
}

impl BenchmarkResult {
    /// Calculate hashrate from totals
    pub fn calculate_hashrate(total_hashes: u64, duration: std::time::Duration) -> f64 {
        total_hashes as f64 / duration.as_secs_f64()
    }
    
    /// Format hashrate for display
    pub fn format_hashrate(&self) -> String {
        if self.hashrate >= 1_000_000.0 {
            format!("{:.2} MH/s", self.hashrate / 1_000_000.0)
        } else if self.hashrate >= 1_000.0 {
            format!("{:.2} KH/s", self.hashrate / 1_000.0)
        } else {
            format!("{:.2} H/s", self.hashrate)
        }
    }
}

// Changelog:
// - v1.1.4-luckypool-xn-support (2025-06-26): Added LuckyPool XN (extra nonce) field support.
//   *** LUCKYPOOL XN SUPPORT ***:
//   - Added xn: Option<String> field to PoolJob struct for LuckyPool extra nonce
//   - Added extranonce2: Option<String> field to MiningJob struct to store parsed xn
//   - XN represents the first 2 bytes of the 8-byte nonce for LuckyPool
//   - When xn is present, nonce format: [2-byte-xn][6-byte-local] = 8 bytes total
//   *** NONCE FORMAT SPECIFICATION ***:
//   - LuckyPool sends xn field in job (e.g., "ad49" = 2 bytes)
//   - Mining threads must use xn as first 2 bytes of nonce
//   - Remaining 6 bytes generated locally by mining threads
//   - Total nonce length: exactly 8 bytes for LuckyPool compatibility
//   *** TECHNICAL IMPLEMENTATION ***:
//   - PoolJob.xn: Raw hex string from pool (e.g., "ad49")
//   - MiningJob.extranonce2: Processed xn for mining threads
//   - Maintains backward compatibility with pools that don't use xn
//   - Updated help text to mention LuckyPool XN support
//   *** INTEGRATION NOTES ***:
//   - Job parsing code must copy xn from PoolJob to MiningJob.extranonce2
//   - Mining loops must check for extranonce2 and use it in nonce generation
//   - Share submission should use combined [xn][local] nonce format
//   - This fixes "Invalid nonce" errors from LuckyPool
// - v1.1.3-luckypool-formats (2025-06-26): Added comprehensive LuckyPool address format support.
//   *** LUCKYPOOL FORMAT SUPPORT ***:
//   - Added support for solo mining format: solo:ADDRESS
//   - Added support for static difficulty format: ADDRESS=DIFF or ADDRESS=DIFF.WORKER  
//   - Added support for worker name format: ADDRESS.WORKER
//   - Combined format support: ADDRESS=DIFF.WORKER (difficulty + worker name)
//   *** ENHANCED VALIDATION ***:
//   - Smart address extraction: Validates only the base Tari address part
//   - Difficulty validation: Supports numeric values with G/M suffixes (e.g., 100G, 80M)
//   - Worker name validation: Alphanumeric, hyphens, underscores only
//   - Format-specific error messages for better user guidance
//   *** TECHNICAL IMPLEMENTATION ***:
//   - Added is_valid_difficulty_format() helper for difficulty parsing
//   - Enhanced validate() method with comprehensive format checking
//   - Updated help text and examples to include all LuckyPool formats
//   - Maintains backward compatibility with standard address format
//   *** SUPPORTED LUCKYPOOL FORMATS ***:
//   - solo:125ohc...PAW (solo mining)
//   - 125ohc...PAW.worker-name (worker identification)
//   - 125ohc...PAW=100G (static 100G difficulty)
//   - 125ohc...PAW=80G.rig-01 (80G difficulty + worker name)
//   *** VALIDATION RULES ***:
//   - Base address must be valid Tari address (80+ chars, starts with 12/14, Base58)
//   - Difficulty supports numbers with optional G/M suffix
//   - Worker names: alphanumeric + hyphens + underscores only
//   - Comprehensive error messages guide users to correct format
// - v1.1.2-gpu-controls (2025-06-25): Added GPU control parameters for gpu and hybrid features.
//   - Added GpuSettings struct for managing GPU parameters.
//   - Added gpu_intensity, gpu_batch_size, gpu_power_limit, gpu_temp_limit fields to Args.
//   - Added get_gpu_settings() method to extract GPU settings from CLI args.
//   - Enhanced validation to check GPU parameter ranges.
//   - Updated help text and examples to include GPU mining modes.
// - v1.1.1-web (2025-06-22): Added web dashboard support.
//   - Added web: bool field to Args struct for enabling web dashboard.
//   - Updated help text and examples to include web dashboard usage.
//   - Enhanced long_about to describe web dashboard functionality.
//   - Maintained all existing SV2 testing and benchmark functionality.
// - v1.1.0-sv2 (2025-06-20): Added SV2 testing support.
//   - Added test_sv2: bool field to Args struct for SV2 connection testing.
//   - Updated validation to skip checks for SV2 test mode.
//   - Restricted algorithm support to SHA3x only (removed SHA-256).
//   - Updated help text and examples to include SV2 testing.
//   - Kept legacy SHA-256 fields for backward compatibility.
// - v1.0.5 (2025-06-17): Fixed type mismatch for benchmark difficulty.
//   - Changed Args::benchmark_difficulty from u64 to f64 to support decimals.
//   - Updated default value to 1.0 and help text for decimal inputs.
//   - Compatible with main.rs v1.0.7, runner.rs v1.0.24, jobs.rs v1.0.12.