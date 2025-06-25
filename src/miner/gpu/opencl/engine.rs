// SHA3x Miner - Free and Open Source Software Statement
//
// File: src/miner/gpu/opencl/engine.rs  
// Version: 2.0.1
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// OpenCL mining engine for SHA3x GPU mining - fixed for correct header size

use anyhow::{Error, Result};
use opencl3::{
    command_queue::CommandQueue,
    context::Context,
    kernel::{ExecuteKernel, Kernel},
    memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_WRITE_ONLY, CL_MEM_COPY_HOST_PTR},
    program::Program,
    types::{cl_ulong, CL_FALSE, CL_TRUE},
};
use std::{ptr, time::Instant};
use tracing::{debug, error, info};
use crate::core::types::MiningJob;
use super::device::OpenClDevice;

/// OpenCL mining engine for GPU SHA3x mining
pub struct OpenClEngine {
    device: OpenClDevice,
    context: Context,
    program: Option<Program>,
    kernel: Option<Kernel>,
    queue: Option<CommandQueue>,
    initialized: bool,
}

impl OpenClEngine {
    /// Create a new OpenCL engine for the specified device
    pub fn new(device: OpenClDevice) -> Self {
        debug!("Creating OpenCL engine for device: {}", device.name());
        let context = Context::from_device(&device.device()).unwrap();
        Self {
            device,
            context,
            program: None,
            kernel: None,
            queue: None,
            initialized: false,
        }
    }
    
    /// Initialize the OpenCL engine with SHA3x kernel
    pub fn initialize(&mut self) -> Result<()> {
        info!("Initializing OpenCL engine for {}", self.device.name());
        
        // Load and compile the SHA3x kernel
        let kernel_source = include_str!("../../../../kernels/opencl/sha3x.cl");
        
        let mut program = Program::create_from_source(&self.context, kernel_source)
            .map_err(|e| Error::msg(format!("Failed to create program: {}", e)))?;
        
        // Build the program
        match program.build(self.context.devices(), "") {
            Ok(_) => {
                info!("OpenCL program built successfully for {}", self.device.name());
            }
            Err(e) => {
                error!("Failed to build OpenCL program: {}", e);
                // Get build log for debugging
                for device_id in self.context.devices() {
                    if let Ok(log) = program.get_build_log(*device_id) {
                        error!("Build log for device {:?}: {}", device_id, log);
                    }
                }
                return Err(Error::msg(format!("Program build failed: {}", e)));
            }
        }
        
        // Create kernel
        let kernel = Kernel::create(&program, "sha3")
            .map_err(|e| Error::msg(format!("Failed to create kernel: {}", e)))?;
        
        // Create command queue
        let queue = CommandQueue::create_default(&self.context, 0)
            .map_err(|e| Error::msg(format!("Failed to create command queue: {}", e)))?;
        
        self.program = Some(program);
        self.kernel = Some(kernel);
        self.queue = Some(queue);
        self.initialized = true;
        
        info!("✅ OpenCL engine initialized for {} (CU: {}, WG: {})", 
              self.device.name(), 
              self.device.max_compute_units(), 
              self.device.max_work_group_size());
        
        Ok(())
    }
    
    /// Check if engine is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Get device info
    pub fn device(&self) -> &OpenClDevice {
        &self.device
    }
    
    /// Calculate optimal work sizes for mining
    fn calculate_work_sizes(&self) -> (usize, usize) {
        let compute_units = self.device.max_compute_units() as usize;
        let max_work_group_size = self.device.max_work_group_size();
        
        // Calculate optimal local work size (threads per work group)
        let local_size = (max_work_group_size / 4).max(64).min(256);
        
        // Calculate global work size (total threads)
        // Use multiple work groups per compute unit for better occupancy
        let work_groups_per_cu = 8; // Tunable parameter
        let total_work_groups = compute_units * work_groups_per_cu;
        let global_size = total_work_groups * local_size;
        
        debug!("Calculated work sizes for {}: global={}, local={} (WG: {}/CU)", 
               self.device.name(), global_size, local_size, work_groups_per_cu);
        
        (global_size, local_size)
    }
    
    /// Mine using GPU - returns (found_nonce, hashes_processed, best_difficulty)
    pub fn mine(
        &self,
        job: &MiningJob,
        nonce_start: u64,
        batch_size: u32,
    ) -> Result<(Option<u64>, u32, u64)> {
        if !self.initialized {
            return Err(Error::msg("Engine not initialized"));
        }
        
        let kernel = self.kernel.as_ref().unwrap();
        let queue = self.queue.as_ref().unwrap();
        
        // Prepare mining data - SHA3x jobs use 32-byte headers
        if job.mining_hash.len() < 32 {
            return Err(Error::msg(format!("Invalid mining hash length: expected >= 32 bytes, got {}", job.mining_hash.len())));
        }
        
        // Convert header to u64 array for OpenCL kernel (32 bytes = 4 u64s)
        let mut buffer_data = vec![0u64; 4];
        for i in 0..4 {
            let start_idx = i * 8;
            if start_idx + 8 <= job.mining_hash.len() {
                buffer_data[i] = u64::from_le_bytes([
                    job.mining_hash[start_idx],
                    job.mining_hash[start_idx + 1],
                    job.mining_hash[start_idx + 2],
                    job.mining_hash[start_idx + 3],
                    job.mining_hash[start_idx + 4],
                    job.mining_hash[start_idx + 5],
                    job.mining_hash[start_idx + 6],
                    job.mining_hash[start_idx + 7],
                ]);
            }
        }
        
        let start_time = Instant::now();
        
        // Create input buffer
        let mut input_buffer = unsafe {
            Buffer::<cl_ulong>::create(
                &self.context,
                CL_MEM_READ_ONLY,
                buffer_data.len(),
                ptr::null_mut(),
            ).map_err(|e| Error::msg(format!("Failed to create input buffer: {}", e)))?
        };
        
        // Write input data
        unsafe {
            queue.enqueue_write_buffer(&mut input_buffer, CL_FALSE, 0, &buffer_data, &[])
                .map_err(|e| Error::msg(format!("Failed to write input buffer: {}", e)))?;
        }
        
        // Create output buffer [nonce, best_hash]
        let initial_output = vec![0u64, 0u64];
        let output_buffer = unsafe {
            Buffer::<cl_ulong>::create(
                &self.context,
                CL_MEM_WRITE_ONLY | CL_MEM_COPY_HOST_PTR,
                2,
                initial_output.as_ptr() as *mut std::ffi::c_void,
            ).map_err(|e| Error::msg(format!("Failed to create output buffer: {}", e)))?
        };
        
        // Calculate work sizes
        let (global_size, _local_size) = self.calculate_work_sizes();
        
        // Calculate the target value for the kernel (not difficulty)
        let target_value = if job.target_difficulty > 0 {
            // Convert difficulty to target hash value
            // Higher difficulty = lower target value
            u64::MAX / job.target_difficulty
        } else {
            u64::MAX
        };
        
        // Execute kernel
        unsafe {
            ExecuteKernel::new(kernel)
                .set_arg(&input_buffer)
                .set_arg(&nonce_start)
                .set_arg(&target_value)  // Pass target value, not difficulty
                .set_arg(&batch_size)
                .set_arg(&output_buffer)
                .set_global_work_size(global_size)
                .enqueue_nd_range(queue)
                .map_err(|e| Error::msg(format!("Failed to execute kernel: {}", e)))?;
        }
        
        // Wait for completion
        queue.finish().map_err(|e| Error::msg(format!("Failed to finish queue: {}", e)))?;
        
        // Read results
        let mut output = vec![0u64, 0u64];
        unsafe {
            queue.enqueue_read_buffer(&output_buffer, CL_TRUE, 0, &mut output, &[])
                .map_err(|e| Error::msg(format!("Failed to read output buffer: {}", e)))?;
        }
        
        let mining_time = start_time.elapsed();
        let hashes_processed = global_size as u32 * batch_size;
        
        debug!("GPU mining completed in {:.2}ms: {} hashes, {:.2} MH/s", 
               mining_time.as_millis(),
               hashes_processed,
               hashes_processed as f64 / mining_time.as_millis() as f64 / 1000.0);
        
        // Check results
        let found_nonce = if output[0] > 0 { Some(output[0]) } else { None };
        // Calculate actual difficulty from the hash value returned by kernel
        let best_difficulty = if output[1] > 0 && output[1] < u64::MAX { 
            // Convert hash value back to difficulty: difficulty = max_value / hash_value
            u64::MAX / output[1].max(1) // Avoid division by zero
        } else { 
            0 
        };
        
        if let Some(nonce) = found_nonce {
            info!("🎉 GPU found share! Nonce: {}, Difficulty: {}", 
                  nonce, crate::miner::stats::MinerStats::format_number(best_difficulty));
        }
        
        Ok((found_nonce, hashes_processed, best_difficulty))
    }
    
    /// Get suggested batch size based on device capabilities
    pub fn get_suggested_batch_size(&self) -> u32 {
        // Base batch size on compute units and memory
        let compute_units = self.device.max_compute_units();
        let memory_gb = self.device.global_mem_size() as f64 / (1024.0 * 1024.0 * 1024.0);
        
        // Conservative batch size calculation
        let base_batch = compute_units * 1000; // 1000 iterations per compute unit
        let memory_limited = (memory_gb * 1000.0) as u32; // Scale by available memory
        
        base_batch.min(memory_limited).max(1000).min(10000) // Clamp between 1K and 10K
    }
    
    /// Get expected hashrate estimate for this device
    pub fn estimate_hashrate(&self) -> f64 {
        // Rough estimate based on compute units
        // RTX 4060 Ti has ~34 compute units, should achieve 200-400 MH/s
        let compute_units = self.device.max_compute_units() as f64;
        let base_rate_per_cu = 8.0; // MH/s per compute unit (conservative estimate)
        compute_units * base_rate_per_cu
    }
    
    /// Calculate the hash result for a given nonce (for share submission)
    pub fn calculate_share_result(&self, job: &MiningJob, nonce: u64) -> Result<String> {
        // Use the exact same SHA3x algorithm as CPU
        use crate::core::sha3x::sha3x_hash_with_nonce;
        
        // Call the same function that CPU uses
        let hash = sha3x_hash_with_nonce(&job.mining_hash, nonce);
        
        // Return hex-encoded hash (same as CPU)
        Ok(hex::encode(&hash))
    }
}

impl Default for OpenClEngine {
    fn default() -> Self {
        // This should not be used - always create with a specific device
        panic!("OpenClEngine must be created with a specific device")
    }
}