// SHA3x Miner - Free and Open Source Software Statement
//
// This project, sha3x-miner, is Free and Open Source Software (FOSS) licensed
// under the MIT License. You are free to use, modify, and distribute this
// software in accordance with the license terms. Contributions are welcome
// via pull requests to the project repository.
//
// File: src/benchmark/jobs.rs
// Version: 1.0.14
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// This file provides static benchmark jobs for testing mining performance
// without requiring pool connectivity, supporting both SHA3X and SHA-256.

use crate::core::types::{Algorithm, MiningJob};
use crate::core::difficulty::{U256, bits_to_target};


/// Benchmark-specific job configuration
#[derive(Debug, Clone)]
pub struct BenchmarkJob {
    /// Static mining job for testing
    pub mining_job: MiningJob,
    
    /// Expected shares per million hashes (for validation)
    pub expected_shares_per_mh: f64,
    
    /// Description of this benchmark job
    pub description: String,
}

/// Compute nBits from a given difficulty
fn difficulty_to_nbits(difficulty: f64) -> u32 {
    if difficulty <= 0.0 {
        return 0x207fffff; // Maximum target (easiest difficulty)
    }

    // For Bitcoin: difficulty 1 corresponds to max_target (0x00000000FFFF0000...)
    // Higher difficulty means lower target (harder to mine)
    // target = max_target / difficulty
    
    // Special handling for very low difficulties
    if difficulty < 0.00000001 {
        return 0x207fffff; // Maximum possible target
    }
    
    // Calculate target = max_target / difficulty
    // max_target for diff 1 is 0x00000000FFFF0000000000000000000000000000000000000000000000000000
    let max_diff1_target = U256::from_big_endian(&[
        0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    
    // Scale difficulty to avoid precision loss
    let scale_factor = 1_000_000u64;
    let scaled_difficulty = (difficulty * scale_factor as f64) as u64;
    let scaled_max_target = max_diff1_target * U256::from(scale_factor);
    let target = scaled_max_target / U256::from(scaled_difficulty);
    
    // Convert target to compact nBits format
    let target_bytes = target.to_big_endian();
    
    // Find first non-zero byte
    let mut leading_zeros = 0;
    for &byte in target_bytes.iter() {
        if byte != 0 {
            break;
        }
        leading_zeros += 1;
    }
    
    if leading_zeros >= 32 {
        return 0x00000000; // Target is zero (impossible difficulty)
    }
    
    // Extract mantissa (first 3 non-zero bytes)
    let mantissa_start = leading_zeros;
    let mut mantissa = 0u32;
    
    if mantissa_start < 32 {
        mantissa |= (target_bytes[mantissa_start] as u32) << 16;
    }
    if mantissa_start + 1 < 32 {
        mantissa |= (target_bytes[mantissa_start + 1] as u32) << 8;
    }
    if mantissa_start + 2 < 32 {
        mantissa |= target_bytes[mantissa_start + 2] as u32;
    }
    
    // Calculate exponent (bytes from right)
    let exponent = 32 - leading_zeros;
    
    // Handle negative flag in mantissa (Bitcoin nBits format)
    if mantissa & 0x00800000 != 0 {
        // If the high bit of the mantissa is set, we need to shift right and increase exponent
        mantissa >>= 8;
        let exponent = exponent + 1;
        if exponent > 0xFF {
            return 0x00000000; // Overflow
        }
        ((exponent as u32) << 24) | mantissa
    } else {
        ((exponent as u32) << 24) | mantissa
    }
}

/// Get the max target for a specific algorithm
pub fn get_max_target(algo: Algorithm) -> U256 {
    match algo {
        Algorithm::Sha256 => {
            // Bitcoin max target: 0x00000000FFFF0000...
            U256::from_big_endian(&[
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ])
        }
        Algorithm::Sha3x => {
            // SHA3x max target: 0xFFFFFFFFFFFFFFFF... (all ones)
            U256::from_big_endian(&[
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            ])
        }
    }
}

/// Calculate difficulty from nBits for SHA-256 jobs
pub fn calculate_difficulty_from_nbits(nbits: u32) -> f64 {
    // Bitcoin difficulty calculation
    // difficulty = max_target / current_target
    // where max_target is the target for difficulty 1
    
    let max_diff1_target = U256::from_big_endian(&[
        0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    
    // Decode nBits to target
    let exponent = (nbits >> 24) as u8;
    let mantissa = nbits & 0x00FFFFFF;
    
    if exponent == 0 || mantissa == 0 {
        return 0.0; // Invalid nBits
    }
    
    // Calculate target from nBits
    let mut target = U256::from(mantissa);
    if exponent > 3 {
        target = target << (8 * (exponent - 3) as usize);
    } else {
        target = target >> (8 * (3 - exponent) as usize);
    }
    
    if target.is_zero() {
        return f64::INFINITY;
    }
    
    // Calculate difficulty = max_target / target
    // To avoid overflow, we scale both values
    let scale = 1_000_000_000u64;
    let scaled_max = max_diff1_target * U256::from(scale);
    let scaled_target = target;
    
    if scaled_target > scaled_max {
        // Target is higher than max_target, difficulty < 1
        let ratio = scaled_target / max_diff1_target;
        if ratio.low_u64() == 0 {
            return 0.000001; // Very low difficulty
        }
        1.0 / (ratio.low_u64() as f64)
    } else {
        // Normal case: difficulty >= 1
        let ratio = scaled_max / scaled_target;
        (ratio.low_u64() as f64) / (scale as f64)
    }
}

/// Create a collection of test jobs with different characteristics
pub fn create_test_jobs() -> Vec<BenchmarkJob> {
    vec![
        create_easy_job(),
        create_medium_job(),
        create_hard_job(),
        create_realistic_job(),
    ]
}

/// Create an easy job that finds shares frequently (for quick testing)
pub fn create_easy_job() -> BenchmarkJob {
    create_easy_sha3x_job(1000.0) // Default to SHA3X with difficulty 1000
}

/// Create a medium difficulty job (good for general testing)
pub fn create_medium_job() -> BenchmarkJob {
    create_medium_sha3x_job(100000.0) // Default to SHA3X with difficulty 100000
}

/// Create a hard job (tests performance under realistic conditions)
pub fn create_hard_job() -> BenchmarkJob {
    create_hard_sha3x_job(10000000.0) // Default to SHA3X with difficulty 10000000
}

/// Create a realistic job matching typical Tari network conditions
pub fn create_realistic_job() -> BenchmarkJob {
    create_medium_sha3x_job(100000.0) // Default to SHA3X with difficulty 100000
}

/// Get a job by difficulty level and algorithm for targeted testing
pub fn get_job_by_difficulty_and_algo(difficulty: f64, algorithm: Algorithm) -> BenchmarkJob {
    match algorithm {
        Algorithm::Sha3x => {
            if difficulty <= 10000.0 {
                create_easy_sha3x_job(difficulty)
            } else if difficulty <= 1000000.0 {
                create_medium_sha3x_job(difficulty)
            } else {
                create_hard_sha3x_job(difficulty)
            }
        }
        Algorithm::Sha256 => {
            if difficulty <= 10000.0 {
                create_easy_sha256_job(difficulty)
            } else if difficulty <= 1000000.0 {
                create_medium_sha256_job(difficulty)
            } else {
                create_hard_sha256_job(difficulty)
            }
        }
    }
}

/// Get a job by difficulty level for targeted testing (original function)
pub fn get_job_by_difficulty(difficulty: u64) -> BenchmarkJob {
    get_job_by_difficulty_and_algo(difficulty as f64, Algorithm::Sha3x) // Default to SHA3X
}

// SHA-256 specific jobs
fn create_easy_sha256_job(difficulty: f64) -> BenchmarkJob {
    let nbits = difficulty_to_nbits(difficulty);
    let target = bits_to_target(nbits);
    let target_bytes = target.to_big_endian();
    let actual_difficulty = calculate_difficulty_from_nbits(nbits);

    BenchmarkJob {
        mining_job: MiningJob {
            job_id: "bench_sha256_easy_001".to_string(),
            mining_hash: create_bitcoin_header(nbits),
            target_difficulty: 0, // Placeholder, unused for SHA-256
            height: 665, // From SV2 job 663
            algo: Algorithm::Sha256,
            extranonce2: None,
            prev_hash: None,
            merkle_root: None,
            version: Some(536870912), // From SV2
            ntime: Some(1750191225), // From SV2
            nbits: Some(nbits),
            merkle_path: None,
            target: Some(target_bytes),
        },
        expected_shares_per_mh: 1000000.0 / actual_difficulty,
        description: format!("Easy SHA-256 job - difficulty ~{:.10}", actual_difficulty),
    }
}

fn create_medium_sha256_job(difficulty: f64) -> BenchmarkJob {
    let nbits = difficulty_to_nbits(difficulty);
    let target = bits_to_target(nbits);
    let target_bytes = target.to_big_endian();
    let actual_difficulty = calculate_difficulty_from_nbits(nbits);

    BenchmarkJob {
        mining_job: MiningJob {
            job_id: "bench_sha256_medium_001".to_string(),
            mining_hash: create_bitcoin_header(nbits),
            target_difficulty: 0, // Placeholder, unused for SHA-256
            height: 665,
            algo: Algorithm::Sha256,
            extranonce2: None,
            prev_hash: None,
            merkle_root: None,
            version: Some(536870912),
            ntime: Some(1750191225),
            nbits: Some(nbits),
            merkle_path: None,
            target: Some(target_bytes),
        },
        expected_shares_per_mh: 1000000.0 / actual_difficulty,
        description: format!("Medium SHA-256 job - difficulty ~{:.10}", actual_difficulty),
    }
}

fn create_hard_sha256_job(difficulty: f64) -> BenchmarkJob {
    let nbits = difficulty_to_nbits(difficulty);
    let target = bits_to_target(nbits);
    let target_bytes = target.to_big_endian();
    let actual_difficulty = calculate_difficulty_from_nbits(nbits);

    BenchmarkJob {
        mining_job: MiningJob {
            job_id: "bench_sha256_hard_001".to_string(),
            mining_hash: create_bitcoin_header(nbits),
            target_difficulty: 0, // Placeholder, unused for SHA-256
            height: 665,
            algo: Algorithm::Sha256,
            extranonce2: None,
            prev_hash: None,
            merkle_root: None,
            version: Some(536870912),
            ntime: Some(1750191225),
            nbits: Some(nbits),
            merkle_path: None,
            target: Some(target_bytes),
        },
        expected_shares_per_mh: 1000000.0 / actual_difficulty,
        description: format!("Hard SHA-256 job - difficulty ~{:.10}", actual_difficulty),
    }
}

// SHA3X specific jobs
fn create_easy_sha3x_job(difficulty: f64) -> BenchmarkJob {
    let u64_difficulty = difficulty.round().max(1.0) as u64;
    BenchmarkJob {
        mining_job: MiningJob {
            job_id: "bench_sha3x_easy_001".to_string(),
            mining_hash: vec![
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
                0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10,
            ],
            target_difficulty: u64_difficulty,
            height: 100000,
            algo: Algorithm::Sha3x,
            extranonce2: None,
            prev_hash: None,
            merkle_root: None,
            version: None,
            ntime: None,
            nbits: None,
            merkle_path: None,
            target: None,
        },
        expected_shares_per_mh: 1000000.0 / difficulty,
        description: format!("Easy SHA3x job - difficulty ~{}", u64_difficulty),
    }
}

fn create_medium_sha3x_job(difficulty: f64) -> BenchmarkJob {
    let u64_difficulty = difficulty.round().max(1.0) as u64;
    BenchmarkJob {
        mining_job: MiningJob {
            job_id: "bench_sha3x_medium_001".to_string(),
            mining_hash: vec![
                0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x07, 0x18,
                0x29, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e, 0x8f, 0x90,
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                0x0f, 0xed, 0xcb, 0xa9, 0x87, 0x65, 0x43, 0x21,
            ],
            target_difficulty: u64_difficulty,
            height: 200000,
            algo: Algorithm::Sha3x,
            extranonce2: None,
            prev_hash: None,
            merkle_root: None,
            version: None,
            ntime: None,
            nbits: None,
            merkle_path: None,
            target: None,
        },
        expected_shares_per_mh: 1000000.0 / difficulty,
        description: format!("Medium SHA3x job - difficulty ~{}", u64_difficulty),
    }
}

fn create_hard_sha3x_job(difficulty: f64) -> BenchmarkJob {
    let u64_difficulty = difficulty.round().max(1.0) as u64;
    BenchmarkJob {
        mining_job: MiningJob {
            job_id: "bench_sha3x_hard_001".to_string(),
            mining_hash: vec![
                0x5a, 0x6b, 0x7c, 0x8d, 0x9e, 0xaf, 0xb0, 0xc1,
                0xd2, 0xe3, 0xf4, 0x05, 0x16, 0x27, 0x38, 0x49,
                0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11,
                0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99,
            ],
            target_difficulty: u64_difficulty,
            height: 300000,
            algo: Algorithm::Sha3x,
            extranonce2: None,
            prev_hash: None,
            merkle_root: None,
            version: None,
            ntime: None,
            nbits: None,
            merkle_path: None,
            target: None,
        },
        expected_shares_per_mh: 1000000.0 / difficulty,
        description: format!("Hard SHA3x job - difficulty ~{}", u64_difficulty),
    }
}

/// Create a realistic 80-byte Bitcoin header for SHA-256 testing
/// Uses SV2 job 663 data with dynamic nBits based on specified difficulty
fn create_bitcoin_header(nbits: u32) -> Vec<u8> {
    // Previous block hash (little-endian): from SV2 job 663
    let _prev_hash = [
        0x24, 0x4d, 0xa8, 0xb6, 0x49, 0x52, 0x0d, 0xcd, 0x21, 0xbf, 0xb9, 0xf7, 0xb8, 0x46,
        0x67, 0x08, 0x25, 0xb1, 0x08, 0x51, 0x70, 0xb2, 0xbc, 0x85, 0x8f, 0xac, 0xb7, 0xb3,
        0x00, 0x00, 0x00, 0x00,
    ];

    // Merkle root (little-endian): from SV2 job 663
    let _merkle_root = [
        0xc5, 0xe6, 0xe5, 0xb7, 0xb7, 0xe3, 0xb9, 0xb7, 0xc8, 0x62, 0x8a, 0x6d, 0x7f, 0x3e,
        0x0b, 0xf9, 0x18, 0x3b, 0x12, 0xf1, 0xd3, 0xe8, 0x00, 0xb4, 0x2e, 0x7b, 0xf7, 0x73,
        0x00, 0x00, 0x00, 0x00,
    ];

    let nbits_bytes = nbits.to_le_bytes();
    vec![
        // Version (4 bytes, little-endian): 0x20000000 (536870912)
        0x00, 0x00, 0x00, 0x20,
        // Previous block hash (32 bytes, little-endian): from SV2
        0x24, 0x4d, 0xa8, 0xb6, 0x49, 0x52, 0x0d, 0xcd, 0x21, 0xbf, 0xb9, 0xf7, 0xb8, 0x46,
        0x67, 0x08, 0x25, 0xb1, 0x08, 0x51, 0x70, 0xb2, 0xbc, 0x85, 0x8f, 0xac, 0xb7, 0xb3,
        0x00, 0x00, 0x00, 0x00,
        // Merkle root (32 bytes, little-endian): from SV2
        0xc5, 0xe6, 0xe5, 0xb7, 0xb7, 0xe3, 0xb9, 0xb7, 0xc8, 0x62, 0x8a, 0x6d, 0x7f, 0x3e,
        0x0b, 0xf9, 0x18, 0x3b, 0x12, 0xf1, 0xd3, 0xe8, 0x00, 0xb4, 0x2e, 0x7b, 0xf7, 0x73,
        0x00, 0x00, 0x00, 0x00,
        // Timestamp (4 bytes, little-endian): 1750191225 (2025-06-17 19:07:45 UTC)
        0x29, 0xe5, 0x50, 0x68,
        // nBits (4 bytes, little-endian): computed from difficulty
        nbits_bytes[0], nbits_bytes[1], nbits_bytes[2], nbits_bytes[3],
        // Nonce (4 bytes, little-endian): initialized to 0
        0x00, 0x00, 0x00, 0x00,
    ]
}

/// Create a custom job with specified difficulty
pub fn create_custom_job(difficulty: u64, job_suffix: &str) -> BenchmarkJob {
    BenchmarkJob {
        mining_job: MiningJob {
            job_id: format!("bench_custom_{}", job_suffix),
            mining_hash: vec![
                0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe,
                0xfe, 0xed, 0xfa, 0xce, 0xd0, 0x0d, 0x00, 0x00,
                0x13, 0x37, 0x42, 0x69, 0x96, 0x24, 0x73, 0x31,
                0xc0, 0xff, 0xee, 0x15, 0x60, 0x0d, 0xf0, 0x0d,
            ],
            target_difficulty: difficulty,
            height: 500000,
            algo: Algorithm::Sha3x,
            extranonce2: None,
            prev_hash: None,
            merkle_root: None,
            version: None,
            ntime: None,
            nbits: None,
            merkle_path: None,
            target: None,
        },
        expected_shares_per_mh: 1000000.0 / difficulty as f64,
        description: format!("Custom job - difficulty {} for specific testing", difficulty),
    }
}

/// Validate a benchmark job's expected metrics
pub fn validate_job_metrics(job: &BenchmarkJob, actual_shares: u64, total_hashes: u64) -> bool {
    if total_hashes == 0 {
        return false;
    }
    
    let actual_shares_per_mh = (actual_shares as f64 * 1_000_000.0) / total_hashes as f64;
    let tolerance = 0.5; // 50% tolerance for randomness
    let expected = job.expected_shares_per_mh;
    
    actual_shares_per_mh >= expected * (1.0 - tolerance) && 
    actual_shares_per_mh <= expected * (1.0 + tolerance)
}

// Changelog:
// - v1.0.16 (2025-06-17): Fixed SHA-256 difficulty calculations.
//   - Rewrote difficulty_to_nbits to properly convert difficulty to Bitcoin nBits format.
//   - Fixed calculate_difficulty_from_nbits to handle difficulty < 1 cases correctly.
//   - Used standard Bitcoin difficulty 1 max_target (0x00000000FFFF0000...) in calculations.
//   - Removed excessive scaling factor that was making difficulties too high.
//   - Compatible with main.rs v1.0.7, runner.rs v1.0.28, types.rs v1.0.5, sha256.rs v1.0.1, difficulty.rs v1.2.4.
// - v1.0.15 (2025-06-17): Added algorithm-specific max targets.
//   - Added get_max_target function to provide correct max targets for each algorithm.
//   - SHA3x uses full 256-bit max target (all 0xFF) for correct difficulty calculation.
//   - sha256 uses Bitcoin's max target (0x00000000FFFF0000...) for standard difficulty.
//   - Updated calculate_difficulty_from_nbits to use get_max_target(Algorithm::sha256).
//   - Compatible with main.rs v1.0.7, runner.rs v1.0.28, types.rs v1.0.5, sha256.rs v1.0.1, difficulty.rs v1.2.4.
// - v1.0.14 (2025-06-17): Fixed compilation errors with U256::from_nbits.
//   - Replaced U256::from_nbits(nbits) with bits_to_target(nbits) using existing function from difficulty.rs.
//   - Added bits_to_target import to use the proper function for converting nBits to U256 target.
//   - Compatible with main.rs v1.0.7, runner.rs v1.0.27, types.rs v1.0.5, sha256.rs v1.0.1, difficulty.rs v1.2.4.