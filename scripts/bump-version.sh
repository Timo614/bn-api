#!/usr/bin/env bash

set -e

new_version=""

function bump_patch {
    local file="$1"
    local version=`sed -En 's/version[[:space:]]*=[[:space:]]*"([[:digit:]]+\.[[:digit:]]+\.[[:digit:]]+)"/\1/p' < $file`
    new_version=`echo $version | awk -F. -v OFS=. 'NF==1{print ++$NF}; NF>1{$NF=sprintf("%0*d", length($NF), ($NF+1)); print}'`
    local search='^(version[[:space:]]*=[[:space:]]*).+'
    local replace="\1\"${new_version}\""

    sed -i.tmp -E "s/${search}/${replace}/g" "$1"
    echo "$file bumped from $version to $new_version"
    rm "$1.tmp"
}

FILES=( "db/Cargo.toml" "api/Cargo.toml" )

for target in "${FILES[@]}"; do
    bump_patch "$target"
    if [[ $1 == "--tag-commit" ]]; then
        git add "$target"
    fi
done

if [[ $1 == "--tag-commit" ]]; then
    git commit -m  "Version bump to ${new_version} [skip ci]"
    git tag ${new_version}
    git push sshremote master
    git push sshremote ${new_version}
fi
