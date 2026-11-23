.PHONY: fmt fmt-check lint test build run ingest docker-build docker-up docker-down clean

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test --all-targets

build:
	cargo build --release --locked

run:
	cargo run --bin braincode-be

ingest:
	cargo run --release --bin ingest_csv

docker-build:
	docker build -t braincode-be:latest .

docker-up:
	docker compose up -d --build

docker-down:
	docker compose down

clean:
	cargo clean
