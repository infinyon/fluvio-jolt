test:
	cargo test


check-fmt:
	rustup component add rustfmt
	cargo fmt -- --check

check-clippy:
	rustup component add clippy
	cargo clippy --all-features --tests -- -D warnings