[![CI](https://github.com/rnambaale/monexo-ecash/actions/workflows/rust.yml/badge.svg)](https://github.com/rnambaale/monexo-ecash/actions/workflows/rust.yml)
[![Codecov](https://codecov.io/github/rnambaale/monexo-ecash/coverage.svg?branch=master)](https://codecov.io/gh/rnambaale/monexo-ecash)
[![Dependency status](https://deps.rs/repo/github/rnambaale/monexo-ecash/status.svg)](https://deps.rs/repo/github/rnambaale/monexo-ecash)

---
sqlx database drop --database-url postgres://postgres:password@localhost:5432/moksha-mint
sqlx database create --database-url postgres://postgres:password@localhost:5432/moksha-mint
sqlx migrate run --database-url postgres://postgres:password@localhost:5432/moksha-mint
cargo sqlx prepare --database-url postgres://postgres:password@localhost:5432/moksha-mint

## Running the mint

```
RUST_LOG=debug RUST_BACKTRACE=1 MINT_APP_ENV=dev cargo run --bin monexo-mint
```

## Creating shared USDC address (Solana)

### 1. Install Solana CLI and SPL CLI (Optional)
```
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

cargo install spl-token-cli
```

### 2. Create or Use a Wallet
```
solana-keygen new --outfile ~/.config/solana/shared-keypair.json
```
Get the wallet public key:
```
solana address
```

### 3. Create the Shared USDC Token Account

USDC on Solana has a specific mint address. The mainnet address for USDC is:
`EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`.

Create the shared token account:
```
spl-token create-account EPjFWdd5Au1hnyYw7T7jn1xnJmgwFb4HpD1w8wLZco3d
```
The command will return a token account address (the shared deposit address).

### 4. Verify the Token Account
You can list your token account addresses and their balances:
```
spl-token accounts
```

## Deposit flow (1)
- user generates a deposit request at the mint
- the mint creates a quote that will be used as a memo, when the user is depositing USDC into the mint's shared address
- We set up a Helius webhook before hand to notify the mint every time a new transaction happens in the mint's USDC address.
```
curl -X POST "https://api.helius.xyz/v0/webhooks?api-key=<your-api-key>" \
-H "Content-Type: application/json" \
-d '{
  "webhookURL": "https://your-backend-url.com/mint-callback",
  "accountAddresses": ["<shared-address>"],
  "transactionTypes": ["ANY"]
}'
```

The webhook typically sends a payload like this for every txn

```
{
  "transaction": {
    "signature": "3xsB...nL",
    "instructions": [
      {
        "programId": "Memo111111111111111111111111111111111111111",
        "parsed": "unique-user-memo-12345"
      }
    ]
  }
}
```

- The mint generates signatures for the amount that ha been deposited, and updates the quote status as "Issued"

## Deposit flow (2)

It turns oout there is a solana-pay method `findReference(reference)` that can be used to directly look up a transaction by the reference given. This might help us eliminate the webhook flow.

The question is; is it possible to generate a qr code with the mint's usdc address, and the unique reference so that when a user scans it from whatever usdc address they are using, those two are automatically detected.

Secondly, is it mandatory that `findReference` must be used with solana-pay

https://github.com/anza-xyz/solana-pay/blob/master/core/src/findReference.ts
https://www.helius.dev/blog/solana-pay


## Payment request (Solana Pay)

solana:<recipient_wallet_address>?amount=0.5&spl-token=<TOKEN_MINT>&reference=<REFERENCE_ID>&label=Store&message=Thank%20you!

solana:HVasUUKPrmrAuBpDFiu8BxQKzrMYY5DvyuNXamvaG2nM?amount=1500&spl-token=4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU&reference=5t6gQ7Mnr3mmsFYquFGwgEKokq9wrrUgCpwWab93LmLL&label=Store&message=Thank%20you!

## Wallet DB Ops
cd monexo-wallet
sqlx database drop --database-url sqlite://wallet.db
sqlx database create --database-url sqlite://wallet.db
sqlx migrate run --database-url sqlite://wallet.db
cargo sqlx prepare --database-url sqlite://wallet.db
