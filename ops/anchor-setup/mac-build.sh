#!/bin/zsh
cd ~/dev/atrium-anchor/DUCTEI
grep -q atrium-watch Cargo.toml || sed -i '' 's#"qsw-inspect"\]#"qsw-inspect","atrium-watch"]#' Cargo.toml
grep members Cargo.toml
~/.cargo/bin/cargo build --release -p atrium-watch 2>&1 | grep -E '^error|-->|Finished' | head -30
~/.cargo/bin/cargo test -p atrium-watch 2>&1 | grep -E 'test result: ok. [1-9]|FAILED|^error'