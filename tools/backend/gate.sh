# Gate for backend promotions. Pinned per job. Run by the installed taik-seed in the checkout.
set -eu
cargo test --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
cargo build --release --offline --locked
cp target/release/taik-seed "$TAIK_CANDIDATE_OUT"
echo "PASS: Rust tests, Clippy with warnings denied, release build"
