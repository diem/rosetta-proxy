> **Note to readers:** On December 1, 2020, the Libra Association was renamed to Diem Association. The project repos are in the process of being migrated. All projects will remain available for use here until the migration to a new GitHub Organization is complete.

# rosetta-proxy

Rosetta API implementation for the Libra Payment Network.

This should be run alongside a Libra fullnode and will take incoming Rosetta
requests and make outbound Libra JSON-RPC requests.

# Usage

`cargo run -- --network mainnet --libra-endpoint http://fullnode-address/port`

To enable debugging information, use `RUST_LOG`:

`RUST_LOG=libra_rosetta_proxy=debug cargo run -- --network mainnet --libra-endpoint http://fullnode-address/port`

# Testing

You can test this implementation locally with `libra-node --test`:

1. In `./libra` run `cargo run -p libra-node -- --test`
2. Run `get-accounts.cli` through the Libra CLI to create accounts and mint
   coins. For example: `cat ./path/to/get-accounts.cli | cargo run -p cli -- -u http://localhost:8080 -m /path/to/mint.key --waypoint 123 --chain-id TESTING`
4. Launch `rosetta-proxy`
5. Run the Rosetta Data API validator: `rosetta-cli check:data --configuration-file rosetta-libra.json`
5. Run the Rosetta Construction API validator: `rosetta-cli check:construction --configuration-file rosetta-libra.json`
