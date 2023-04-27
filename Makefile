test:
	cargo test


check-fmt:
	rustup component add rustfmt
	cargo fmt -- --check

check-clippy:
	rustup component add clippy
	cargo clippy --all-features --tests -- -D warnings

fuzz-parser:
	cargo +nightly fuzz run fuzz_target_1
