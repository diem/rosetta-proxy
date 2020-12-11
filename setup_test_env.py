# Copyright (c) The Diem Core Contributors
# SPDX-License-Identifier: Apache-2.0

from diem import testnet, LocalAccount
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
import json


def accounts():
    with open("rosetta-diem.json", "r") as f:
        config=json.load(f)
        return map(lambda c: create_account(c["privkey"]), config["construction"]["prefunded_accounts"])


def create_account(key):
    return LocalAccount(Ed25519PrivateKey.from_private_bytes(bytes.fromhex(key)))


faucet = testnet.Faucet(testnet.create_client())
for account in accounts():
    faucet.mint(account.auth_key.hex(), 5_000_000_000, testnet.TEST_CURRENCY_CODE)
