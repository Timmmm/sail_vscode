# Sail VSCode Extension

This extension provides basic support for writing Sail code.

## Features

* Basic syntax highlighting.
* Go-to definition.

## License

All code licensed under the MIT license (see [`LICENSE.md`](https://github.com/timmmm/sail_vscode/blob/master/LICENSE.md)), except `syntaxes/sail.tmLanguage.json` which was copied from the Sail project [here](https://github.com/rems-project/sail/blob/f3bf59ea8f8a44089a2fb3306c75f35279e156ce/editors/vscode/sail/syntaxes/sail.tmLanguage.json) and is 2-clause BSD licensed.

## Development

To build this you need:

* Rust
* NPM

Then run:

```
./make release
```

To test it open this directory in VSCode and run it (F5 or Debug->Start Debugging).

To build a VSIX package (VSCode extension package), run `./make package`.
