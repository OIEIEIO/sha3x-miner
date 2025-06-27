// GPU Mining Test - Standalone test to verify GPU mining works
// File: src/bin/gpu_test.rs
// Version: 1.0.1 - Added XN field support

use sha3x_miner::core::types::MiningJob;
use sha3x_miner::miner::gpu::opencl::{OpenClDevice, OpenClEngine};
use std::time::Instant;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("🎮 GPU Mining Test - Testing RTX 4060 Ti REAL Performance");
    
    // Detect GPU devices
    let devices = OpenClDevice::detect_devices()?;
    if devices.is_empty() {
        error!("❌ No GPU devices found");
        return Ok(());
    }
    
    let device = &devices[0];
    info!("🎮 Testing device: {}", device.info_string());
    
    // Initialize engine
    let mut engine = OpenClEngine::new(device.clone());
    engine.initialize()?;
    info!("✅ Engine initialized successfully");
    
    // Create a test job (dummy SHA3x mining job)
    let test_job = MiningJob {
        job_id: "test-job-001".to_string(),
        mining_hash: vec![
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
        ],
        target_difficulty: 1000000, // Easy target for testing
        height: 12345,
        algo: sha3x_miner::core::types::Algorithm::Sha3x,
        extranonce2: None, // ✅ Added XN field support - no XN for test job
        prev_hash: None,
        merkle_root: None,
        version: None,
        ntime: None,
        nbits: None,
        merkle_path: None,
        target: None,
    };
    
    info!("🎯 Starting GPU mining test...");
    info!("├─ Job ID: {}", test_job.job_id);
    info!("├─ Target difficulty: {}", test_job.target_difficulty);
    info!("├─ Algorithm: {:?}", test_job.algo);
    info!("└─ XN (extra nonce): {}", test_job.extranonce2.as_ref().unwrap_or(&"None".to_string()));
    
    let batch_size = engine.get_suggested_batch_size();
    info!("🔧 Batch size: {}", batch_size);
    
    // Run mining test for 10 seconds
    let test_duration = std::time::Duration::from_secs(10);
    let start_time = Instant::now();
    let mut total_hashes = 0u64;
    let mut nonce_offset = 0u64;
    let mut iteration = 0;
    
    info!("🚀 Starting REAL GPU mining test for 10 seconds...");
    
    while start_time.elapsed() < test_duration {
        iteration += 1;
        
        match engine.mine(&test_job, nonce_offset, batch_size).await {
            Ok((found_nonce, hashes_processed, best_difficulty)) => {
                total_hashes += hashes_processed as u64;
                
                if let Some(nonce) = found_nonce {
                    info!("🎉 FOUND SHARE! Nonce: {:016x}, Difficulty: {}", 
                          nonce, best_difficulty);
                }
                
                nonce_offset += hashes_processed as u64;
                
                // Progress update every 50 iterations
                if iteration % 50 == 0 {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let hashrate = total_hashes as f64 / elapsed / 1_000_000.0;
                    info!("📊 Progress: {:.1}s elapsed, {:.2} MH/s ACTUAL hashrate", 
                          elapsed, hashrate);
                }
            }
            Err(e) => {
                error!("❌ Mining error: {}", e);
                break;
            }
        }
    }
    
    // Final results
    let elapsed = start_time.elapsed().as_secs_f64();
    let average_hashrate = total_hashes as f64 / elapsed / 1_000_000.0;
    
    info!("🏁 GPU Mining Test Complete!");
    info!("├─ Duration: {:.2}s", elapsed);
    info!("├─ Total hashes: {}", total_hashes);
    info!("├─ ACTUAL MEASURED hashrate: {:.2} MH/s", average_hashrate);
    info!("├─ Estimated was: 272.0 MH/s");
    info!("├─ Actual vs Estimate: {:.1}%", (average_hashrate / 272.0) * 100.0);
    info!("├─ Iterations: {}", iteration);
    info!("└─ Status: {}", if average_hashrate > 50.0 { "✅ EXCELLENT!" } else { "⚠️ Needs optimization" });
    
    if average_hashrate > 100.0 {
        info!("🚀 Your RTX 4060 Ti is CRUSHING IT! Ready for real mining!");
        info!("💰 This is {}x faster than your current CPU mining!", 
              (average_hashrate / 1.05) as u32);
    } else if average_hashrate > 10.0 {
        info!("👍 Good performance! We can optimize further.");
    } else {
        info!("🔧 Lower than expected - kernel may need optimization.");
    }
    
    // Test LuckyPool XN nonce generation
    info!("🔧 Testing LuckyPool XN nonce generation...");
    
    // Create a test job with XN (simulating LuckyPool)
    let luckypool_test_job = MiningJob {
        job_id: "luckypool-test-001".to_string(),
        mining_hash: test_job.mining_hash.clone(),
        target_difficulty: 1000000,
        height: 12345,
        algo: sha3x_miner::core::types::Algorithm::Sha3x,
        extranonce2: Some("ad49".to_string()), // ✅ Simulate LuckyPool XN
        prev_hash: None,
        merkle_root: None,
        version: None,
        ntime: None,
        nbits: None,
        merkle_path: None,
        target: None,
    };
    
    // Test a few iterations with XN
    info!("🔧 Testing XN nonce format (simulating LuckyPool):");
    for i in 0..3 {
        match engine.mine(&luckypool_test_job, i * 100000, 1000).await {
            Ok((found_nonce, hashes_processed, best_difficulty)) => {
                if let Some(nonce) = found_nonce {
                    // This would be formatted with XN in the actual manager
                    info!("├─ Found nonce: {:016x} (would be formatted as XN + 6 bytes for LuckyPool)", nonce);
                }
                info!("├─ Test {}: {} hashes, best difficulty: {}", i + 1, hashes_processed, best_difficulty);
            }
            Err(e) => {
                error!("├─ XN test error: {}", e);
            }
        }
    }
    
    info!("✅ XN test complete - ready for LuckyPool integration!");
    
    Ok(())
}

// Changelog:
// - v1.0.1-xn-support (2025-06-26): Added XN field support for LuckyPool compatibility.
//   - Added extranonce2: None field to test MiningJob to fix compilation
//   - Added XN field logging to show when extra nonce is present
//   - Added LuckyPool XN simulation test with extranonce2: Some("ad49")
//   - Enhanced logging to show XN status in job information
//   - Ready for testing complete LuckyPool integration
// - v1.0.0 (2025-06-25): Initial GPU mining test
//   - Basic GPU performance testing
//   - Real hashrate measurement
//   - RTX 4060 Ti optimization validation