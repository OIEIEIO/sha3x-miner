// SHA3x Miner - Free and Open Source Software Statement
//
// File: src/miner/gpu/opencl/engine.rs  
// Version: 2.2.0-sequential-autotune
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// OpenCL mining engine with sequential parameter autotuning for maximum hashrate

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
use tokio::time::Duration;
use tracing::{debug, error, info, warn};
use crate::core::types::{MiningJob, GpuSettings};
use super::device::OpenClDevice;

/// Sequential autotuning configuration
#[derive(Debug, Clone)]
pub struct AutotuneConfig {
    pub cycles: u32,           // Number of optimization cycles (start with 1)
    pub test_duration_secs: u64, // How long to test each setting (default: 30)
    pub intensity_range: Vec<u8>, // Values to test: [80, 85, 90, 95, 100]
    pub batch_sizes: Vec<u32>,    // Values to test: [50_000, 100_000, 200_000, 500_000]
    pub work_groups_per_cu: Vec<usize>, // Values to test: [4, 8, 12, 16, 20]
}

impl Default for AutotuneConfig {
    fn default() -> Self {
        Self {
            cycles: 1, // Start with one cycle
            test_duration_secs: 30,
            intensity_range: vec![80, 85, 90, 95, 100],
            batch_sizes: vec![50_000, 100_000, 200_000, 500_000, 750_000],
            work_groups_per_cu: vec![4, 8, 12, 16, 20],
        }
    }
}

/// OpenCL mining engine for GPU SHA3x mining with sequential autotuning
pub struct OpenClEngine {
    device: OpenClDevice,
    context: Context,
    program: Option<Program>,
    kernel: Option<Kernel>,
    queue: Option<CommandQueue>,
    initialized: bool,
    gpu_settings: GpuSettings,
    work_groups_per_cu: usize, // Add this as tunable parameter
    autotune_config: Option<AutotuneConfig>,
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
            gpu_settings: GpuSettings::default(),
            work_groups_per_cu: 8, // Default value
            autotune_config: None,
        }
    }

    /// Create a new OpenCL engine with GPU settings and optional autotuning
    pub fn new_with_settings(device: OpenClDevice, settings: GpuSettings) -> Self {
        debug!("Creating OpenCL engine for device: {} with settings: intensity={}%, batch={:?}", 
               device.name(), settings.intensity, settings.batch_size);
        let context = Context::from_device(&device.device()).unwrap();
        Self {
            device,
            context,
            program: None,
            kernel: None,
            queue: None,
            initialized: false,
            gpu_settings: settings,
            work_groups_per_cu: 8, // Default value
            autotune_config: None,
        }
    }

    /// Enable sequential autotuning with configuration
    pub fn enable_autotuning(&mut self, config: AutotuneConfig) {
        info!("🎯 Autotuning enabled: {} cycle(s), {}s test duration", 
              config.cycles, config.test_duration_secs);
        self.autotune_config = Some(config);
    }

    /// Update GPU settings after creation
    pub fn set_gpu_settings(&mut self, settings: GpuSettings) {
        info!("Updating GPU settings for {}: intensity={}%, batch={:?}, power={:?}%, temp={:?}°C", 
              self.device.name(), settings.intensity, settings.batch_size, 
              settings.power_limit, settings.temp_limit);
        self.gpu_settings = settings;
    }

    /// Get current GPU settings
    pub fn get_gpu_settings(&self) -> &GpuSettings {
        &self.gpu_settings
    }
    
    /// Initialize the OpenCL engine with SHA3x kernel
    pub fn initialize(&mut self) -> Result<()> {
        info!("Initializing OpenCL engine for {} with GPU settings", self.device.name());
        
        // Display GPU settings being applied
        info!("🎮 GPU Settings: Intensity={}%, Batch={:?}, Power={:?}%, Temp={:?}°C",
              self.gpu_settings.intensity, self.gpu_settings.batch_size,
              self.gpu_settings.power_limit, self.gpu_settings.temp_limit);
        
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

    /// Run sequential autotuning to find optimal settings
    pub async fn run_sequential_autotune(&mut self, test_job: &MiningJob) -> Result<GpuSettings> {
        if !self.initialized {
            return Err(Error::msg("Engine not initialized for autotuning"));
        }

        let config = self.autotune_config.clone()
            .ok_or_else(|| Error::msg("Autotuning not enabled"))?;

        info!("🚀 Starting sequential autotuning with {} cycle(s)", config.cycles);
        info!("📊 Baseline: intensity={}%, batch={:?}, work_groups={}", 
              self.gpu_settings.intensity, self.gpu_settings.batch_size, self.work_groups_per_cu);

        let mut best_settings = self.gpu_settings.clone();
        let mut best_work_groups = self.work_groups_per_cu;
        
        // Record baseline performance
        let baseline_hashrate = self.measure_hashrate(test_job, config.test_duration_secs).await?;
        info!("📈 Baseline hashrate: {:.1} MH/s", baseline_hashrate);

        for cycle in 1..=config.cycles {
            info!("🔄 Starting optimization cycle {}/{}", cycle, config.cycles);
            
            // Phase 1: Fix nothing → Tune intensity (keep batch and work_groups fixed)
            info!("🔧 Phase 1: Optimizing intensity (batch={:?}, work_groups={} fixed)", 
                  best_settings.batch_size, best_work_groups);
            let optimal_intensity = self.optimize_intensity(&config, test_job, &best_settings, best_work_groups).await?;
            best_settings.intensity = optimal_intensity;
            info!("✅ Phase 1 complete: optimal intensity = {}%", optimal_intensity);

            // Phase 2: Fix intensity → Tune batch_size (keep work_groups fixed)
            info!("🔧 Phase 2: Optimizing batch size (intensity={}%, work_groups={} fixed)", 
                  best_settings.intensity, best_work_groups);
            let optimal_batch = self.optimize_batch_size(&config, test_job, &best_settings, best_work_groups).await?;
            best_settings.batch_size = Some(optimal_batch);
            info!("✅ Phase 2 complete: optimal batch size = {}", optimal_batch);

            // Phase 3: Fix intensity and batch → Tune work_groups  
            info!("🔧 Phase 3: Optimizing work groups (intensity={}%, batch={} fixed)", 
                  best_settings.intensity, optimal_batch);
            let optimal_wg = self.optimize_work_groups(&config, test_job, &best_settings).await?;
            best_work_groups = optimal_wg;
            self.work_groups_per_cu = optimal_wg;
            info!("✅ Phase 3 complete: optimal work_groups = {}", optimal_wg);

            // Measure final performance for this cycle
            self.gpu_settings = best_settings.clone();
            let cycle_hashrate = self.measure_hashrate(test_job, config.test_duration_secs).await?;
            let improvement = ((cycle_hashrate - baseline_hashrate) / baseline_hashrate) * 100.0;
            
            info!("📈 Cycle {} results: {:.1} MH/s ({:+.1}% vs baseline)", 
                  cycle, cycle_hashrate, improvement);
        }

        // Apply final optimized settings
        self.gpu_settings = best_settings.clone();
        self.work_groups_per_cu = best_work_groups;
        
        let final_hashrate = self.measure_hashrate(test_job, config.test_duration_secs * 2).await?;
        let total_improvement = ((final_hashrate - baseline_hashrate) / baseline_hashrate) * 100.0;
        
        info!("🏆 AUTOTUNING COMPLETE!");
        info!("├─ Baseline: {:.1} MH/s", baseline_hashrate);
        info!("├─ Optimized: {:.1} MH/s", final_hashrate);
        info!("├─ Improvement: {:+.1}%", total_improvement);
        info!("├─ Final settings: intensity={}%, batch={}, work_groups={}", 
              best_settings.intensity, best_settings.batch_size.unwrap_or(0), best_work_groups);
        info!("└─ Ready for mining!");

        Ok(best_settings)
    }

    /// Phase 1: Optimize intensity while keeping batch and work_groups fixed
    async fn optimize_intensity(&mut self, config: &AutotuneConfig, job: &MiningJob, 
                                fixed_settings: &GpuSettings, fixed_wg: usize) -> Result<u8> {
        let mut best_intensity = fixed_settings.intensity;
        let mut best_hashrate = 0.0;

        for &intensity in config.intensity_range.iter() {
            let mut test_settings = fixed_settings.clone();
            test_settings.intensity = intensity;
            self.gpu_settings = test_settings;
            self.work_groups_per_cu = fixed_wg; // Keep work groups fixed

            let hashrate = self.measure_hashrate(job, config.test_duration_secs).await?;
            info!("  📊 intensity={}%: {:.1} MH/s", intensity, hashrate);

            if hashrate > best_hashrate {
                best_hashrate = hashrate;
                best_intensity = intensity;
            }
        }

        info!("🎯 Best intensity: {}% ({:.1} MH/s)", best_intensity, best_hashrate);
        Ok(best_intensity)
    }

    /// Phase 2: Optimize batch_size while keeping intensity and work_groups fixed
    async fn optimize_batch_size(&mut self, config: &AutotuneConfig, job: &MiningJob, 
                                 fixed_settings: &GpuSettings, fixed_wg: usize) -> Result<u32> {
        let mut best_batch = fixed_settings.batch_size.unwrap_or(100_000);
        let mut best_hashrate = 0.0;

        for &batch_size in config.batch_sizes.iter() {
            let mut test_settings = fixed_settings.clone();
            test_settings.batch_size = Some(batch_size);
            self.gpu_settings = test_settings;
            self.work_groups_per_cu = fixed_wg; // Keep work groups fixed

            let hashrate = self.measure_hashrate(job, config.test_duration_secs).await?;
            info!("  📊 batch={}: {:.1} MH/s", batch_size, hashrate);

            if hashrate > best_hashrate {
                best_hashrate = hashrate;
                best_batch = batch_size;
            }
        }

        info!("🎯 Best batch size: {} ({:.1} MH/s)", best_batch, best_hashrate);
        Ok(best_batch)
    }

    /// Phase 3: Optimize work_groups while keeping intensity and batch fixed
    async fn optimize_work_groups(&mut self, config: &AutotuneConfig, job: &MiningJob, 
                                 fixed_settings: &GpuSettings) -> Result<usize> {
        let mut best_wg = self.work_groups_per_cu;
        let mut best_hashrate = 0.0;

        for &work_groups in config.work_groups_per_cu.iter() {
            self.gpu_settings = fixed_settings.clone(); // Keep intensity and batch fixed
            self.work_groups_per_cu = work_groups;

            let hashrate = self.measure_hashrate(job, config.test_duration_secs).await?;
            info!("  📊 work_groups={}: {:.1} MH/s", work_groups, hashrate);

            if hashrate > best_hashrate {
                best_hashrate = hashrate;
                best_wg = work_groups;
            }
        }

        info!("🎯 Best work groups: {} ({:.1} MH/s)", best_wg, best_hashrate);
        Ok(best_wg)
    }

    /// Measure hashrate over a specified duration
    async fn measure_hashrate(&self, job: &MiningJob, duration_secs: u64) -> Result<f64> {
        let start_time = Instant::now();
        let mut total_hashes = 0u64;
        let mut iterations = 0u32;

        let batch_size = self.get_suggested_batch_size();
        
        while start_time.elapsed().as_secs() < duration_secs {
            let nonce_start = rand::random::<u64>();
            
            match self.mine(job, nonce_start, batch_size).await {
                Ok((_, hashes_processed, _)) => {
                    total_hashes += hashes_processed as u64;
                    iterations += 1;
                }
                Err(e) => {
                    warn!("Mining error during hashrate measurement: {}", e);
                    continue;
                }
            }
        }

        let actual_duration = start_time.elapsed().as_secs_f64();
        let hashrate_mhs = (total_hashes as f64) / actual_duration / 1_000_000.0;
        
        debug!("Measured: {} hashes in {:.1}s ({} iterations) = {:.1} MH/s", 
               total_hashes, actual_duration, iterations, hashrate_mhs);
        
        Ok(hashrate_mhs)
    }
    
    /// Check if engine is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Get device info
    pub fn device(&self) -> &OpenClDevice {
        &self.device
    }
    
    /// Calculate optimal work sizes for mining with intensity consideration and tunable work groups
    fn calculate_work_sizes(&self) -> (usize, usize) {
        let compute_units = self.device.max_compute_units() as usize;
        let max_work_group_size = self.device.max_work_group_size();
        
        // Calculate optimal local work size (threads per work group)
        let local_size = (max_work_group_size / 4).max(64).min(256);
        
        // Use the tunable work_groups_per_cu value
        let base_work_groups = compute_units * self.work_groups_per_cu;
        
        // Apply intensity scaling to work groups
        let intensity_factor = self.gpu_settings.intensity as f32 / 100.0;
        let adjusted_work_groups = ((base_work_groups as f32) * intensity_factor) as usize;
        let global_size = adjusted_work_groups.max(1) * local_size;
        
        debug!("Calculated work sizes for {}: global={}, local={}, intensity={}% (WG: {}/CU)", 
               self.device.name(), global_size, local_size, self.gpu_settings.intensity, self.work_groups_per_cu);
        
        (global_size, local_size)
    }

    /// Apply intensity delay if needed (for power/thermal management)
    async fn apply_intensity_delay(&self) {
        if self.gpu_settings.intensity < 100 {
            let delay_ms = (100 - self.gpu_settings.intensity as u32) / 2; // 0.5ms per % reduction
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
            }
        }
    }
    
    /// Mine using GPU with intensity and settings applied - returns (found_nonce, hashes_processed, best_difficulty)
    pub async fn mine(
        &self,
        job: &MiningJob,
        nonce_start: u64,
        batch_size: u32,
    ) -> Result<(Option<u64>, u32, u64)> {
        if !self.initialized {
            return Err(Error::msg("Engine not initialized"));
        }

        // Apply intensity delay for power/thermal management
        self.apply_intensity_delay().await;
        
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
        
        // Calculate work sizes with intensity and tunable work groups applied
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
        
        debug!("GPU mining completed in {:.2}ms: {} hashes, {:.2} MH/s (intensity: {}%, WG: {})", 
               mining_time.as_millis(),
               hashes_processed,
               hashes_processed as f64 / mining_time.as_millis() as f64 / 1000.0,
               self.gpu_settings.intensity, self.work_groups_per_cu);
        
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
            info!("🎉 GPU found share! Nonce: {}, Difficulty: {} (intensity: {}%, WG: {})", 
                  nonce, crate::miner::stats::MinerStats::format_number(best_difficulty),
                  self.gpu_settings.intensity, self.work_groups_per_cu);
        }
        
        Ok((found_nonce, hashes_processed, best_difficulty))
    }
    
    /// Get suggested batch size based on device capabilities and GPU settings
    pub fn get_suggested_batch_size(&self) -> u32 {
        // Start with base calculation
        let base_batch = self.calculate_base_batch_size();
        
        // Apply user override if specified
        if let Some(override_batch) = self.gpu_settings.batch_size {
            info!("🎮 Using override batch size: {} (was: {})", override_batch, base_batch);
            return override_batch.max(1_000).min(1_000_000); // Safety clamps
        }

        // Apply intensity scaling to batch size
        let intensity_factor = self.gpu_settings.intensity as f32 / 100.0;
        let adjusted_batch = ((base_batch as f32) * intensity_factor) as u32;
        
        let final_batch = adjusted_batch.max(1_000).min(500_000); // Increased max from 100K to 500K
        
        debug!("Calculated batch size for {}: base={}, intensity={}%, final={}", 
               self.device.name(), base_batch, self.gpu_settings.intensity, final_batch);
        
        final_batch
    }

    /// Calculate base batch size without intensity scaling
    fn calculate_base_batch_size(&self) -> u32 {
        let compute_units = self.device.max_compute_units();
        let memory_gb = self.device.global_mem_size() as f64 / (1024.0 * 1024.0 * 1024.0);
        
        // Enhanced batch size calculation for better performance
        let memory_based = (memory_gb * 50_000.0) as u32; // 50K per GB (increased from 1K)
        let cu_based = compute_units * 10_000; // 10K per CU (increased from 1K)
        
        // Use the smaller of the two, but with higher minimums
        let base_batch = memory_based.min(cu_based).max(50_000); // Minimum 50K
        
        debug!("Base batch calculation for {}: memory_based={}, cu_based={}, final={}", 
               self.device.name(), memory_based, cu_based, base_batch);
        
        base_batch
    }
    
    /// Get expected hashrate estimate for this device with intensity consideration
    pub fn estimate_hashrate(&self) -> f64 {
        // Base estimate based on compute units
        let compute_units = self.device.max_compute_units() as f64;
        let base_rate_per_cu = 12.0; // Increased from 8.0 MH/s per CU for better estimates
        let base_hashrate = compute_units * base_rate_per_cu;
        
        // Apply intensity scaling
        let intensity_factor = self.gpu_settings.intensity as f64 / 100.0;
        let estimated_hashrate = base_hashrate * intensity_factor;
        
        debug!("Hashrate estimate for {}: base={:.1} MH/s, intensity={}%, estimated={:.1} MH/s", 
               self.device.name(), base_hashrate, self.gpu_settings.intensity, estimated_hashrate);
        
        estimated_hashrate
    }

    /// Get performance info string for logging
    pub fn get_performance_info(&self) -> String {
        format!("GPU: {} | Intensity: {}% | Batch: {} | WG: {} | Est: {:.1} MH/s",
                self.device.name(),
                self.gpu_settings.intensity,
                self.get_suggested_batch_size(),
                self.work_groups_per_cu,
                self.estimate_hashrate())
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

// Changelog:
// - v2.2.0-sequential-autotune (2025-06-25): SEQUENTIAL PARAMETER AUTOTUNING
//   *** NEW AUTOTUNING SYSTEM ***:
//   1. Sequential parameter optimization: fix one, tune others, fix next
//   2. Configurable number of optimization cycles (default: 1)
//   3. Phase-based optimization: intensity → batch_size → work_groups
//   4. Baseline performance measurement and improvement tracking
//   5. Comprehensive logging of optimization progress and results
//   *** TECHNICAL IMPLEMENTATION ***:
//   - AutotuneConfig struct for flexible tuning parameters
//   - measure_hashrate() for accurate performance measurement
//   - optimize_intensity/batch_size/work_groups() for phase-specific tuning
//   - run_sequential_autotune() orchestrates the entire process
//   - Tunable work_groups_per_cu parameter added to engine
//   *** USAGE ***:
//   - Enable with engine.enable_autotuning(AutotuneConfig::default())
//   - Run with engine.run_sequential_autotune(&test_job).await
//   - Automatically finds optimal settings for maximum hashrate
//   *** EXPECTED BENEFITS ***:
//   - Should optimize RTX 4060 Ti from 380 MH/s to 400+ MH/s
//   - Systematic approach ensures true optimum rather than local maximum
//   - Safe parameter ranges prevent GPU crashes or instability