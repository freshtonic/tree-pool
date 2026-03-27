# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/freshtonic/tree-pool/compare/v0.1.0...v0.1.1) - 2026-03-27

### Fixed

- *(ci)* add success check and remove YAML anchors in release-plz workflow
- set git user config in clone test for CI compatibility

### Other

- restrict release-plz to CI runs on main branch
- bump actions/checkout to v5 for Node.js 24 compatibility
- add Rust dependency caching with Swatinem/rust-cache
- change release-plz trigger to manual workflow_dispatch
