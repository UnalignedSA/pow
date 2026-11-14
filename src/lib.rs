//! Proof of work for signup and login forms, shared by the server (verify) and the
//! browser (solve, this crate compiled to WebAssembly).
//!
//! Same scheme as Tor's onion-service DoS defense: Equi-X, an asymmetric
//! puzzle. Solving takes memory and random per-challenge programs, so GPUs and
//! ASICs gain little over a browser; verifying takes microseconds, so fake
//! solutions can't be used to load the server.
//!
//! The browser tries nonces until an Equi-X solution of
//! `PREFIX ‖ seed ‖ nonce ‖ effort` also passes the effort check below:
//! on average `effort` solutions are needed (~2 per nonce).

use sha2::{Digest, Sha256};

pub const SEED_LEN: usize = 32;
pub const NONCE_LEN: usize = 16;
pub const SOLUTION_LEN: usize = equix::Solution::NUM_BYTES;

/// Domain separation: solutions made for another protocol never verify here.
/// Change it to give your project its own domain (then rebuild the solver).
const PREFIX: &[u8] = b"example/pow/v1";
const CHALLENGE_LEN: usize = PREFIX.len() + SEED_LEN + NONCE_LEN + 4;

fn challenge(seed: &[u8; SEED_LEN], nonce: &[u8; NONCE_LEN], effort: u32) -> [u8; CHALLENGE_LEN] {
    let mut out = [0; CHALLENGE_LEN];
    let (prefix, rest) = out.split_at_mut(PREFIX.len());
    let (s, rest) = rest.split_at_mut(SEED_LEN);
    let (n, e) = rest.split_at_mut(NONCE_LEN);
    prefix.copy_from_slice(PREFIX);
    s.copy_from_slice(seed);
    n.copy_from_slice(nonce);
    e.copy_from_slice(&effort.to_le_bytes());
    out
}

/// Passes with probability 1/effort: `hash × effort` must not overflow 32 bits.
fn meets_effort(challenge: &[u8], solution: &[u8; SOLUTION_LEN], effort: u32) -> bool {
    let hash = Sha256::new().chain_update(challenge).chain_update(solution).finalize();
    let r = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]);
    u64::from(r) * u64::from(effort) <= u64::from(u32::MAX)
}

pub fn verify(seed: &[u8; SEED_LEN], effort: u32, nonce: &[u8; NONCE_LEN], solution: &[u8; SOLUTION_LEN]) -> bool {
    let challenge = challenge(seed, nonce, effort);
    // The hash first: it rejects all but 1/effort of random input before Equi-X runs.
    meets_effort(&challenge, solution, effort) && equix::verify_bytes(&challenge, solution).is_ok()
}

/// Tries nonces from `nonce` upward; returns the winning nonce and solution.
pub fn solve(seed: &[u8; SEED_LEN], effort: u32, mut nonce: [u8; NONCE_LEN]) -> ([u8; NONCE_LEN], [u8; SOLUTION_LEN]) {
    let mut memory = equix::SolverMemory::new();
    loop {
        let challenge = challenge(seed, &nonce, effort);
        // A small fraction of challenges has no valid program: skip them.
        if let Ok(equix) = equix::EquiX::new(&challenge) {
            for solution in equix.solve_with_memory(&mut memory) {
                let bytes = solution.to_bytes();
                if meets_effort(&challenge, &bytes, effort) {
                    return (nonce, bytes);
                }
            }
        }
        nonce = u128::from_le_bytes(nonce).wrapping_add(1).to_le_bytes();
    }
}

/// Browser entry points. JS writes `seed ‖ start nonce` into `buffer()`,
/// calls `solve(effort)`, then reads `nonce ‖ solution` back from it.
#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;

    const LEN: usize = SEED_LEN + NONCE_LEN + SOLUTION_LEN;
    static mut BUFFER: [u8; LEN] = [0; LEN];

    #[unsafe(no_mangle)]
    pub extern "C" fn buffer() -> *mut u8 {
        (&raw mut BUFFER).cast()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn solve(effort: u32) {
        // Single-threaded: one call at a time, from one worker.
        let buf = unsafe { &mut *(&raw mut BUFFER) };
        let seed: [u8; SEED_LEN] = buf[..SEED_LEN].try_into().unwrap();
        let start: [u8; NONCE_LEN] = buf[SEED_LEN..SEED_LEN + NONCE_LEN].try_into().unwrap();
        let (nonce, solution) = super::solve(&seed, effort, start);
        buf[..NONCE_LEN].copy_from_slice(&nonce);
        buf[NONCE_LEN..NONCE_LEN + SOLUTION_LEN].copy_from_slice(&solution);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_and_verifies() {
        let seed = [3; SEED_LEN];
        let (nonce, solution) = solve(&seed, 8, [0; NONCE_LEN]);
        assert!(verify(&seed, 8, &nonce, &solution));
        assert!(!verify(&[4; SEED_LEN], 8, &nonce, &solution), "other seed");
        assert!(!verify(&seed, 9, &nonce, &solution), "other effort");
    }

    #[test]
    fn rejects_garbage() {
        assert!(!verify(&[0; SEED_LEN], 1, &[0; NONCE_LEN], &[0; SOLUTION_LEN]));
    }
}
