#!/usr/bin/env bash

set -e

if [[ -n $1 && $1 == "--tag-commit" ]]; then
  TAG_COMMIT=1
fi

BRANCH="$DRONE_COMMIT_BRANCH"
new_version=""

# Expects a current version
# Optional major|minor|patch (default: patch)
function bump_patch() {
  local CURRENT_VERSION=$1
  local BUMP_VER=$([[ -z $2 ]] && echo "patch" || echo "$2")

  IFS='.' read -a version_parts <<<"$CURRENT_VERSION"

  major=${version_parts[0]}
  minor=${version_parts[1]}
  patch=${version_parts[2]}

  case "$BUMP_VER" in
  "major")
    major=$((major + 1))
    minor=0
    patch=0
    ;;
  "minor")
    minor=$((minor + 1))
    patch=0
    ;;
  "patch")
    patch=$((patch + 1))
    ;;
  esac

  new_version="$major.$minor.$patch"
}

# bump_file filename major|minor|patch
function bump_file() {
  local INPUT_FILE="$1"
  local BUMP_VER=$2

  local CURRENT_VERSION=$(grep -m 1 -o 'version = ".*"'  "$INPUT_FILE" | sed -n 's/.*version = "\(.*\)"/\1/p')
  bump_patch "$CURRENT_VERSION" "$BUMP_VER"

  local SEARCH='^(version[[:space:]]*=[[:space:]]*).+'
  local REPLACE="\1\"${new_version}\""

  sed -i.tmp -E "s/${SEARCH}/${REPLACE}/g" "$1"
  echo "$INPUT_FILE bumped from $CURRENT_VERSION to $new_version"
  rm "$INPUT_FILE.tmp"
}

FILES=("db/Cargo.toml" "api/Cargo.toml")

for target in "${FILES[@]}"; do
  BUMP_VER=$([[ "$BRANCH" == "master" ]] && echo "minor" || echo "patch")
  bump_file "$target" "$BUMP_VER"
  if [[ -n $TAG_COMMIT ]]; then
    git add "$target"
  fi
done

if [[ -n $TAG_COMMIT ]]; then
  mkdir -p "$HOME/.ssh/"

  ssh-keyscan github.com >~/.ssh/known_hosts
  eval $(ssh-agent -s)
  ssh-add <(echo "$GITHUB_SSH_KEY")

  git config --global user.email "travis@travis-ci.org"
  git config --global user.name "Travis CI"

  git checkout "$BRANCH"

  git remote add sshremote git@github.com:$DRONE_REPO.git

  git commit -m "Version bump to ${new_version} [skip ci]"
  git tag ${new_version}
  git push sshremote "$BRANCH"
  git push sshremote ${new_version}
fi
