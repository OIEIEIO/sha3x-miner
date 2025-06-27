// SHA3x Miner - Free and Open Source Software Statement
//
// This project, sha3x-miner, is Free and Open Source Software (FOSS) licensed
// under the MIT License. You are free to use, modify, and distribute this
// software in accordance with the license terms. Contributions are welcome
// via pull requests to the project repository.
//
// File: src/miner/gpu/mod.rs
// Version: 1.2.0-hybrid
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// This file is the module entry point for GPU mining functionality in the SHA3x miner.
// It provides OpenCL-based GPU mining capabilities that work alongside existing CPU mining.
//
// Features:
// - OpenCL GPU mining for NVIDIA, AMD, and Intel GPUs
// - Parallel GPU and CPU mining coordination
// - Integrated stats and monitoring
// - Optional compilation via the "gpu" or "hybrid" feature flags

// GPU mining is available when either "gpu" or "hybrid" feature is enabled
#[cfg(any(feature = "gpu", feature = "hybrid"))]
pub mod opencl;

#[cfg(any(feature = "gpu", feature = "hybrid"))]
pub mod manager;

#[cfg(any(feature = "gpu", feature = "hybrid"))]
pub mod gpu_miner;

// Re-export key types when GPU features are enabled
#[cfg(any(feature = "gpu", feature = "hybrid"))]
pub use manager::GpuManager;

#[cfg(any(feature = "gpu", feature = "hybrid"))]
pub use gpu_miner::GpuMiner;

// Placeholder for when GPU features are disabled
#[cfg(not(any(feature = "gpu", feature = "hybrid")))]
pub struct GpuManager;

#[cfg(not(any(feature = "gpu", feature = "hybrid")))]
impl GpuManager {
    pub fn new() -> Self {
        Self
    }
    
    pub fn is_available() -> bool {
        false
    }
}

// Changelog:
// - v1.2.0-hybrid (2025-06-25): Added hybrid feature support
//   - Changed feature gates from feature = "gpu" to any(feature = "gpu", feature = "hybrid")
//   - Enables GPU modules when either gpu or hybrid features are active
//   - Supports both standalone GPU mode and hybrid CPU+GPU mode
// - v1.1.0 (2025-06-24): Added GpuMiner support
//   - Added gpu_miner module declaration for GPU-only mining
//   - Added GpuMiner re-export for direct GPU mining capabilities  
//   - Enables 363+ MH/s GPU-only mining mode
// - v1.0.0 (2025-06-24): Initial GPU module structure
//   - Added conditional compilation for GPU features
//   - Created module structure for OpenCL GPU mining
//   - Maintains compatibility when GPU feature is disabled
//   - Provides foundation for GPU mining integration