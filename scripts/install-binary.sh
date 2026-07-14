#!/usr/bin/env sh
set -eu

REPO_OWNER="rmarganti"
REPO_NAME="herdr-pluck"
BIN_DIR="bin"
BIN_PATH="$BIN_DIR/herdr-pluck"
log() {
    printf '%s\n' "$*" >&2
}

plugin_version() {
    awk -F'"' '/^version = "/ { print $2; exit }' herdr-plugin.toml
}

archive_name() {
    version="$1"
    target="$2"
    printf '%s-v%s-%s.tar.gz' "$REPO_NAME" "$version" "$target"
}

release_url() {
    version="$1"
    archive="$2"
    printf 'https://github.com/%s/%s/releases/download/v%s/%s' "$REPO_OWNER" "$REPO_NAME" "$version" "$archive"
}

current_target() {
    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Darwin)
            case "$arch" in
                arm64|aarch64) printf 'aarch64-apple-darwin' ;;
                *) log "Unsupported macOS architecture: $arch"; exit 1 ;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64|amd64) printf 'x86_64-unknown-linux-musl' ;;
                *) log "Unsupported Linux architecture: $arch"; exit 1 ;;
            esac
            ;;
        *)
            log "Unsupported operating system: $os"
            exit 1
            ;;
    esac
}

have_command() {
    command -v "$1" >/dev/null 2>&1
}

download_release_binary() {
    version="$1"
    target="$2"
    archive=$(archive_name "$version" "$target")
    url=$(release_url "$version" "$archive")
    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT INT TERM HUP

    log "Downloading $url"

    if have_command curl; then
        curl -fsSL "$url" -o "$tmpdir/$archive" || return 1
    elif have_command wget; then
        wget -qO "$tmpdir/$archive" "$url" || return 1
    else
        log "Need curl or wget to install release binary"
        return 1
    fi

    mkdir -p "$BIN_DIR"
    tar -xzf "$tmpdir/$archive" -C "$tmpdir"
    cp "$tmpdir/herdr-pluck" "$BIN_PATH"
    chmod +x "$BIN_PATH"

    rm -rf "$tmpdir"
    trap - EXIT INT TERM HUP
}

build_from_source() {
    log "Building from local source with cargo"
    cargo build --release
    mkdir -p "$BIN_DIR"
    cp target/release/herdr-pluck "$BIN_PATH"
    chmod +x "$BIN_PATH"
}

main() {
    version=$(plugin_version)
    target=$(current_target)

    if [ "${HERDR_PLUCK_BUILD_FROM_SOURCE:-0}" = "1" ]; then
        if ! have_command cargo; then
            log "HERDR_PLUCK_BUILD_FROM_SOURCE=1 requires cargo"
            exit 1
        fi

        build_from_source
        log "Installed $BIN_PATH from local source"
        exit 0
    fi

    if download_release_binary "$version" "$target"; then
        log "Installed $BIN_PATH for $target"
        exit 0
    fi

    if have_command cargo; then
        log "No prebuilt binary found for version $version on $target"
        build_from_source
        log "Installed $BIN_PATH from local source"
        exit 0
    fi

    log "No prebuilt binary found for version $version on $target, and cargo is unavailable"
    exit 1
}

main "$@"
