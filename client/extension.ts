// Based on https://github.com/microsoft/vscode-extension-samples/blob/main/lsp-sample/client/src/extension.ts (MIT licensed).

import * as os from "os";
import { ExtensionContext } from "vscode";
import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

function serverExe(): string {
	switch (`${os.platform()}_${os.arch()}`) {
		case "linux_x64":
			return "dist/server_x86_64-unknown-linux-musl";
		case "darwin_arm64":
			return "dist/server_aarch64-apple-darwin";
		case "darwin_x64":
			return "dist/server_x86_64-apple-darwin";
		case "win32_x64":
			return "dist/server_x86_64-pc-windows-gnu.exe";
		default:
			throw new Error(`Unsupported platform: ${os.platform}`);
	}
}

export function activate(context: ExtensionContext): void {
	console.log("Activating Sail extension");

	// The server is implemented in Rust.
	let serverCommand = context.asAbsolutePath(serverExe());

	// If the extension is launched in debug mode then the debug server options are used
	// Otherwise the run options are used
	let serverOptions: ServerOptions = {
		run: { command: serverCommand, options: { env: { RUST_BACKTRACE: "1" } } },
		debug: { command: serverCommand, options: { env: { RUST_BACKTRACE: "1" } } },
	};

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
