// SHA3x (Tari) OpenCL Kernel - Exact match to CPU implementation
// Input format: nonce(8 bytes LE) + header(32 bytes) + marker(1 byte) = 41 bytes
// Algorithm: Triple SHA3-256 (exactly like CPU sha3x_hash_with_nonce)

// Keccak-f[1600] implementation for SHA3-256
constant ulong keccakf_rndc[24] = {
    0x0000000000000001UL, 0x0000000000008082UL, 0x800000000000808aUL,
    0x8000000080008000UL, 0x000000000000808bUL, 0x0000000080000001UL,
    0x8000000080008081UL, 0x8000000000008009UL, 0x000000000000008aUL,
    0x0000000000000088UL, 0x0000000080008009UL, 0x000000008000000aUL,
    0x000000008000808bUL, 0x800000000000008bUL, 0x8000000000008089UL,
    0x8000000000008003UL, 0x8000000000008002UL, 0x8000000000000080UL,
    0x000000000000800aUL, 0x800000008000000aUL, 0x8000000080008081UL,
    0x8000000000008080UL, 0x0000000080000001UL, 0x8000000080008008UL
};

constant uint keccakf_rotc[24] = {
    1,  3,  6,  10, 15, 21, 28, 36, 45, 55, 2,  14,
    27, 41, 56, 8,  25, 43, 62, 18, 39, 61, 20, 44
};

constant uint keccakf_piln[24] = {
    10, 7,  11, 17, 18, 3, 5,  16, 8,  21, 24, 4,
    15, 23, 19, 13, 12, 2, 20, 14, 22, 9,  6,  1
};

void keccakf(ulong st[25]) {
    ulong t, bc[5];
    
    for (int r = 0; r < 24; r++) {
        // Theta
        for (int i = 0; i < 5; i++) {
            bc[i] = st[i] ^ st[i + 5] ^ st[i + 10] ^ st[i + 15] ^ st[i + 20];
        }
        
        for (int i = 0; i < 5; i++) {
            t = bc[(i + 4) % 5] ^ rotate(bc[(i + 1) % 5], (ulong)1);
            for (int j = 0; j < 25; j += 5) {
                st[j + i] ^= t;
            }
        }
        
        // Rho Pi
        t = st[1];
        for (int i = 0; i < 24; i++) {
            int j = keccakf_piln[i];
            bc[0] = st[j];
            st[j] = rotate(t, (ulong)keccakf_rotc[i]);
            t = bc[0];
        }
        
        // Chi
        for (int j = 0; j < 25; j += 5) {
            for (int i = 0; i < 5; i++) {
                bc[i] = st[j + i];
            }
            for (int i = 0; i < 5; i++) {
                st[j + i] ^= (~bc[(i + 1) % 5]) & bc[(i + 2) % 5];
            }
        }
        
        // Iota
        st[0] ^= keccakf_rndc[r];
    }
}

void sha3_256(uchar* input, uint input_len, uchar output[32]) {
    ulong st[25];
    for (int i = 0; i < 25; i++) st[i] = 0;
    
    // Absorb phase - rate = 136 bytes for SHA3-256
    uint rate = 136;
    uint offset = 0;
    
    while (input_len >= rate) {
        for (uint i = 0; i < rate / 8; i++) {
            ulong word = 0;
            for (int j = 0; j < 8; j++) {
                word |= ((ulong)input[offset + i * 8 + j]) << (j * 8);
            }
            st[i] ^= word;
        }
        keccakf(st);
        input_len -= rate;
        offset += rate;
    }
    
    // Final block with padding
    uchar final_block[136];
    for (uint i = 0; i < 136; i++) final_block[i] = 0;
    
    // Copy remaining input
    for (uint i = 0; i < input_len; i++) {
        final_block[i] = input[offset + i];
    }
    
    // SHA3 padding: 0x06 then zeros, then 0x80 at end
    final_block[input_len] = 0x06;
    final_block[135] = 0x80;
    
    // Absorb final block
    for (uint i = 0; i < 17; i++) { // 136/8 = 17
        ulong word = 0;
        for (int j = 0; j < 8; j++) {
            word |= ((ulong)final_block[i * 8 + j]) << (j * 8);
        }
        st[i] ^= word;
    }
    keccakf(st);
    
    // Squeeze 32 bytes
    for (int i = 0; i < 4; i++) {
        for (int j = 0; j < 8; j++) {
            output[i * 8 + j] = (st[i] >> (j * 8)) & 0xFF;
        }
    }
}

kernel void sha3(global ulong *header_buffer, ulong nonce_start, ulong target_value,
                 uint num_rounds, global ulong *output) {

    // Initialize output (first thread only)
    if (get_global_id(0) == 0) {
        output[0] = 0;           // Found nonce (0 = not found)
        output[1] = ULONG_MAX;   // Best hash value found
    }
    barrier(CLK_GLOBAL_MEM_FENCE);
    
    ulong thread_nonce = nonce_start + get_global_id(0);
    
    for (uint round = 0; round < num_rounds; round++) {
        ulong current_nonce = thread_nonce + round * get_global_size(0);
        
        // Build SHA3x input exactly like CPU: nonce(8) + header(32) + marker(1) = 41 bytes
        uchar input[41];
        
        // Nonce as little-endian bytes (matches CPU nonce.to_le_bytes())
        for (int i = 0; i < 8; i++) {
            input[i] = (current_nonce >> (i * 8)) & 0xFF;
        }
        
        // Header bytes from header_buffer (4 u64s = 32 bytes)
        for (int i = 0; i < 4; i++) {
            ulong header_word = header_buffer[i];
            for (int j = 0; j < 8; j++) {
                input[8 + i * 8 + j] = (header_word >> (j * 8)) & 0xFF;
            }
        }
        
        // Marker byte (matches CPU .push(1u8))
        input[40] = 1;
        
        // Triple SHA3-256 (exactly like CPU)
        uchar hash1[32], hash2[32], hash3[32];
        
        sha3_256(input, 41, hash1);     // First SHA3-256
        sha3_256(hash1, 32, hash2);     // Second SHA3-256  
        sha3_256(hash2, 32, hash3);     // Third SHA3-256
        
        // Convert first 8 bytes of final hash to u64 for difficulty (big-endian like CPU)
        ulong hash_value = 0;
        for (int i = 0; i < 8; i++) {
            hash_value = (hash_value << 8) | hash3[i];
        }
        
        // Check if hash meets target (lower hash = higher difficulty)
        if (hash_value <= target_value) {
            // Found valid share!
            atomic_min(&output[1], hash_value);
            if (output[0] == 0) {
                output[0] = current_nonce;
            }
        } else {
            // Track best hash for statistics
            atomic_min(&output[1], hash_value);
        }
    }
}