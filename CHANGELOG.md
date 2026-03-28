# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.1.2] - 2026-03-28

### Features

- Add shell completion support
## [0.1.1] - 2026-03-27

### Bug Fixes

- Filter release tags to valid semver only
- Compare against last release tag for artifact-relevant changes
- Preserve untracked files (build artifacts) during tree reset
- Add success check and remove YAML anchors in release-plz workflow
- Set git user config in clone test for CI compatibility
