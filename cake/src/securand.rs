//! Secure random number generation utilities.

use core::hash::{BuildHasher, Hasher};

use arrayvec::ArrayVec;
use log::{error, info};
use rand_chacha::{ChaCha12Core, ChaCha20Core, ChaCha20Rng};
use rand_core::{Rng, SeedableRng, block::Generator};
use rs_sha512_256::{HasherContext, Sha512_256Hasher, Sha512_256State};
use spin::Mutex;

use crate::OnceMutex;

mod arch {
    #[cfg(target_arch = "x86_64")]
    pub use super::x86_64::*;

    #[cfg(target_arch = "aarch64")]
    pub use super::aarch64::*;
}

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use raw_cpuid::CpuId;
    pub fn hardware_rng_available() -> bool {
        // Check if the CPU supports RDRAND instruction

        let cpuid = CpuId::new();
        if let Some(feature_info) = cpuid.get_feature_info() {
            return feature_info.has_rdrand();
        }

        false
    }

    pub fn hardware_seed_available() -> bool {
        // Check if the CPU supports RDSEED instruction

        let cpuid = CpuId::new();
        if let Some(extended_features) = cpuid.get_extended_feature_info() {
            return extended_features.has_rdseed();
        }

        false
    }

    pub fn rng() -> Option<u64> {
        if hardware_rng_available() {
            {
                let mut value: u64 = 0;
                let success = unsafe { core::arch::x86_64::_rdrand64_step(&mut value) };
                if success == 1 {
                    return Some(value);
                }
            }
        }
        None
    }

    pub fn seed() -> Option<u64> {
        if hardware_seed_available() {
            {
                let mut value: u64 = 0;
                let success = unsafe { core::arch::x86_64::_rdseed64_step(&mut value) };
                if success == 1 {
                    return Some(value);
                }
            }
        }
        None
    }
}

#[cfg(target_arch = "aarch64")]
mod aarch64 {
    // Placeholder for ARM64 architecture-specific implementations.
    // Implementations for hardware RNG and seed retrieval would go here.
    pub fn hardware_rng_available() -> bool {
        // Implement ARM64-specific check for hardware RNG support.
        false
    }

    pub fn hardware_seed_available() -> bool {
        // Implement ARM64-specific check for hardware seed support.
        false
    }

    pub fn rng() -> Option<u64> {
        // Implement ARM64-specific hardware RNG retrieval.
        None
    }

    pub fn seed() -> Option<u64> {
        // Implement ARM64-specific hardware seed retrieval.
        None
    }
}

static RNG: OnceMutex<ChaCha20Rng> = OnceMutex::uninitialized();

static ENTROPY_SOURCES: Mutex<ArrayVec<fn() -> Option<u64>, 32>> =
    Mutex::new(ArrayVec::new_const());

/// Adds a new entropy source to the list of available sources for the secure random number generator.
pub fn add_entropy_source(source: fn() -> Option<u64>) {
    let mut sources = ENTROPY_SOURCES.lock();
    if sources.len() < sources.capacity() {
        sources.push(source);
    } else {
        error!(target:"rng", "Maximum number of entropy sources reached, cannot add more.");
    }
}

/// Initializes the secure random number generator with entropy from available sources.
pub fn init_rng() {
    let mut entropy_sources = ENTROPY_SOURCES.lock();

    // Add hardware RNG and seed functions as entropy sources
    entropy_sources.push(arch::rng);
    entropy_sources.push(arch::seed);

    if entropy_sources.is_empty() {
        error!(
            target:"rng",
            "No entropy sources available, secure random number generation may be weak."
        );
    }

    let mut seed: [u8; 32] = [0; 32];
    let state = Sha512_256State::default();

    for source in entropy_sources.iter() {
        if let Some(value) = source() {
            let mut hasher = state.build_hasher();
            hasher.write_u64(value);
            hasher.write(bytemuck::cast_slice(&seed));
            let hash_bytes = HasherContext::finish(&mut hasher);
            seed.copy_from_slice(&hash_bytes[..]);
        }
    }

    let rng = ChaCha20Rng::from_seed(bytemuck::cast(seed));
    RNG.call_init(|| rng);

    info!(target:"rng",
        "Initialized secure random number generator with entropy from {} sources.",
        entropy_sources.len(),
    );

    info!(target:"rng", "data sample: {:x?} (if there's any patterns then there's a problem!)", RNG.get().next_u64());
}

/// Generates a random u64 using the secure random number generator.
pub fn rng64() -> u64 {
    let mut rng = RNG.get();
    rng.next_u64()
}

/// Generates a random u32 using the secure random number generator.
pub fn rng32() -> u32 {
    let mut rng = RNG.get();
    rng.next_u32()
}

/// Fills the provided buffer with random bytes using the secure random number generator.
pub fn rng(buf: &mut [u8]) {
    let mut rng = RNG.get();
    rng.fill_bytes(buf);
}
