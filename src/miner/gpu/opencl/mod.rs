// SHA3x Miner - Free and Open Source Software Statement
//
// File: src/miner/gpu/opencl/mod.rs
// Version: 1.0.0
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// OpenCL module for GPU mining - provides OpenCL-based SHA3x mining

pub mod device;
pub mod engine;

// Re-export key types
pub use device::OpenClDevice;
pub use engine::OpenClEngine;