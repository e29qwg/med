# Minimal Editor (med)

## About
**med** is a text editor written in rust language.
This project is revised from [hecto](https://github.com/pflenker/hecto-tutorial) project.
However, this project aims to provide cross-platform compatability
utilizing [crossterm](https://github.com/crossterm-rs/crossterm)
and also to mimic subset of [vi/vim](https://github.com/vim/vim)'s frontend.

## Usage
```sh
med your_file.txt
```
While vim's frontend is unimplemented,
use <kbd>Esc</kbd> to quit and <kbd>Ctrl</kbd>+<kbd>S</kbd> to save.

## Licensing
This project is under [MIT](./LICENSE) license.

The predecessor project [hecto](https://github.com/pflenker/hecto-tutorial) is under Creative Commons [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) which is compatible with the MIT license.