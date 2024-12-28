# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://git.cscs.ch/msopena/cama/releases/tag/v0.1.1) - 2024-12-28

### Added

- add option to create hsm group when migrating nodes
- add new endpoint to check cama version
- new endpoint to migrate nodes between hsm groups
- test PUT with url and query params
- update mesa version
- update cama version
- feat: add new endpoint to check CFS service health
- add cfs configuration used to build image when listing nodes configuration related to a HSM group
- feat: add new features to manage nodes power state
- start login page
- add frontend

### Fixed

- fix merge issues
- install script not deploying properly
- server listens to any address
- code fixes
- fix bug with xterm not being able to handle TTY echo off from remote

### Other

- trying to use more idiomatic Rust code
- Closes [#9](https://git.cscs.ch/msopena/cama/pulls/9) - Create functionality to get kernel parameters
- Feature required to compile on my laptop
- refactor: fix tokio features
- improve installation script
- Merge pull request [#8](https://git.cscs.ch/msopena/cama/pulls/8) from eth-cscs/1-feature-create-endpoint-to-check-if-bos-service-is-ready
- update README
- whoami endpoint returning string rather than printing on the server stdout
- add command to test http request in README
- fix README issues
- house keeping
- house keeping
- adapt to mesa code
- adapt to new mesa code
- clean files and update README
- add license file
- increase buffer/scrollback size and disable linewrap
- change tab title and clean code
- clean frontend files
- initial commit with cfs session logs and xname console features
- init
