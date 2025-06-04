use crate::HighscoreReq;
use std::fmt::Write;

fn permute(state: &mut [u32]) {
    let mut copy: [u32; 8] = [0; 8];
    for i in 0..8 {
        copy[i] = state[i] & !state[(i + 1) % 8];
    }

    for i in 0..8 {
        state[i] ^= copy[i] >> 2;
    }

    let temp = state[2];
    state[2] = state[3];
    state[3] = state[0];
    state[0] = state[6];
    state[6] = state[5];
    state[5] = state[7];
    state[7] = state[4];
    state[4] = state[1];
    state[1] = temp;

    for i in 0..8 {
        copy[i] = state[i] ^ (!state[(i + 1) % 8] & state[(i + 6) % 8]) >> 3;
        copy[i] ^= copy[i] << 5;
    }

    for i in 0..8 {
        state[i] = copy[i] ^ ((copy[(17 * i + 20) % 8] << 16) | (copy[(7 - i) % 8] >> 16));
    }
    state[0] ^= state[1] ^ state[2] ^ state[7];
    state[4] ^= state[5] ^ state[6] ^ state[3];
}

fn hash(input: &str) -> String {
    let mut state: [u32; 8] = [
        0x5fb0_39fb,
        0x65b5_67d2,
        0x996f_9cf8,
        0x4d82_daac,
        0x68a8_3c70,
        0xd111_cdbc,
        0xd288_f9e3,
        0xd2f4_60e7,
    ];
    let bytes = input.as_bytes();
    let mut i = 0;
    bytes.chunks(8).for_each(|chunk| {
        let mut res: u32 = 0;
        for byte in chunk {
            res <<= 8;
            res |= u32::from(*byte);
        }
        state[i % 3] ^= res;
        i += 1;
        permute(&mut state);
    });
    permute(&mut state);
    let mut output = String::new();
    for i in 0..8 {
        let _ = write!(output, "{:08x}", state[(i * 13) % 5]);
        permute(&mut state);
    }
    output
}

/// Generates an auth token for a request and time
pub fn gen_auth_token(req: &HighscoreReq, tens: u32) -> String {
    hash(&format!(
        "{:o} fffffffff {} esiovtb3w5iothbiouthes0u1234567890{tens}",
        req.score, req.name
    ))
}
