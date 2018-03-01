# This script takes care of testing your crate

set -ex

main() {

    cargo test --target $TARGET --all
    cargo test --target $TARGET --release --all

}

# we don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi
