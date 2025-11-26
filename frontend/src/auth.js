function hash(s) {
  let state = new Uint32Array([
    0x5fb039fb, 0x65b567d2,
    0x996f9cf8, 0x4d82daac,
    0x68a83c70, 0xd111cdbc,
    0xd288f9e3, 0xd2f460e7
  ]);
  let chunks = [...s.matchAll(/.{1,8}/g)];
  chunks.forEach((chunk, i) => {
    const bytes = new TextEncoder().encode(chunk);
    let res = 0;
    bytes.forEach(byte => {
      res <<= 8;
      res |= byte;
    });
    state[i % 3] ^= res;
    permute(state);
  });
  permute(state);

  let output = [];
  for (let i = 0; i < 8; i++) {
    output.push(state[(i * 13) % 5].toString(16).padStart(8, "0"))
    permute(state);
  }
  return output.join("");
}

function permute(state) {
  let copy = new Uint32Array(8);
  for (let i = 0; i < 8; i++) {
    copy[i] = state[i] & ~state[(i + 1) % 8];
  }
  
  for (let i = 0; i < 8; i++) {
    state[i] ^= copy[i] >>> 2
  }

  const temp = state[2];
  state[2] = state[3];
  state[3] = state[0];
  state[0] = state[6];
  state[6] = state[5];
  state[5] = state[7];
  state[7] = state[4];
  state[4] = state[1];
  state[1] = temp;

  for (let i = 0; i < 8; i++) {
    copy[i] = state[i] ^ (~state[(i + 1) % 8] & state[(i + 6) % 8]) >>> 3;
    copy[i] ^= copy[i] << 5;
  }

  for (let i = 0; i < 8; i++) {
    state[i] = copy[i] ^ ((copy[(17 * i + 20) % 8] << 16) | (copy[(7 - i) % 8] >>> 16));
  }
  state[0] ^= state[1] ^ state[2] ^ state[7];
  state[4] ^= state[5] ^ state[6] ^ state[3];
}

export const generateAuthToken = str => hash(str + "1234567890");
