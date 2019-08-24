# This script takes care of testing your crate

set -ex

main() {
    cross build --target $TARGET
    cross build --target $TARGET --release

    if [ ! -z $DISABLE_TESTS ]; then
        return
    fi

    cross test --target $TARGET
    cross test --target $TARGET --release

    # Run the cli with --help otherwise it won't return a success exit
    # code and cause ci to fail
    cross run --target $TARGET --help
    cross run --target $TARGET --release --help
}

# Don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi
