use file::File;
use serde::Deserialize;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::{cmp::Reverse, path::PathBuf};
use std::collections::hash_map::HashMap;
use lsp_types::{
    request, CompletionOptions, CompletionParams, CompletionResponse, DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidChangeWatchedFilesRegistrationOptions, DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams, FileSystemWatcher, GlobPattern, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams, Location, MessageType, OneOf, PublishDiagnosticsParams, Range, Registration, ServerCapabilities, SignatureHelp, SignatureHelpOptions, SignatureHelpParams, TextDocumentSyncCapability, TextDocumentSyncKind, Uri, WatchKind, WorkDoneProgressOptions, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities
};
use anyhow::{bail, Result};

mod text_document;

mod completion;
mod definitions;
mod diagnostics;
mod file;
mod files;
mod hover;
mod lsp;
mod signature;

fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    if uri.scheme().is_some_and(|s| s.as_str() == "file") {
        Some(uri.path().as_str().into())
    } else {
        None
    }
}

#[derive(Default)]
struct Server {
    disk_files: files::Files,
    open_files: HashMap<Uri, File>,
}

impl Server {
    /// Get all the files, ignoring files on disk that are also open.
    fn all_files(&self) -> impl Iterator<Item = (Uri, &File)> {
        let open_paths = self.open_files.keys().filter_map(uri_to_path).collect::<HashSet<_>>();

        self.open_files.iter().map(|(uri, file)| (uri.to_owned(), file)).chain(
            self.disk_files
                .all_files()
                .filter(move |(path, _)| !open_paths.contains(*path))
                .map(|(path, file)| (Uri::from_str(&format!("file://{}", path.display())).unwrap(), file))
        )
    }

    fn initialize(&mut self, params: InitializeParams) -> Result<InitializeResult> {
        eprintln!("server initialized");

        if let Some(workspace_folders) = params.workspace_folders {
            for folder in workspace_folders {
                if let Some(path) = uri_to_path(&folder.uri) {
                    self.disk_files.add_folder(path);
                }
            }
        }

        let folders = self.disk_files.folders().clone();
        self.disk_files.update(files::scan_folders(folders));

        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![" ".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                    all_commit_characters: None,
                    completion_item: None,
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec![" ,".to_string()]),
                    retrigger_characters: Some(vec![" ,".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                }),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    fn initialized(&self, _: InitializedParams) {
        eprintln!("server initialized");

        // Technically we should check if the client capabilities support this
        // but I can't be bothered.

        // TODO: This is kind of broken. If you rename a folder that contains some of
        // these files then you won't get a notification for them. The easiest
        // solution is to watch all files.

        // TODO: Restore this.
        // let result = self
        //     .client
        //     .register_capability(vec![Registration {
        //         id: "sail_watch_files_id".to_string(),
        //         method: "workspace/didChangeWatchedFiles".to_string(),
        //         register_options: Some(
        //             serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
        //                 watchers: vec![FileSystemWatcher {
        //                     glob_pattern: GlobPattern::String("**/*.sail".to_string()),
        //                     kind: Some(WatchKind::all()),
        //                 }],
        //             })
        //             .unwrap(),
        //         ),
        //     }])
        //     .await;

        // match result {
        //     Ok(()) => {
        //         self.client
        //             .log_message(MessageType::INFO, "registered file watcher")
        //             .await;
        //     }
        //     Err(e) => {
        //         self.client
        //             .log_message(
        //                 MessageType::ERROR,
        //                 format!("error registering file watcher: {:?}", e),
        //             )
        //             .await;
        //     }
        // }
    }

    fn did_change_workspace_folders(&mut self, params: DidChangeWorkspaceFoldersParams) {
        eprintln!("workspace folders changed");

        for folder in params.event.added.iter() {
            if let Some(path) = uri_to_path(&folder.uri) {
                self.disk_files.add_folder(path);
            }
        }
        for folder in params.event.removed.iter() {
            if let Some(path) = uri_to_path(&folder.uri) {
                self.disk_files.remove_folder(&path);
            }
        }
    }

    fn did_change_configuration(&self, _params: DidChangeConfigurationParams) {
        eprintln!("configuration changed");
    }

    fn did_change_watched_files(&mut self, params: DidChangeWatchedFilesParams) {
        let mut files = String::new();
        for change in &params.changes {
            files.push_str(&format!(" {:?}", change.uri));
        }
        eprintln!("watched files have changed: {}", files);

        for change in &params.changes {
            match change.typ {
                lsp_types::FileChangeType::DELETED => {
                    if let Some(path) = uri_to_path(&change.uri) {
                        self.disk_files.remove_file(&path);
                    }
                }
                lsp_types::FileChangeType::CREATED
                | lsp_types::FileChangeType::CHANGED => {
                    // Parse the file.
                    if let Some(path) = uri_to_path(&change.uri) {
                        if let Ok(source) = std::fs::read_to_string(path.clone()) {
                            let file = File::new(source);
                            self.disk_files.add_file(path, file);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Result<PublishDiagnosticsParams> {
        let uri = params.text_document.uri;
        eprintln!("file opened: {:?}", uri);

        let file = File::new(params.text_document.text);
        let diagnostics = file.diagnostics.clone();

        self.open_files.insert(uri.clone(), file);

        Ok(PublishDiagnosticsParams {
            version: Some(params.text_document.version),
            uri,
            diagnostics,
        })
    }

    fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Result<Option<PublishDiagnosticsParams>> {
        let uri = params.text_document.uri;
        eprintln!("file changed: {:?}", uri);

        if let Some(file) = self.open_files.get_mut(&uri) {
            file.update(params.content_changes);
            Ok(Some(PublishDiagnosticsParams {
                version: Some(params.text_document.version),
                uri,
                diagnostics: file.diagnostics.clone(),
            }))
        } else {
            Ok(None)
        }
    }

    fn did_save(&self, params: DidSaveTextDocumentParams) {
        eprintln!("file saved: {:?}", params.text_document.uri);
    }

    fn did_close(&mut self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        eprintln!("file closed: {:?}", uri);

        self.open_files.remove(&uri);
    }

    fn goto_definition(
        &mut self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        eprintln!("goto definition: {:?}", params);

        let uri = params.text_document_position_params.text_document.uri;

        let ident = self.open_files.get(&uri).and_then(|file| {
            let position = params.text_document_position_params.position;
            file.token_at(position).and_then(|(token, _)| {
                if let sail_parser::Token::Id(ident) = token {
                    Some(ident.clone())
                } else {
                    None
                }
            })
        });

        let ident = match ident {
            Some(ident) => ident,
            None => return Ok(None),
        };

        // TODO: This is currently limited to one definition per file
        // even though you can actually have more (e.g. for `overload`).
        let mut definitions = self.all_files()
            .filter_map(|(uri, file)| {
                if let Some(offset) = file.definitions.get(&ident) {
                    let position = file.source.position_at(*offset);
                    Some(Location::new(uri.clone(), Range::new(position, position)))
                } else {
                    None
                }
            }).collect::<Vec<_>>();

        // Sort by "distance" to the file from the currently open one,
        // as measured by the number of shared path components.
        // TODO: For some reason this doesn't quite work on Windows
        // because `uri.path_segments()` starts with `c%3A` sometimes
        // instead of `c:`. Also we should do case insensitive comparison
        // on Windows. Let's just give up on Windows for now.
        // TODO: Above comment was for server_url. Need to check again
        // for fluent_uri.

        // TODO: Restore.
        // definitions.sort_by_key(|location| Reverse(
        //     match (uri.path_segments(), location.uri.path_segments()) {
        //         (Some(p0), Some(p1)) =>
        //             p0.zip(p1).take_while(|(a, b)| a == b).count(),
        //         _ => 0,
        //     }
        // ));

        if definitions.is_empty() {
            Ok(None)
        } else {
            eprintln!("First definition URI: {:?}", definitions[0].uri);
            return Ok(Some(GotoDefinitionResponse::Array(definitions)));
        }
    }

    fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // TODO: Completion

        Ok(None)
    }

    fn hover(&self, _params: HoverParams) -> Result<Option<Hover>> {
        // TODO: Hover

        Ok(None)
    }

    fn signature_help(&self, _params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        // TODO: Signature help.

        Ok(None)
    }
}

fn handle<H, MethodParams, MethodResult>(output: impl Write, handler: H, request: lsp::Request) -> Result<()>
where
    H: FnOnce(MethodParams) -> Result<MethodResult>,
    for<'de> MethodParams: Deserialize<'de>,
    MethodResult: serde::Serialize,
{
    let params = match serde_json::from_value(request.params) {
        Ok(params) => params,
        Err(e) => {
            lsp::send_error_response(output, Some(request.id), lsp::ERROR_INVALID_PARAMS, format!("Invalid params: {}", e))?;
            return Ok(());
        }
    };
    match handler(params) {
        Ok(result) => {
            lsp::send_result_response(output, request.id, &result)?;
        },
        Err(e) => {
            lsp::send_error_response(output, Some(request.id), lsp::ERROR_INTERNAL_ERROR, format!("Handler error: {}", e))?;
        },
    }
    Ok(())
}

fn main() -> Result<()> {
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();

    let mut stdin_buf_read = std::io::BufReader::new(stdin);
    // stdout is already line buffered which is sufficient since we only write
    // a few lines per response.

    let mut server = Server::default();

    loop {
        let request = match lsp::receive_request(&mut stdin_buf_read) {
            Ok(request) => request,
            Err(e) => {
                lsp::send_error_response(&mut stdout, None, lsp::ERROR_INVALID_REQUEST, format!("Invalid JSON-RPC request: {}", e))?;
                continue;
            },
        };

        if request.jsonrpc != "2.0" {
            lsp::send_error_response(&mut stdout, Some(request.id), lsp::ERROR_INVALID_REQUEST, format!("Invalid JSON-RPC version: {:?}", request.jsonrpc))?;
            continue;
        }

        eprintln!("request: {}", request.method);

        match request.method.as_str() {
            "initialize" => {
                handle(&mut stdout, |params| server.initialize(params), request)?;
            }
            _ => {
                lsp::send_error_response(&mut stdout, Some(request.id), lsp::ERROR_METHOD_NOT_FOUND, format!("Unknown method: {:?}", request.method))?;
            }
        }
    }
}
