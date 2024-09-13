import { WASI } from "node:wasi";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

async function main() {
	console.log("Sail LSP WASM launcher starting...");
	const wasm = await WebAssembly.compile(
		await readFile(join(__dirname, "server.wasm")),
	);
	const wasi = new WASI({
		env: {
			RUST_BACKTRACE: "1",
		},
		// This option is mandatory.
		version: "preview1",
	});
	const instance = await WebAssembly.instantiate(wasm, <WebAssembly.Imports>wasi.getImportObject());

	console.log("Sail LSP WASM start...");
	wasi.start(instance);
}

main();
