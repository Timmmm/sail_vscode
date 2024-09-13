// Based on https://github.com/microsoft/vscode-extension-samples/blob/main/lsp-sample/client/src/extension.ts (MIT licensed).

import { ExtensionContext } from "vscode";
import {
	Executable,
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
} from "vscode-languageclient/node";
import { join } from "node:path";

let client: LanguageClient | undefined;

export function activate(_context: ExtensionContext): void {
	console.log("Activating Sail extension");

	// The server is implemented in Rust.
	let launcherPath = join(__dirname, "server_launcher.js");

	// If the extension is launched in debug mode then the debug server options are used
	// Otherwise the run options are used

	let executableConfig: Executable = {
		command: process.execPath,
		args: ["--", launcherPath],
		options: {
			env: {
				// Get backtraces on Rust panics.
				RUST_BACKTRACE: "1",
				// `process.execPath` will be VSCode (Electron) so we need
				// to tell it to actually behave like Node and run the script.
				ELECTRON_RUN_AS_NODE: "1",
			},
		},
	};

	let serverOptions: ServerOptions = {
		run: executableConfig,
		debug: executableConfig,
	};

	console.log(`Running: ${process.execPath} -- ${launcherPath}`);

	// Options to control the language client
	let clientOptions: LanguageClientOptions = {
		// Register the server for Sail documents
		documentSelector: [{ scheme: "file", language: "sail" }],
	};

	// Create the language client and start the client.
	client = new LanguageClient(
		"sail",
		"Sail Language Support",
		serverOptions,
		clientOptions,
	);

	// Start the client. This will also launch the server
	client.start();
}

export function deactivate(): Thenable<void> | undefined {
	if (!client) {
		return undefined;
	}
	return client.stop();
}
