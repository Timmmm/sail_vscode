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
	});
	// On newer Node versions we can do `wasi.getImportObject()`.
	const importObject = { wasi_snapshot_preview1: wasi.wasiImport };
	const instance = await WebAssembly.instantiate(wasm, importObject);

	wasi.start(instance);
}

main();
