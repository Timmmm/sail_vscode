# Sail VSCode Extension

This extension provides basic support for writing Sail code. [Sail](https://github.com/rems-project/sail) is a programming language for describing the semantics of instruction set architectures (ISAs). It is [the official specification language for RISC-V](https://github.com/riscv/sail-riscv) but there are also Sail models for ARM and x86.

## Features / Differences from the official Extension

There is [an existing extension in the marketplace](https://marketplace.visualstudio.com/items?itemName=rems-project.sail) however it only does syntax highlighting, and notably it is missing highlighting for single line `// comments` which is quite annoying.

This extension is quite basic but it improves on that in two ways:

* Syntax highlighting for `// comments` is fixed.
* It has *basic* support for go-to definition. This is based on lexing; not parsing, so it only works some of the time. It is much much better than nothing though.

I have started working on a parser but unfortunately Sail is quite a difficult language to parse. It also doesn't have a proper module system yet so it is not especially IDE-friendly. If multiple files define the same function, when you go-to-definition a more or less random one will be picked. This is unfortunately very common because the only way to write extensible Sail files is to define the same function in multiple files and then only compile one of them. In RISC-V this is used for RV32/RV64 and also for CHERI.

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
