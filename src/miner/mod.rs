// SHA3x Miner - Free and Open Source Software Statement
//
// This project, sha3x-miner, is Free and Open Source Software (FOSS) licensed
// under the MIT License. You are free to use, modify, and distribute this
// software in accordance with the license terms. Contributions are welcome
// via pull requests to the project repository.
//
// File: src/miner/mod.rs
// Version: 1.1.0
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// This file is the module declaration for the miner functionality of the SHA3x
// miner, located in the miner subdirectory. It declares submodules and re-exports
// key types for use throughout the project.
//
// Tree Location:
// - src/miner/mod.rs (miner module entry point)
// - Submodules: cpu, gpu, stats

pub mod cpu;
pub mod gpu;  // Added GPU mining module
pub mod stats;

// Re-export key types for convenience
pub use cpu::CpuMiner;
pub use gpu::GpuManager;  // Added GPU manager export
pub use stats::{MinerStats, ThreadStats};

// Changelog:
// - v1.1.0 (2025-06-24): Added GPU mining module.
//   - Added gpu submodule declaration for GPU mining functionality.
//   - Added GpuManager re-export for easy access to GPU mining capabilities.
//   - GPU module is conditionally compiled based on "gpu" feature flag.
// - v1.0.0 (2025-06-14): Extracted from monolithic main.rs.
//   - Purpose: Defines the miner module, organizing CPU mining and statistics
//     tracking into cpu and stats submodules.
//   - Features: Declares cpu and stats submodules, with re-exports of key types
//     (e.g., CpuMiner, MinerStats, ThreadStats) for easy access by main.rs and
//     other modules.
//   - Note: This file provides a clean structure for mining-related logic,
//     centralizing the core components of the miner's operation.