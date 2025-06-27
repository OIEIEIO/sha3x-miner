# SHA3x GPU Miner 🚀

High-performance GPU miner for SHA3x (Tari) - delivering 385+ MH/s on RTX 4060 Ti!

## 🎮 GPU Mining - TESTED & PROVEN

**2 blocks found on https://backend.sha3x.supportxtm.com/** 🎯  
**74 XTM earned** - This GPU miner earns its keep! 💯

### Quick Start - GPU Mining

```bash
cargo run --release --features gpu --bin sha3x-miner -- \
  --algo sha3x \
  --pool pool.sha3x.supportxtm.com:6118 \
  --wallet 125ohcEDcG8sL4DcdtqZ6YLbSgVYFJWtGeCfHmRocTcyGNYRqMYidnfs1JQPijqQvqV5SLygC5ynxZH3zED5Rr9fPAW \
  --web \
  --worker riddick \
  --gpu-intensity 100 \
  --gpu-batch-size 10000
```

### Building

```bash
# GPU-only build (recommended)
cargo build --release --features gpu

# Hybrid build (WIP - currently runs either GPU OR CPU, not both)
cargo build --release --features hybrid
```

## ⚠️ Important Notes

- **Hybrid Feature**: Currently WIP - it builds but runs either GPU OR CPU mining, not both simultaneously
- **GPU Performance**: RTX 4060 Ti achieves 385+ MH/s at 100% intensity
- **XN Support**: Framework in place, still being worked on
- **Web Dashboard**: Access real-time stats at http://localhost:8080 with `--web` flag

## 🔧 GPU Parameters

- `--gpu-intensity`: 0-100% (default: 100)
- `--gpu-batch-size`: Override automatic batch size
- `--gpu-power-limit`: 50-110% (requires external tools)
- `--gpu-temp-limit`: 60-85°C temperature throttle

## 📊 Performance

| GPU | Hashrate | Settings |
|-----|----------|----------|
| RTX 4060 Ti | 385+ MH/s | 100% intensity, batch 10000 |

## 🏗️ Current Status

This is "Dirty Harry" code - it works, it's fast, and it mines blocks. Like Harry Callahan himself, this miner doesn't play by the rules of 'clean code' - it just gets the job done.

**What works:**
- ✅ GPU mining with OpenCL
- ✅ 385+ MH/s performance
- ✅ Pool connectivity and share submission
- ✅ Web dashboard monitoring
- ✅ Proven block finding capability

**Work in Progress:**
- 🔧 XN parameter support
- 🔧 True hybrid CPU+GPU mining
- 🔧 Code cleanup and optimization

## License

MIT - Free and Open Source Software

---

*"Do you feel lucky? Well, we ARE lucky!"* - 2 blocks and counting! 🎲
