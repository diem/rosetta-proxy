# Copyright (c) The Diem Core Contributors
# SPDX-License-Identifier: Apache-2.0

init:
	rm -rf venv
	python3 -m venv ./venv

	./venv/bin/pip install --upgrade pip wheel setuptools
	./venv/bin/pip install diem
	./venv/bin/python setup_test_env.py

test-server:
	RUST_LOG=diem_rosetta_proxy=debug cargo run -- --network testnet --diem-endpoint http://testnet.diem.com/v1

check-data:
	~/bin/rosetta-cli check:data --configuration-file rosetta-diem.json

check-con:
	~/bin/rosetta-cli check:construction --configuration-file rosetta-diem.json

.PHONY: init test-server check-data check-con
