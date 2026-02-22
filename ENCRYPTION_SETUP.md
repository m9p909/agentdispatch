# API Key Encryption Setup

## Overview
API keys stored in the `llm_providers` table are now encrypted at rest using ChaCha20-Poly1305 AEAD encryption. Keys are encrypted before writing to the database and decrypted when reading.

## Prerequisites
You must set the `ENCRYPTION_KEY` environment variable before running the application.

## Generating an Encryption Key

Generate a 32-byte (256-bit) random key in hex format:

```bash
# Using openssl
openssl rand -hex 32

# Using Python
python3 -c "import secrets; print(secrets.token_hex(32))"

# Using Rust
cargo run --bin=generate_key  # (if you create this binary)
```

Example output: `a7f4e8b2c1d9f5e3a6b2c8d1e9f5a3b7c2d8e1f4a5b6c7d8e9f0a1b2c3d4e5`

## Environment Setup

Add to your `.env` file or environment:

```bash
ENCRYPTION_KEY=a7f4e8b2c1d9f5e3a6b2c8d1e9f5a3b7c2d8e1f4a5b6c7d8e9f0a1b2c3d4e5
```

The application will fail to start if `ENCRYPTION_KEY` is not set.

## Data Migration

### For New Installations
No action needed. Keys are encrypted on creation.

### For Existing Data
When you deploy this change:

1. All new API keys will be automatically encrypted
2. Existing unencrypted keys remain readable but unencrypted
3. When an existing key is updated through the API, it gets encrypted
4. To encrypt all existing keys, write a migration script that:
   - Reads all providers
   - Updates each one with its current api_key (triggers encryption in the application)

## Architecture

### Encryption Flow
```
POST /api/providers
  └─> create_provider(api_key)
      └─> cipher.encrypt(api_key) -> "hex_encoded_ciphertext_with_nonce"
          └─> INSERT INTO llm_providers (api_key) VALUES ("hex_encoded...")
```

### Decryption Flow
```
GET /api/providers/{id}
  └─> get_provider_by_id(id)
      └─> SELECT * FROM llm_providers WHERE id = $1
          └─> cipher.decrypt("hex_encoded...") -> "original_api_key"
              └─> Return LlmProvider { api_key: "original_api_key", ... }
```

### Key Details
- **Algorithm**: ChaCha20-Poly1305 (modern, fast AEAD encryption)
- **Key Size**: 256 bits (32 bytes)
- **Nonce**: Random 96-bit (12 bytes) nonce per encryption
- **Encoding**: Nonce + Ciphertext stored as hex in the database
- **Lookup**: Keys cannot be searched in encrypted form. Use provider ID or name instead.

## Security Considerations

✅ **Protected**
- API keys are encrypted at rest in the database
- Each encryption uses a random nonce (no pattern leakage)
- Database backups contain only encrypted data

⚠️ **Not Protected**
- Keys are in plaintext in application memory after decryption
- Keys are visible in HTTP headers when calling LLM APIs
- Keys are visible in application logs (if logged)
- Database must be protected - anyone with database access + ENCRYPTION_KEY can read keys

🔒 **Best Practices**
- Rotate ENCRYPTION_KEY periodically
- Use strong, unique ENCRYPTION_KEY per environment
- Never commit ENCRYPTION_KEY to version control
- Use environment-specific .env files
- Audit database access logs
- Monitor for unusual key access patterns

## Testing

The encryption module includes a basic roundtrip test:

```bash
ENCRYPTION_KEY=0000000000000000000000000000000000000000000000000000000000000000 \
  cargo test crypto::tests::test_encrypt_decrypt_roundtrip
```

## Troubleshooting

### "ENCRYPTION_KEY not set in environment"
Set the environment variable before running:
```bash
export ENCRYPTION_KEY=$(openssl rand -hex 32)
```

### "ENCRYPTION_KEY must be 32 bytes"
The key must be exactly 64 hex characters (32 bytes). Generate a new one:
```bash
openssl rand -hex 32
```

### "Decryption failed"
- Ensure ENCRYPTION_KEY hasn't changed (can't decrypt with different key)
- Data in database may be corrupted
- Check application logs for the actual error

## Files Modified

- `src/crypto.rs` - Encryption/decryption logic
- `src/lib.rs` - Added crypto module
- `src/main.rs` - Added crypto module
- `src/llm_providers/db.rs` - Integrated encryption on create/read/update
- `Cargo.toml` - Added `chacha20poly1305` dependency
