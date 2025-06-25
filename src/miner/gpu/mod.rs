// SHA3x Miner - Free and Open Source Software Statement
//
// This project, sha3x-miner, is Free and Open Source Software (FOSS) licensed
// under the MIT License. You are free to use, modify, and distribute this
// software in accordance with the license terms. Contributions are welcome
// via pull requests to the project repository.
//
// File: src/miner/gpu/mod.rs
// Version: 1.1.0
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// This file is the module entry point for GPU mining functionality in the SHA3x miner.
// It provides OpenCL-based GPU mining capabilities that work alongside existing CPU mining.
//
// Features:
// - OpenCL GPU mining for NVIDIA, AMD, and Intel GPUs
// - Parallel GPU and CPU mining coordination
// - Integrated stats and monitoring
// - Optional compilation via the "gpu" feature flag

// GPU mining is only available when the "gpu" feature is enabled
#[cfg(feature = "gpu")]
pub mod opencl;

#[cfg(feature = "gpu")]
pub mod manager;

#[cfg(feature = "gpu")]
pub mod gpu_miner;  // Add this line

// Re-export key types when GPU feature is enabled
#[cfg(feature = "gpu")]
pub use manager::GpuManager;

#[cfg(feature = "gpu")]
pub use gpu_miner::GpuMiner;  // Add this line

// Placeholder for when GPU feature is disabled
#[cfg(not(feature = "gpu"))]
pub struct GpuManager;

#[cfg(not(feature = "gpu"))]
impl GpuManager {
    pub fn new() -> Self {
        Self
    }
    
    pub fn is_available() -> bool {
        false
    }
}

// Changelog:
// - v1.1.0 (2025-06-24): Added GpuMiner support
//   - Added gpu_miner module declaration for GPU-only mining
//   - Added GpuMiner re-export for direct GPU mining capabilities  
//   - Enables 363+ MH/s GPU-only mining mode
// - v1.0.0 (2025-06-24): Initial GPU module structure
//   - Added conditional compilation for GPU features
//   - Created module structure for OpenCL GPU mining
//   - Maintains compatibility when GPU feature is disabled
//   - Provides foundation for GPU mining integration