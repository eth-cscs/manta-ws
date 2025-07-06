# Changelog

All notable changes to this project will be documented in this file.

## What's Changed in v0.1.15
* chore: Release manta-ws version 0.1.15 by @Masber
* refactor: We did not really have a valid way to return errors in manta-ws, this fix tries to improve the situation by returning the error from the http client that talks to the backend by @Masber
* fix: fix bug by @Masber
* refactor: in order to simplify the files in the project, we will reduce the number of files related to the redfish functionality from 2 to 1 by @Masber
* chore: update CHANGELOG.md for v0.1.14 by @actions-user

**Full Changelog**: https://github.com/eth-cscs/manta-ws/compare/v0.1.14...v0.1.15

## What's Changed in v0.1.14
* chore: Release manta-ws version 0.1.14 by @Masber
* chore: update Cargo.toml by @Masber
* Feature/power status by @Masber in [#24](https://github.com/eth-cscs/manta-ws/pull/24)
* fix: change keyworkd "ref" -> "branch" in Cargo.toml by @aescoubas
* fix: reference projects git repositories by @aescoubas
* feat: power-status by @aescoubas
* chore: update CHANGELOG.md for v0.1.13 by @actions-user

**Full Changelog**: https://github.com/eth-cscs/manta-ws/compare/v0.1.13...v0.1.14

## What's Changed in v0.1.13
* chore: Release manta-ws version 0.1.13 by @Masber
* fix: update function to create a redfish endpoint to accept same struct as the backend dispatcher by @Masber
* feat: add endpoints related to redfish endpoints by @Masber
* chore: update CHANGELOG.md for v0.1.12 by @actions-user

**Full Changelog**: https://github.com/eth-cscs/manta-ws/compare/v0.1.12...v0.1.13

## What's Changed in v0.1.12
* chore: Release manta-ws version 0.1.12 by @Masber
* feat: add new functionality to delete boot parameters by @Masber
* chore: update CHANGELOG.md for v0.1.11 by @actions-user

**Full Changelog**: https://github.com/eth-cscs/manta-ws/compare/v0.1.11...v0.1.12

## What's Changed in v0.1.11
* chore: Release manta-ws version 0.1.11 by @Masber
* chore: Release manta-ws version 0.1.10 by @Masber
* feat: add endpoints for BSS to create and get entities by @Masber
* chore: update CHANGELOG.md for v0.1.9 by @actions-user

**Full Changelog**: https://github.com/eth-cscs/manta-ws/compare/v0.1.9...v0.1.11

## What's Changed in v0.1.9
* chore: Release manta-ws version 0.1.9 by @Masber
* chore: Release manta-ws version 0.1.8 by @Masber
* fix: handlers to return struct that implements IntoResponse by @Masber
* chore: get/post boot parameters by @Masber
* refactor: clean code by @Masber
* refactor: clean code by @Masber
* chore: update CHANGELOG.md for v0.1.7 by @actions-user
* chore: improve the docker image creation by @Masber in [#19](https://github.com/eth-cscs/manta-ws/pull/19)
* chore(docker): add again jobs flag by @t-h2o
* feat(rust): add optimizations for building release
* chore(docker): change the way to build the binary
* chore(docker): add flag to apt
* chore(docker): upgrade the builder
* chore(docker): use the complete path
* chore(docker): as casing
* feat: auto changelog generation on main branch by @Masber in [#20](https://github.com/eth-cscs/manta-ws/pull/20)
* add the changelog generation to main branch by @aescoubas
* chore: update CHANGELOG.md for v0.1.5 by @actions-user
* chore: typo by @aescoubas
* add auto-changelog feature by @aescoubas

## New Contributors
* @actions-user made their first contribution
* @t-h2o made their first contribution

**Full Changelog**: https://github.com/eth-cscs/manta-ws/compare/v0.1.7...v0.1.9

## What's Changed in v0.1.7
* chore: Release manta-ws version 0.1.7 by @Masber
* fix: websockets connecting to node's console by @Masber

**Full Changelog**: https://github.com/eth-cscs/manta-ws/compare/v0.1.6...v0.1.7

## What's Changed in v0.1.6
* chore: Release manta-ws version 0.1.6 by @Masber
* fix: websockets connecting to node's console by @Masber

**Full Changelog**: https://github.com/eth-cscs/manta-ws/compare/v0.1.5...v0.1.6

## What's Changed in v0.1.5
* chore: Release manta-ws version 0.1.5 by @Masber
* chore: clean code by @Masber

**Full Changelog**: https://github.com/eth-cscs/manta-ws/compare/v0.1.4...v0.1.5

## What's Changed in v0.1.4
* chore: Release manta-ws version 0.1.4 by @Masber
* feat: clean and update power management commands by @Masber
* chore: cargo fix by @Masber
* fix: clean code by @Masber
* Fix/pipeline tests by @Masber in [#18](https://github.com/eth-cscs/manta-ws/pull/18)
* Add a cargo install command in pull request pipeline to check if package builds by @aescoubas
* split pipeline into two jobs by @aescoubas
* sanitize the tag name by @aescoubas
* Fix matching pattern in github action rules by @aescoubas
* add a cargo test command and disable building image during merge request checks by @aescoubas
* Fix/dependencies by @Masber in [#17](https://github.com/eth-cscs/manta-ws/pull/17)
* Merge branch 'main' into fix/dependencies by @Masber

**Full Changelog**: https://github.com/eth-cscs/manta-ws/compare/v0.1.3...v0.1.4

## What's Changed in v0.1.3
* chore: Release manta-ws version 0.1.3 by @Masber
* fix: get /hsm by @Masber
* Merge branch 'main' into fix/dependencies by @Masber
* feature: OpenAPI base setup by @Masber in [#16](https://github.com/eth-cscs/manta-ws/pull/16)
* Remove useless file by @aescoubas
* feature: OpenAPI base setup by @aescoubas
* fix: bring missing changes from gitlab by @Masber
* chore: code documentation by @Masber
* Deprecation of serde_yaml by @aescoubas
* Upgrade axum and various other dependencies by @aescoubas
* update hyper version by @aescoubas
* low-hanging fruits dependencies updates by @aescoubas
* fix: previous mistake in how to skip ci run [skip ci] by @aescoubas
* document: Add details on pipeline in README.md by @aescoubas

**Full Changelog**: https://github.com/eth-cscs/manta-ws/compare/v0.1.2...v0.1.3

## What's Changed in v0.1.2
* fix: wrong year for Cargo by @aescoubas
* fix: add the proper version in Cargo.toml and retag the appropriate commit by @aescoubas
* Merge branch 'dev' by @aescoubas
* Merge branch 'main' into dev by @aescoubas
* refactor: change project name from cama to manta-ws by @aescoubas
* feature: add the possibility to trigger build manually by @aescoubas
* Cleanup old gitlab ci file by @aescoubas
* fix: Freeze dependencies versions and add github actions by @aescoubas
* Rename project from "cama" to "manta-ws" by @aescoubas in [#15](https://github.com/eth-cscs/manta-ws/pull/15)
* Dev by @aescoubas in [#14](https://github.com/eth-cscs/manta-ws/pull/14)
* fix: update gitlab pipeline by @Masber
* fix: update gitlab pipeline by @Masber
* chore: update Cargo.toml by @Masber
* fix: update backend libraries by @Masber
* chore: update rust builder container image version by @Masber
* chore: update Cargo.toml by @Masber
* fix: set exact mesa version in Cargo.toml by @Masber
* fix: install cmake to compile librdkafka create by @Masber
* feat: fix CID pipeline by @Masber
* fix: compilation errors after upgrade mesa version by @Masber
* chore: rename application from manta-ws to api-server by @Masber
* chore: test build container image in gitlab pipeline by @Masber
* chore: test build container image in gitlab pipeline by @Masber
* chore: test build container image in gitlab pipeline by @Masber
* chore: test build container image in gitlab pipeline by @Masber
* fix: Dockerfile by @Masber
* fix: cfs logs by @Masber
* Update .gitlab-ci.yml file by @ManuelSopenaBallesteros
* fix(cicd): git runner tags by @Masber
* fix(cicd): git runner tags by @Masber
* fix(cicd): git runner tags by @Masber
* fix(cicd): git runner tags by @Masber
* feat: add gitlab pipeline by @Masber
* Merge branch 'release-plz-2024-12-28T20-41-45Z' into 'main' by @ManuelSopenaBallesteros
* chore: release v0.1.1 by @ManuelSopenaBallesteros
* fix: fix merge issues by @ManuelSopenaBallesteros
* feat: add option to create hsm group when migrating nodes by @ManuelSopenaBallesteros
* fix: install script not deploying properly by @ManuelSopenaBallesteros
* feat: add new endpoint to check cama version by @ManuelSopenaBallesteros
* feat: new endpoint to migrate nodes between hsm groups by @ManuelSopenaBallesteros
* feat: test PUT with url and query params by @ManuelSopenaBallesteros
* Closes #9 - Create functionality to get kernel parameters by @Masber in [#10](https://github.com/eth-cscs/manta-ws/pull/10)
* refactor: trying to use more idiomatic Rust code by @ManuelSopenaBallesteros
* Closes #9 - Create functionality to get kernel parameters by @matteo-chesi
* feat: update mesa version by @ManuelSopenaBallesteros
* Feature required to compile on my laptop by @matteo-chesi
* refactor: fix tokio features by @ManuelSopenaBallesteros
* refactor: improve installation script by @ManuelSopenaBallesteros
* ISSUE#1 add BOS service health and first refactoring attempt to CFS one. by @Masber in [#8](https://github.com/eth-cscs/manta-ws/pull/8)
* ISSUE#1 add BOS service health and first refactoring attempt to CFS one. by @matteo-chesi
* refactor: update README by @ManuelSopenaBallesteros
* fix: server listens to any address by @ManuelSopenaBallesteros
* refactor: whoami endpoint returning string rather than printing on the server stdout by @ManuelSopenaBallesteros
* feat: update cama version by @ManuelSopenaBallesteros
* refactor: add command to test http request in README by @ManuelSopenaBallesteros
* fix: code fixes by @ManuelSopenaBallesteros
* feat: add new endpoint to check CFS service health by @ManuelSopenaBallesteros
* refactor: fix README issues by @ManuelSopenaBallesteros
* refactor: house keeping by @ManuelSopenaBallesteros
* refactor: house keeping by @ManuelSopenaBallesteros
* feat: add cfs configuration used to build image when listing nodes configuration related to a HSM group by @ManuelSopenaBallesteros
* refactor: adapt to mesa code by @ManuelSopenaBallesteros
* feat: add new features to manage nodes power state by @ManuelSopenaBallesteros
* refactor: adapt to new mesa code by @ManuelSopenaBallesteros
* feat: start login page by @ManuelSopenaBallesteros
* refactor: clean files and update README by @ManuelSopenaBallesteros
* doc: add license file by @ManuelSopenaBallesteros
* feat: add frontend by @ManuelSopenaBallesteros
* increase buffer/scrollback size and disable linewrap
* change tab title and clean code
* fix bug with xterm not being able to handle TTY echo off from remote
* clean frontend files
* initial commit with cfs session logs and xname console features
* init

## New Contributors
* @aescoubas made their first contribution
* @Masber made their first contribution
* @ManuelSopenaBallesteros made their first contribution
* @matteo-chesi made their first contribution
* @ made their first contribution

<!-- generated by git-cliff -->
