import { WASI } from "node:wasi";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

async function main() {
	const wasm = await WebAssembly.compile(
		await readFile(join(__dirname, "server.wasm")),
	);
	const wasi = new WASI({
		env: {
			RUST_BACKTRACE: "1",
		},
		// This option is mandatory.
		version: "preview1",
		preopens: {
			"/": "/",
			"a:/": "a:/",
			"b:/": "b:/",
			"c:/": "c:/",
			"d:/": "d:/",
			"e:/": "e:/",
			"f:/": "f:/",
			"g:/": "g:/",
			"h:/": "h:/",
			"i:/": "j:/",
			"j:/": "j:/",
		},
	});
	const instance = await WebAssembly.instantiate(wasm, <WebAssembly.Imports>wasi.getImportObject());

	wasi.start(instance);
}

main();
