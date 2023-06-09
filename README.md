Meteora DAO

The repository consists of 7 programs:

1. Govern program
2. Merkle Distributor
3. Smart wallet
4. Voter
5. Gauge
6. Quarry
7. Minter

To run all the test, please run the following commands:

```
anchor test
```

You may run tests belong to specific program by:

```
Govern:
anchor test --run ./tests/govern -- --features test-bpf

Merkle distributor:
anchor test --run ./tests/merkle-distributor -- --features test-bpf

Smart wallet:
anchor test --skip-build --run ./tests/smartwallet -- --features test-bpf

Voter:
anchor test --run ./tests/voter -- --features test-bpf

Gauge:
anchor test --skip-build --run ./tests/gauge

Quarry:
anchor test --skip-build --run ./tests/quarry

Minter:
anchor test --skip-build --run ./tests/minter

```
