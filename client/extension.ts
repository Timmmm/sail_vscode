// Based on https://github.com/microsoft/vscode-extension-samples/blob/main/lsp-sample/client/src/extension.ts (MIT licensed).

import { ExtensionContext } from "vscode";
import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
} from "vscode-languageclient/node";
import { join } from "node:path";
import { fork } from "node:child_process";

let client: LanguageClient | undefined;

export function activate(_context: ExtensionContext): void {
	console.log("Activating Sail extension");

	let serverOptions: ServerOptions = async () => fork(
			join(__dirname, "server_launcher.js"),
			{
				stdio: "pipe",
			},
		);

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
