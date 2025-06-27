// SHA3x Miner - Free and Open Source Software Statement
//
// This project, sha3x-miner, is Free and Open Source Software (FOSS) licensed
// under the MIT License. You are free to use, modify, and distribute this
// software in accordance with the license terms. Contributions are welcome
// via pull requests to the project repository.
//
// File: src/miner/cpu/thread.rs
// Version: 1.1.4
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// This file contains the implementation of individual mining threads for the
// SHA3x miner, located in the cpu subdirectory of the miner module. It handles
// nonce iteration, hash computation, and share detection for CPU mining.

use crate::core::{calculate_difficulty, Algorithm, MiningJob, sha3x::sha3x_hash_with_nonce_batch, sha256::sha256d_hash_with_nonce_batch, difficulty::{bits_to_target, U256}};
use crate::miner::stats::{MinerStats, ThreadStats};
use tokio::sync::mpsc::UnboundedSender as MpscSender;
use tokio::sync::broadcast::Receiver as BroadcastReceiver;
use hex;
use rand::{rngs::ThreadRng, Rng};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};
use std::time::{Instant, Duration};
use tracing::{info, debug, error};

pub fn start_mining_thread(
    thread_id: usize,
    num_threads: usize,
    job_rx: BroadcastReceiver<MiningJob>,
    share_tx: MpscSender<(String, String, String, usize, u64, String, u32)>,
    thread_stats: Arc<ThreadStats>,
    stats: Arc<MinerStats>,
) {
    let should_stop = Arc::new(AtomicBool::new(false));
    let should_stop_clone = Arc::clone(&should_stop);

    std::thread::spawn(move || {
        mining_thread(
            thread_id,
            num_threads,
            job_rx,
            share_tx,
            thread_stats,
            stats,
            should_stop_clone,
        );
    });
}

fn mining_thread(
    thread_id: usize,
    num_threads: usize,
    mut job_rx: BroadcastReceiver<MiningJob>,
    share_tx: MpscSender<(String, String, String, usize, u64, String, u32)>,
    thread_stats: Arc<ThreadStats>,
    stats: Arc<MinerStats>,
    should_stop: Arc<AtomicBool>,
) {
    let mut rng: ThreadRng = rand::thread_rng();
    let mut current_job: Option<MiningJob> = None;
    let mut hash_count = 0u64;
    let mut last_report = Instant::now();

    loop {
        if should_stop.load(Ordering::Relaxed) {
            break;
        }

        match job_rx.try_recv() {
            Ok(job) => {
                debug!("Thread {}: Received job {}, target_difficulty={:016x}, algo={:?}", 
                    thread_id, job.job_id, job.target_difficulty, job.algo);
                thread_stats.current_difficulty_target.store(job.target_difficulty, Ordering::Relaxed);
                current_job = Some(job);
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {},
            Err(_) => break,
        }

        if let Some(ref job) = current_job {
            match job.algo {
                Algorithm::Sha3x => {
                    let mut nonce = rng.r#gen::<u64>();
                    nonce = nonce.wrapping_add(thread_id as u64);

                    for _ in (0..1000).step_by(4) {
                        let batch_results = sha3x_hash_with_nonce_batch(&job.mining_hash, nonce);
                        
                        for (hash, batch_nonce) in batch_results.iter() {
                            let difficulty = calculate_difficulty(hash, job.algo);
                            hash_count += 1;

                            if difficulty >= job.target_difficulty {
                                // Convert local nonce to bytes (use only 6 least-significant bytes)
let local_nonce = batch_nonce.to_le_bytes();

// Decode XN (2 bytes) from job.extranonce2
let xn = hex::decode(&job.extranonce2.clone().unwrap_or_default()).unwrap_or(vec![0, 0]);

// Compose full 8-byte nonce: [XN][Local Nonce]
let full_nonce = [
    xn.get(0).copied().unwrap_or(0),
    xn.get(1).copied().unwrap_or(0),
    local_nonce[0],
    local_nonce[1],
    local_nonce[2],
    local_nonce[3],
    local_nonce[4],
    local_nonce[5],
];

// Encode as hex string
let nonce_hex_le = hex::encode(full_nonce);
                                let nonce_hex_be = hex::encode(batch_nonce.to_be_bytes());
                                let result_hex = hex::encode(hash);

                                thread_stats.record_share(difficulty, true);
                                stats.record_share_found(thread_id, difficulty, job.target_difficulty, true);

                                info!("💎 Thread {} found SHA3X share! Difficulty: {}, Target: {}", 
                                    thread_id, 
                                    MinerStats::format_number(difficulty), 
                                    MinerStats::format_number(job.target_difficulty));
                                
                                info!("🔍 Share details - Nonce LE: {}, Nonce BE: {}, Hash: {}", 
                                    nonce_hex_le, nonce_hex_be, result_hex);

                                stats.add_activity(format!(
                                    "💎 Thread {} found SHA3X share! Difficulty: {}",
                                    thread_id,
                                    MinerStats::format_number(difficulty)
                                ));

                                let _ = share_tx.send((
                                    job.job_id.clone(),
                                    nonce_hex_le,
                                    result_hex,
                                    thread_id,
                                    difficulty,
                                    String::new(),
                                    0,
                                ));

                                stats.shares_submitted.fetch_add(1, Ordering::Relaxed);
                            }
                        }

                        nonce = nonce.wrapping_add((4 * num_threads) as u64);
                    }
                }
                Algorithm::Sha256 => {
                    let header = build_bitcoin_header(job);
                    if header.len() != 80 {
                        error!("Thread {}: Invalid Bitcoin header length: {} bytes", thread_id, header.len());
                        continue;
                    }
                    debug!("Thread {}: Header: {}", thread_id, hex::encode(&header));

                    let ntime = job.ntime.unwrap_or_else(|| {
                        error!("Thread {}: No ntime provided for SHA-256 job", thread_id);
                        0
                    });

                    static EXTRANONCE2_COUNTER: AtomicU32 = AtomicU32::new(0);
                    let extranonce2_bytes = EXTRANONCE2_COUNTER.fetch_add(1, Ordering::SeqCst).to_le_bytes();
                    let extranonce2 = hex::encode(extranonce2_bytes);
                    debug!("Thread {}: Extranonce2: {}", thread_id, extranonce2);

                    let target = if let Some(target_bytes) = job.target {
                        if target_bytes.len() != 32 {
                            error!("Thread {}: Invalid target length: {} bytes", thread_id, target_bytes.len());
                            continue;
                        }
                        let target_u256 = U256::from_big_endian(&target_bytes); // Target is big-endian for comparison
                        if target_u256.is_zero() {
                            error!("Thread {}: Zero target for job {}, skipping", thread_id, job.job_id);
                            continue;
                        }
                        target_u256
                    } else if let Some(nbits) = job.nbits {
                        let target_u256 = bits_to_target(nbits);
                        if target_u256.is_zero() {
                            error!("Thread {}: Zero target from nbits {:08x}, skipping", thread_id, nbits);
                            continue;
                        }
                        target_u256
                    } else {
                        error!("Thread {}: No target or nbits for job {}, skipping", thread_id, job.job_id);
                        continue;
                    };
                    debug!("Thread {}: Target: {:064x}", thread_id, target);

                    let mut nonce = rng.r#gen::<u32>() as u64;
                    nonce = nonce.wrapping_add(thread_id as u64);

                    for _ in (0..1000).step_by(4) {
                        let batch_results = sha256d_hash_with_nonce_batch(&header, nonce as u32);
                        
                        for (hash, batch_nonce) in batch_results.iter() {
                            hash_count += 1;
                            let hash_u256 = U256::from_big_endian(hash);
                            debug!("Thread {}: Hash: {:064x}, Nonce: {:08x}", thread_id, hash_u256, batch_nonce);

                            if hash_u256 <= target {
                                let difficulty = calculate_difficulty(hash, job.algo);
                                let nonce_hex = format!("{:08x}", batch_nonce);
                                let result_hex = hex::encode(hash);

                                thread_stats.record_share(difficulty, true);
                                stats.record_share_found(thread_id, difficulty, job.target_difficulty, true);

                                info!("💎 Thread {} found Bitcoin share! Difficulty: {}, Target: {:x}", 
                                    thread_id, MinerStats::format_number(difficulty), target);
                                info!("🔍 Share details - Job: {}, Nonce: {}, Extranonce2: {}, Ntime: {:08x}, Hash: {}", 
                                    job.job_id, nonce_hex, extranonce2, ntime, result_hex);

                                stats.add_activity(format!(
                                    "💎 Thread {} found Bitcoin share! Difficulty: {}",
                                    thread_id, MinerStats::format_number(difficulty)
                                ));

                                let _ = share_tx.send((
                                    job.job_id.clone(),
                                    nonce_hex,
                                    result_hex,
                                    thread_id,
                                    difficulty,
                                    extranonce2.clone(),
                                    ntime,
                                ));

                                stats.shares_submitted.fetch_add(1, Ordering::Relaxed);
                            }
                        }

                        nonce = nonce.wrapping_add((4 * num_threads) as u64);
                    }
                }
            }

            if last_report.elapsed() > Duration::from_secs(1) {
                thread_stats.update_hashrate(hash_count);
                stats.hashes_computed.fetch_add(hash_count, Ordering::Relaxed);
                stats.update_hashrate_history(stats.hashes_computed.load(Ordering::Relaxed));
                debug!("Thread {}: Hashrate: {} H/s", thread_id, hash_count);
                hash_count = 0;
                last_report = Instant::now();
            }
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn build_bitcoin_header(job: &MiningJob) -> Vec<u8> {
    let mut header = Vec::with_capacity(80);
    
    if let Some(version) = job.version {
        header.extend_from_slice(&version.to_le_bytes());
    } else {
        header.extend_from_slice(&[0u8, 0, 0, 2]); // Version 2.0.0
    }
    
    if let Some(prev_hash) = &job.prev_hash {
        if prev_hash.len() == 32 {
            let mut swapped = [0u8; 32];
            for i in 0..8 {
                swapped[i * 4..(i + 1) * 4].copy_from_slice(&prev_hash[(7 - i) * 4..(8 - i) * 4]);
            }
            header.extend_from_slice(&swapped); // Swap to little-endian
        } else {
            error!("Invalid prev_hash length: {} bytes", prev_hash.len());
            header.extend_from_slice(&[0u8; 32]);
        }
    } else {
        header.extend_from_slice(&[0u8; 32]);
    }
    
    if let Some(merkle_root) = &job.merkle_root {
        if merkle_root.len() == 32 {
            let mut swapped = [0u8; 32];
            for i in 0..8 {
                swapped[i * 4..(i + 1) * 4].copy_from_slice(&merkle_root[(7 - i) * 4..(8 - i) * 4]);
            }
            header.extend_from_slice(&swapped); // Swap to little-endian
        } else {
            error!("Invalid merkle_root length: {} bytes", merkle_root.len());
            header.extend_from_slice(&[0u8; 32]);
        }
    } else {
        header.extend_from_slice(&[0u8; 32]);
    }
    
    if let Some(ntime) = job.ntime {
        header.extend_from_slice(&ntime.to_le_bytes());
    } else {
        header.extend_from_slice(&[0u8, 0, 0, 0]);
    }
    
    if let Some(nbits) = job.nbits {
        header.extend_from_slice(&nbits.to_le_bytes());
    } else {
        header.extend_from_slice(&[0x3B, 0x43, 0x06, 0x19]); // Default nbits
    }
    
    header.extend_from_slice(&[0u8; 4]); // Nonce placeholder
    
    if header.len() != 80 {
        error!("Bitcoin header invalid: {} bytes, expected 80", header.len());
    }
    debug!("Built header: {}", hex::encode(&header));
    header
}

// Changelog:
// - v1.1.4 (2025-06-19): Fixed SHA-256 share validation.
//   - Changed target to use from_big_endian in SHA-256 branch.
//   - Added byte swapping for prev_hash and merkle_root to match Bitcoin's little-endian header.
//   - Preserved SHA3x logic unchanged.
//   - Compatible with miner.rs v1.2.30, difficulty.rs v1.2.10, sha256.rs v1.0.4, protocol.rs v1.0.1.