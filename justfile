version := `cat Cargo.toml  | grep '^version = ' | sed 's/version = "\(.*\)"/\1/'`
release_dir := "target/wasm32-unknown-unknown/release/"

pkg: rm-pkg wasm
    wasm-bindgen --target nodejs --out-dir pkg {{release_dir}}/jsona_js.wasm
    cp lib/package.json lib/jsona_js.d.ts pkg/
    cp README.md pkg/
    sed -i 's/__VERSION__/{{version}}/' pkg/package.json

wasm:
    cargo build --release --target wasm32-unknown-unknown

test:
    node tests/test.js

rm-pkg:
    rm -rf pkg
