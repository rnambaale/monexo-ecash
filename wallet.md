## Install Solana CLI and SPL CLI (Optional)
```
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

cargo install spl-token-cli
```

## Wallet creation

```
solana-keygen new --outfile my-wallet.json
```
This will generate a key pair for the wallet. Remember to take note of the seed phrase, and secure it properly.

## USDC ATA Address creation

Load the wallet key pair
```
export SOLANA_WALLET="path/to/my-wallet.json"
solana address
```

This will show your wallet address.

---

Ensure that the payer has enough SOL.
```
solana airdrop 2 --keypair my-wallet.json
```

Getting the config
```
solana config get
```

```
// 4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU USDC devnet address
spl-token create-account <USDC_MINT_ADDRESS> --owner "$(solana address)"

spl-token create-account 4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU --owner DHoRFzF1814ymMe9KdViwqXGKiw2JhAp8SiaB5SrrM8L --fee-payer  my-wallet.json

```
This creates a USDC ATA derived from your wallet address for the USDC mint.

Check the balance to confirm the linkage (This gets that USDC balance):

```
spl-token balance <USDC_MINT_ADDRESS> --owner "$(solana address)"

spl-token balance 4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU --owner "DHoRFzF1814ymMe9KdViwqXGKiw2JhAp8SiaB5SrrM8L"
```

### Verify the Token Account
You can list your token account addresses and their balances:
```
spl-token accounts
```


## Wallet DB Ops
```
cd monexo-wallet
sqlx database drop --database-url sqlite://wallet.db
sqlx database create --database-url sqlite://wallet.db
sqlx migrate run --database-url sqlite://wallet.db
cargo sqlx prepare --database-url sqlite://wallet.db
```
