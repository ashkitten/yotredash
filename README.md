# Yotredash

[![Build Status][travis_badge]][travis_link] [![GPLv3][license_badge]][license_link]

[travis_badge]: https://travis-ci.org/ashkitten/yotredash.svg?branch=master
[travis_link]: https://travis-ci.org/ashkitten/yotredash
[license_badge]: https://img.shields.io/github/license/ashkitten/yotredash.svg
[license_link]: LICENSE

A shader demotool written in Rust

# Contributing

There is a git pre-commit hook in the `hooks` directory which runs `cargo fmt -- --write-mode=diff` before commit, which will fail the commit if it doesn't match the format guidelines. You can link the hook into your `.git` directory either by running the `hooks/link` script or with `ln -sf` on your own.
