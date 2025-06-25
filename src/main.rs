// SHA3x Miner - Free and Open Source Software Statement
//
// File: src/main.rs
// Version: 2.0.0-feature-based
// Developer: OIEIEIO <oieieio@protonmail.com>
//
// Feature-based mining: --features cpu, --features gpu, --features hybrid

use sha3x_miner::{
    core::types::{Args, Algorithm}, 
    miner::CpuMiner, 
    benchmark::runner::BenchmarkRunner, 
    Result
};
use clap::Parser;
use tracing::info;
use tracing_subscriber;

// Web server module for real-time mining dashboard
mod web_server;

// Ensure exactly one feature is selected
#[cfg(not(any(feature = "cpu", feature = "gpu", feature = "hybrid")))]
compile_error!("Must specify one feature: --features cpu, --features gpu, or --features hybrid");

#[cfg(all(feature = "cpu", feature = "gpu"))]
compile_error!("Cannot use both --features cpu and --features gpu. Use --features hybrid for both.");

#[cfg(all(feature = "cpu", feature = "hybrid"))]
compile_error!("Cannot use both --features cpu and --features hybrid. Choose one.");

#[cfg(all(feature = "gpu", feature = "hybrid"))]
compile_error!("Cannot use both --features gpu and --features hybrid. Choose one.");

//
// CPU-ONLY MINING MODE
//
#[cfg(feature = "cpu")]
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Check for SV2 test mode first
    if args.test_sv2 {
        return handle_sv2_test(&args).await;
    }

    // Validate arguments
    if let Err(err) = args.validate() {
        eprintln!("❌ Error: {}", err);
        std::process::exit(1);
    }

    // Initialize tracing only if TUI is disabled
    #[cfg(not(feature = "tui"))]
    tracing_subscriber::fmt::init();

    let algo = parse_algorithm(&args.algo)?;

    if args.benchmark {
        return handle_benchmark(&args, algo).await;
    } else {
        return handle_cpu_mining(&args, algo).await;
    }
}

//
// GPU-ONLY MINING MODE
//
#[cfg(feature = "gpu")]
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Check for SV2 test mode first
    if args.test_sv2 {
        return handle_sv2_test(&args).await;
    }

    // Validate arguments
    if let Err(err) = args.validate() {
        eprintln!("❌ Error: {}", err);
        std::process::exit(1);
    }

    // Initialize tracing only if TUI is disabled
    #[cfg(not(feature = "tui"))]
    tracing_subscriber::fmt::init();

    let algo = parse_algorithm(&args.algo)?;

    if args.benchmark {
        return handle_benchmark(&args, algo).await;
    } else {
        return handle_gpu_mining(&args, algo).await;
    }
}

//
// HYBRID MINING MODE (CPU + GPU)
//
#[cfg(feature = "hybrid")]
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Check for SV2 test mode first
    if args.test_sv2 {
        return handle_sv2_test(&args).await;
    }

    // Validate arguments
    if let Err(err) = args.validate() {
        eprintln!("❌ Error: {}", err);
        std::process::exit(1);
    }

    // Initialize tracing only if TUI is disabled
    #[cfg(not(feature = "tui"))]
    tracing_subscriber::fmt::init();

    let algo = parse_algorithm(&args.algo)?;

    if args.benchmark {
        return handle_benchmark(&args, algo).await;
    } else {
        return handle_hybrid_mining(&args, algo).await;
    }
}

//
// SHARED HELPER FUNCTIONS
//

async fn handle_sv2_test(args: &Args) -> Result<()> {
    // Initialize tracing for SV2 test
    #[cfg(not(feature = "tui"))]
    tracing_subscriber::fmt::init();

    info!("🔧 SV2 Connection Test Mode");
    
    // Validate required arguments for SV2 test
    let pool_address = match &args.pool {
        Some(pool) => pool,
        None => {
            eprintln!("❌ Error: --pool is required for SV2 testing");
            eprintln!("Example: cargo run --release --features cpu -- --test-sv2 --pool 127.0.0.1:34254");
            std::process::exit(1);
        }
    };

    info!("🎯 Target JDS: {}", pool_address);

    // Create a test miner instance - pass pool address as string
    let miner = CpuMiner::new(
        "test-wallet".to_string(), // Dummy wallet for SV2 test
        pool_address.clone(), // Pass as string, miner will resolve DNS
        "sv2-test-worker".to_string(),
        1, // Single thread for test
        Algorithm::Sha3x, // Algorithm doesn't matter for connection test
    );

    // Run SV2 connection test
    match miner.test_sv2_connection().await {
        Ok(()) => {
            info!("✅ TCP connection to JDS successful");
            info!("❌ Noise protocol not implemented yet");
            info!("🔧 Next: Implement noise_sv2 handshake with step_0/step_2");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("❌ SV2 connection test failed: {}", e);
            eprintln!("💡 Make sure JDS is running and accepting connections");
            eprintln!("💡 Check the JDS address and port");
            std::process::exit(1);
        }
    }
}

fn parse_algorithm(algo_str: &str) -> Result<Algorithm> {
    let algo = match algo_str {
        "sha3x" => Algorithm::Sha3x,
        "sha256" => Algorithm::Sha256,
        _ => {
            eprintln!("❌ Invalid algorithm: {}", algo_str);
            std::process::exit(1);
        }
    };
    Ok(algo)
}

async fn handle_benchmark(args: &Args, algo: Algorithm) -> Result<()> {
    info!("🧪 Starting Benchmark Mode (Algo: {:?})", algo);
    info!("🧵 Threads: {}", if args.threads == 0 { "auto".to_string() } else { args.threads.to_string() });
    info!("⏱️ Duration: {}s", args.benchmark_duration);
    info!("🎯 Target difficulty: {:.10}", args.benchmark_difficulty);

    let benchmark_runner = BenchmarkRunner::new(
        args.threads,
        args.benchmark_duration,
        args.benchmark_difficulty,
        algo,
    );

    let result = benchmark_runner.run().await?;
    
    info!("📊 Benchmark Complete!");
    info!("🧪 Algorithm: {:?}", algo);
    info!("🎯 Difficulty tested: {:.10}", args.benchmark_difficulty);
    info!("⏱️ Duration: {:.2}s", result.duration.as_secs_f64());
    info!("⚡ Average hashrate: {}", result.format_hashrate());
    info!("🔥 Peak hashrate: {:.2} MH/s", result.peak_hashrate / 1_000_000.0);
    info!("📈 Total hashes: {}", result.total_hashes);
    info!("💎 Shares found: {}", result.shares_found);
    info!("📊 Shares/MH: {:.2}", result.shares_found as f64 / (result.total_hashes as f64 / 1_000_000.0));
    info!("🧵 Threads used: {}", result.thread_count);

    Ok(())
}

//
// CPU-ONLY MINING
//
#[cfg(feature = "cpu")]
async fn handle_cpu_mining(args: &Args, algo: Algorithm) -> Result<()> {
    // Only SHA3x mining supported now
    if algo != Algorithm::Sha3x {
        eprintln!("❌ Only SHA3x algorithm is supported in this version");
        eprintln!("💡 Use --algo sha3x for mining");
        std::process::exit(1);
    }

    info!("🚀 Starting SHA3x Miner - CPU-ONLY Mode");
    info!("📍 Pool: {}", args.pool.as_ref().unwrap());
    info!("💳 Wallet: {}", args.wallet.as_ref().unwrap());
    info!("👷 Worker: {}", args.worker);
    info!("🧵 CPU Threads: {}", if args.threads == 0 { "auto".to_string() } else { args.threads.to_string() });
    info!("💻 Mode: CPU-only mining (compile with --features gpu for 300+ MH/s boost!)");

    // Create and run your existing CPU miner
    let miner = CpuMiner::new(
        args.wallet.as_ref().unwrap().clone(),
        args.pool.as_ref().unwrap().clone(),
        args.worker.clone(),
        args.threads,
        algo,
    ).into_arc();

    // Start web server in background if --web flag is enabled
    if args.web {
        let miner_clone = miner.clone();
        tokio::spawn(async move {
            let stats = miner_clone.get_stats();
            info!("🌐 Starting web dashboard server...");
            web_server::start_web_server(stats).await;
        });

        info!("📊 Real-time dashboard available at: http://localhost:8080");
        info!("📈 Live charts accessible via the 'Live Charts' tab");
        info!("🔗 WebSocket endpoint: ws://localhost:8080/ws");
    } else {
        info!("💡 Add --web flag to enable real-time web dashboard");
    }

    // Start CPU mining
    info!("🚀 Starting CPU mining");
    miner.run().await?;

    Ok(())
}

//
// GPU-ONLY MINING
//
#[cfg(feature = "gpu")]
async fn handle_gpu_mining(args: &Args, algo: Algorithm) -> Result<()> {
    // Only SHA3x mining supported now
    if algo != Algorithm::Sha3x {
        eprintln!("❌ Only SHA3x algorithm is supported in this version");
        eprintln!("💡 Use --algo sha3x for mining");
        std::process::exit(1);
    }

    info!("🚀 Starting SHA3x Miner - GPU-ONLY Mode");
    info!("📍 Pool: {}", args.pool.as_ref().unwrap());
    info!("💳 Wallet: {}", args.wallet.as_ref().unwrap());
    info!("👷 Worker: {}", args.worker);
    info!("🎮 Mode: GPU-only mining (363+ MH/s beast mode!)");

    // Create GPU miner
    use sha3x_miner::miner::gpu::{GpuManager, GpuMiner};
    
    let gpu_manager = GpuManager::new();
    let gpu_miner = match GpuMiner::new(
        args.wallet.as_ref().unwrap().clone(),
        args.pool.as_ref().unwrap().clone(),
        args.worker.clone(),
        algo,
        gpu_manager,
    ) {
        Ok(miner) => miner.into_arc(),
        Err(e) => {
            eprintln!("❌ Failed to create GPU miner: {}", e);
            eprintln!("💡 Make sure you have OpenCL drivers installed");
            std::process::exit(1);
        }
    };

    // Start web server in background if --web flag is enabled
    if args.web {
        let miner_clone = gpu_miner.clone();
        tokio::spawn(async move {
            let stats = miner_clone.get_stats();
            info!("🌐 Starting GPU web dashboard server...");
            web_server::start_web_server(stats).await;
        });

        info!("📊 Real-time GPU dashboard available at: http://localhost:8080");
        info!("📈 Live GPU charts accessible via the 'Live Charts' tab");
        info!("🔗 WebSocket endpoint: ws://localhost:8080/ws");
    } else {
        info!("💡 Add --web flag to enable real-time web dashboard");
    }

    // Start GPU mining - 363+ MH/s beast mode!
    info!("🚀 Starting GPU mining - unleashing the beast!");
    gpu_miner.run().await?;

    Ok(())
}

//
// HYBRID MINING (CPU + GPU)
//
#[cfg(feature = "hybrid")]
async fn handle_hybrid_mining(args: &Args, algo: Algorithm) -> Result<()> {
    // Only SHA3x mining supported now
    if algo != Algorithm::Sha3x {
        eprintln!("❌ Only SHA3x algorithm is supported in this version");
        eprintln!("💡 Use --algo sha3x for mining");
        std::process::exit(1);
    }

    info!("🚀 Starting SHA3x Miner - HYBRID Mode");
    info!("📍 Pool: {}", args.pool.as_ref().unwrap());
    info!("💳 Wallet: {}", args.wallet.as_ref().unwrap());
    info!("👷 Worker: {}", args.worker);
    info!("🧵 CPU Threads: {}", if args.threads == 0 { "auto".to_string() } else { args.threads.to_string() });
    info!("🎮 Mode: Hybrid CPU+GPU mining (365+ MH/s total!)");

    // TODO: Implement hybrid mining
    info!("🔧 Hybrid mining implementation coming later...");
    info!("💡 This will combine CPU + GPU for maximum hashrate!");
    
    // Prevent exit immediately
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    Ok(())
}