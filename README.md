# Trawls
![CI dev branch](https://github.com/Silberschleier/trawls/actions/workflows/test-on-pr.yml/badge.svg?branch=dev)

## Download
Pre-compiled packages for Linux, macOS and Windows can be found and downloaded on the [release page](https://github.com/Silberschleier/trawls/releases).

## Project setup
Trawls is based on [Tauri](https://tauri.studio/en/) and uses [Vue](https://vuejs.org/) for its user interface.
Tauri is integrated using a vue-cli plugin, such that setup, compilation and building of the Rust parts can be controlled through `npm`.

```
npm install
```

### Compiles and hot-reloads for development
```
npm run tauri:serve
```

### Compiles, minifies and packages for production
```
npm run tauri:build
```

### Lints and fixes Vue/JS files
```
npm run lint
```

### Lints Rust code
```
cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml
```