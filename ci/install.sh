set -ex
export PATH="$PATH:$HOME/.cargo/bin"

gethost() {
    case "$TRAVIS_OS_NAME" in
        linux)
            echo x86_64-unknown-linux-gnu
            ;;
        osx)
            echo x86_64-apple-darwin
            ;;
    esac
}

install_rustup() {
    curl https://sh.rustup.rs -sSf \
      | sh -s -- -y --default-toolchain="$TRAVIS_RUST_VERSION"
    rustc -V
    cargo -V
}

install_targets() {
    local host=gethost
    if [ host != "$TARGET" ]; then
        rustup target add $TARGET
    fi
}

main() {
    install_rustup
    install_targets
}

main
