#!/usr/bin/env bash

set -e

CRATES=( \
    jsona \
    jsona-ast \
    jsona-schema \
    jsona-schema-validator \
    jsona-util \
    jsona-lsp \
    jsona-cli \
)

npm_vars() {
    pkg_names=( $(yarn workspaces info | sed '1d;$d' | jq -r 'to_entries[] | .key' | tr -d '\r') )
    pkg_paths=( $(yarn workspaces info | sed '1d;$d' | jq -r 'to_entries[] | .value.location' | tr -d '\r') )
}

# @cmd Prepare env
prepare() {
    cargo --version
    jq --version
    yarn
}

# @cmd Build all
build() {
    cargo build -r -p jsona
    build-js all
    vscode.pkg
}

# @cmd Build js modules
# @arg name[core|schema|lsp|util-types|all]
build-js() {
    name=$1
    npm_vars
    _build_js() {
        local path=$1
        echo Build $path
        (cd $path && npm run build)
    }
    if [[ $# -eq 0 ]] || [[ $name == all ]]; then
        for path in ${pkg_paths[@]}; do
            if [[ $path =~ vscode ]]; then
                continue
            fi
            name=${path##*/}
            _build_js $path
        done
    else
        for path in ${pkg_paths[@]}; do
            if [[ $path =~ $name ]]; then
                _build_js $path
                break
            fi
        done
    fi
}

# @cmd Test js
test.js() {
    (cd tests/js && npm test $@)
}

# @cmd Build vscode extension
# @arg kind[node|browser]
vscode.build() {
    pushd editors/vscode > /dev/null
    if [[ "$1" == "browser" ]]; then
        npm run build:browser-server
        npm run build:browser-extension
    elif [[ "$1" == "node" ]]; then
        npm run build:node
    else
        npm run build
    fi
    popd > /dev/null
}

# @cmd Run web extension in chrome
# @flag --no-build
# @arg entry=tests/fixtures
vscode.web() {
    if [[ -z $argc_no_build ]]; then
        export DEBUG="true"
        export LOG_TOPICS="worker,lsp" 
        vscode.build browser
    fi
    npx @vscode/test-web --browserType=chromium --extensionDevelopmentPath=editors/vscode $argc_entry # vscode-test-web
}

# @cmd Package vscode extension
vscode.pkg() {
    pushd editors/vscode > /dev/null
        npm run package
        pkg_ver=$(node -p "require('./package.json').version")
        ls -lh vscode-jsona-$pkg_ver.vsix
    popd > /dev/null
}

# @cmd Update crate version
# @arg crates* crate with version, e.g. jsona@0.5.1
version.crate() {
    if [[ $# -eq 0 ]]; then
        for name in ${CRATES[@]}; do
            id=$(cargo pkgid -p $name)
            echo $name@${id##*#}
        done
    else
        for item in ${argc_crates[@]}; do
            local name=${item%%@*}
            local version=${item##*@}
            local minor=${version%.*}
            sed -i 's/^version = ".*"/version = "'$version'"/' crates/$name/Cargo.toml
            for crate in crates/*; do
                sed -i 's|path = "../'$name'", version = ".*"|path = "../'$name'", version = "'$minor'"|' $crate/Cargo.toml
            done
        done
    fi
}


# @cmd Update npm version
# @arg modules* npm module with version, e.g. @jsona/core@0.1.2
version.npm() {
    npm_vars
    if [[ $# -eq 0 ]]; then
        for i in ${!pkg_names[@]}; do
            pkg_ver=$(node -p "require('"${pkg_paths[$i]}"/package.json').version")
            echo ${pkg_names[$i]}@$pkg_ver
        done
    else
        for item in ${argc_modules[@]}; do
            name=${item%@*}
            version=${item##*@}
            for i in ${!pkg_names[@]}; do
                if [[ ${pkg_names[$i]} = $name ]]; then
                    path=${pkg_paths[$i]}
                    break
                fi
            done
            if [ -z $path ]; then
                echo "Not found $name"
                continue
            fi
            sed -i 's/^  "version": ".*",/  "version": "'$version'",/' $path/package.json
            for dep_path in ${pkg_paths[@]}; do
                sed -i 's|"'$name'": ".*"|"'$name'": "^'$version'"|' $dep_path/package.json
            done
        done
    fi
}

# @cmd Publish crate to carte.io
publish.crate() {
    for name in ${CRATES[@]}; do
        online_ver=$(curl -fsSL https://crates.io/api/v1/crates/$name 2>/dev/null | jq -r '.crate.newest_version')
        crate_ver=$(cargo pkgid -p $name | sed 's/.*#//')
        if [[ "$online_ver" != "$crate_ver" ]]; then
            read -p "Upgrade $name from $online_ver to $crate_ver (y/n)? " choice
            if [[ "$choice" == y ]]; then
                cargo publish -p $name
                sleep 15
            fi
        fi
    done
}


# @cmd Publish to npm
publish.npm() {
    npm_vars
    for i in ${!pkg_names[@]}; do
        local name=${pkg_names[$i]}
        local path=${pkg_paths[$i]}
        local access=$(node -p "require('./"$path"/package.json').publishConfig?.access")
        if [[ $access != "public" ]]; then
            continue
        fi
        local online_ver=$(npm show $name version)
        local pkg_ver=$(node -p "require('./"$path"/package.json').version")
        if [[ "$online_ver" != "$pkg_ver" ]]; then
            read -p "Upgrade $name from $online_ver to $pkg_ver (y/n)? " choice
            if [[ "$choice" == y ]]; then
                (cd $path && npm publish)
                sleep 15
            fi
        else
            echo @$name:$pkg_ver is up to date
        fi
    done
}

# @cmd Run jsona-cli
# @arg args*
run() {
    cargo run -p jsona-cli -- $@
}

# @cmd Print jsona syntax
# @arg jsona_file!
run.syntax() {
    cargo run -p jsona --example syntax -- $argc_jsona_file
}

# @cmd Parse jsona as ast
# @arg jsona_file!
run.to-ast() {
    cargo run -p jsona-ast --example to-ast -- $argc_jsona_file
}

# @cmd Generate jsona from ast
# @arg ast_file!
run.from-ast() {
    cargo run -p jsona-ast --example from-ast -- $argc_ast_file
}

# @cmd Format jsona doc
# @arg jsona_file!
run.format() {
    cargo run -p jsona --example format -- $argc_jsona_file
}

# @cmd Convert jsona jsonschema to plain jsonschema
# @arg jsona_file!
# @arg pointer
run.to-json-schema() {
    cargo run -p jsona-schema --example to-json-schema -- $argc_jsona_file $argc_pointer
}

# @cmd Get jsona schema value
# @arg jsona_file!
# @arg pointer
run.query-schema() {
    cargo run -p jsona-schema-validator --example query-schema -- $argc_jsona_file $argc_pointer
}
# @cmd Validate jsona file with a schema file
# @arg schema_file!
# @arg jsona_file!
run.validate() {
    cargo run -p jsona-schema-validator --example validate -- $argc_schema_file $argc_jsona_file
}

# @cmd Build and install jsona-cli to $HOME/.cargo/bin
# @alias i
# @flag --prod
install() {
    if [[ -n $argc_prod ]]; then
        cargo build -r -p jsona-cli
        cp -f target/release/jsona $HOME/.cargo/bin/
    else
        cargo build -p jsona-cli
        cp -f target/debug/jsona $HOME/.cargo/bin/
    fi
    ls -alh $HOME/.cargo/bin/jsona
}

eval "$(argc --argc-eval $0 "$@")"
