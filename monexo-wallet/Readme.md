## Wallet DB Ops
```sh
cd monexo-wallet
sqlx database drop --database-url sqlite://wallet.db
sqlx database create --database-url sqlite://wallet.db
sqlx migrate run --database-url sqlite://wallet.db
cargo sqlx prepare --database-url sqlite://wallet.db
```
