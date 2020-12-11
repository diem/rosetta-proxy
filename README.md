# rosetta-proxy

Rosetta API implementation for the Diem Payment Network.

This should be run alongside a Diem fullnode and will take incoming Rosetta
requests and make outbound Diem JSON-RPC requests.

# Usage

`cargo run -- --network mainnet --diem-endpoint http://fullnode-address/port`

To enable debugging information, use `RUST_LOG`:

`RUST_LOG=diem_rosetta_proxy=debug cargo run -- --network mainnet --diem-endpoint http://fullnode-address/port`

# Testing

2. Run `make init`
4. Launch test server: `make test-server`
5. Run the Rosetta Data API validator: `make check-data`
5. Run the Rosetta Construction API validator: `make check-con`
