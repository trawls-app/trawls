# Trawls
![CI dev branch](https://github.com/Silberschleier/trawls/actions/workflows/test-on-pr.yml/badge.svg?branch=dev)

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

### Lints and fixes files
```
npm run lint
```

