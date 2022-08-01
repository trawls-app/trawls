# Trawls
<img align="right" width="128" height="128" src="src-tauri/icons/128x128.png">

![CI dev branch](https://github.com/Silberschleier/trawls/actions/workflows/test-on-pr.yml/badge.svg?branch=dev)

Trawls is a fast image processing tool to merge nightsky RAW photos to create a startrail image and output it as another RAW file.


## Download
Pre-compiled packages for Linux, macOS and Windows can be found and downloaded on the [release page](https://github.com/Silberschleier/trawls/releases).

## Project setup
Trawls is based on [Tauri](https://tauri.studio/en/) and uses [Vue](https://vuejs.org/) for its user interface.
Tauri is integrated using a vue-cli plugin, such that setup, compilation and building of the Rust parts can be controlled through `yarn`.

```
CXXFLAGS="--std=c++14" yarn install
```

### Compiles and hot-reloads for development
```
yarn run tauri:serve
```

### Compiles, minifies and packages for production
```
yarn run tauri:build
```

### Lints and fixes Vue/JS files
```
yarn run lint
```

### Lints Rust code
```
cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml
```
