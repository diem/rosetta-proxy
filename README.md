> **Note to readers:** On December 1, 2020, the Diem Association was renamed to Diem Association. The project repos are in the process of being migrated. All projects will remain available for use here until the migration to a new GitHub Organization is complete.

# rosetta-proxy

Rosetta API implementation for the Diem Payment Network.

This should be run alongside a Diem fullnode and will take incoming Rosetta
requests and make outbound Diem JSON-RPC requests.

# Usage

`cargo run -- --network mainnet --diem-endpoint http://fullnode-address/port`

To enable debugging information, use `RUST_LOG`:

`RUST_LOG=diem_rosetta_proxy=debug cargo run -- --network mainnet --diem-endpoint http://fullnode-address/port`

# Testing

You can test this implementation locally with `diem-node --test`:

1. In `./diem` run `cargo run -p diem-node -- --test`
2. Run `get-accounts.cli` through the Diem CLI to create accounts and mint
   coins. For example: `cat ./path/to/get-accounts.cli | cargo run -p cli -- -u http://localhost:8080 -m /path/to/mint.key --waypoint 123 --chain-id TESTING`
4. Launch `rosetta-proxy`:
   `RUST_LOG=diem_rosetta_proxy=debug cargo run -- --network testing --diem-endpoint http://localhost:8080/v1`
5. Run the Rosetta Data API validator: `rosetta-cli check:data --configuration-file rosetta-diem.json`
5. Run the Rosetta Construction API validator: `rosetta-cli check:construction --configuration-file rosetta-diem.json`
