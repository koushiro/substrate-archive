ulimit -n 100000
export RUST_BACKTRACE=full
nohup ./target/release/patract-archive -c jupiter.toml --spec=jupiter > output.log 2>&1 &
