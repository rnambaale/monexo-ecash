--
## Drop database
```bash
sqlx database drop --database-url postgres://postgres:password@localhost:5432/monexo-mint
```
## Create database
```bash
sqlx database create --database-url postgres://postgres:password@localhost:5432/monexo-mint
```

## Run migrations
```bash
sqlx migrate run --database-url postgres://postgres:password@localhost:5432/monexo-mint
```

## Build `.sqlx` cache
```bash
cargo sqlx prepare --database-url postgres://postgres:password@localhost:5432/monexo-mint
```
