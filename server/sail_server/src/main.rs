use file::File;
use std::collections::hash_map::HashMap;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, DidChangeConfigurationParams,
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidChangeWatchedFilesRegistrationOptions, DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    FileSystemWatcher, GlobPattern, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverParams, HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams,
    Location, MessageType, OneOf, Range, Registration, ServerCapabilities, SignatureHelp,
    SignatureHelpOptions, SignatureHelpParams, TextDocumentSyncCapability, TextDocumentSyncKind,
    Url, WatchKind, WorkDoneProgressOptions, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod text_document;

mod completion;
mod definitions;
mod diagnostics;
mod file;
mod files;
mod hover;
mod signature;

#[derive(Default)]
struct State {
    disk_files: files::Files,
    open_files: HashMap<Url, File>,
}

impl State {
    /// Get all the files, ignoring files on disk that are also open.
    fn all_files(&self) -> impl Iterator<Item = (&Url, &File)> {
        self.open_files.iter().chain(
            self.disk_files
                .all_files()
                .filter(|(uri, _)| !self.open_files.contains_key(uri)),
        )
    }
}

struct Backend {
    state: Mutex<State>,
    client: Client,
}

impl Backend {
    pub fn new_with_client(client: Client) -> Self {
        Self {
            state: Mutex::new(State::default()),
            client,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "server initialized")
            .await;

        let mut state = self.state.lock().await;
        if let Some(workspace_folders) = params.workspace_folders {
            for folder in workspace_folders {
                state.disk_files.add_folder(folder.uri);
            }
        }

        let folders = state.disk_files.folders().clone();
        state.disk_files.update(files::scan_folders(folders));

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

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized")
            .await;

        // Technically we should check if the client capabilities support this
        // but I can't be bothered.

        // TODO: This is kind of broken. If you rename a folder that contains some of
        // these files then you won't get a notification for them. The easiest
        // solution is to watch all files.

        let result = self
            .client
            .register_capability(vec![Registration {
                id: "sail_watch_files_id".to_string(),
                method: "workspace/didChangeWatchedFiles".to_string(),
                register_options: Some(
                    serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                        watchers: vec![FileSystemWatcher {
                            glob_pattern: GlobPattern::String("**/*.sail".to_string()),
                            kind: Some(WatchKind::all()),
                        }],
                    })
                    .unwrap(),
                ),
            }])
            .await;

        match result {
            Ok(()) => {
                self.client
                    .log_message(MessageType::INFO, "registered file watcher")
                    .await;
            }
            Err(e) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("error registering file watcher: {:?}", e),
                    )
                    .await;
            }
        }
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        self.client
            .log_message(MessageType::INFO, "workspace folders changed")
            .await;

        let mut state = self.state.lock().await;

        for folder in params.event.added.iter() {
            state.disk_files.add_folder(folder.uri.clone());
        }
        for folder in params.event.removed.iter() {
            state.disk_files.remove_folder(&folder.uri);
        }
    }

    async fn did_change_configuration(&self, _params: DidChangeConfigurationParams) {
        self.client
            .log_message(MessageType::INFO, "configuration changed")
            .await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let mut files = String::new();
        for change in &params.changes {
            files.push_str(&format!(" {}", change.uri));
        }
        self.client
            .log_message(
                MessageType::INFO,
                format!("watched files have changed: {}", files),
            )
            .await;

        let mut state = self.state.lock().await;
        for change in &params.changes {
            match change.typ {
                tower_lsp::lsp_types::FileChangeType::DELETED => {
                    state.disk_files.remove_file(&change.uri);
                }
                tower_lsp::lsp_types::FileChangeType::CREATED
                | tower_lsp::lsp_types::FileChangeType::CHANGED => {
                    // Parse the file.
                    if change.uri.scheme() == "file" {
                        if let Ok(path) = change.uri.to_file_path() {
                            if let Ok(source) = std::fs::read_to_string(path) {
                                let file = File::new(source);
                                state.disk_files.add_file(change.uri.clone(), file);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("file opened: {}", params.text_document.uri),
            )
            .await;

        let uri = &params.text_document.uri;

        let mut state = self.state.lock().await;

        let file = File::new(params.text_document.text /*, true*/);

        self.client
            .publish_diagnostics(uri.clone(), file.diagnostics.clone(), None)
            .await;

        state.open_files.insert(uri.clone(), file);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("file changed: {}", params.text_document.uri),
            )
            .await;

        let uri = &params.text_document.uri;

        let mut state = self.state.lock().await;

        let file = state
            .open_files
            .get_mut(uri)
            .expect("document changed that isn't open");
        file.update(params.content_changes);

        self.client
            .publish_diagnostics(uri.clone(), file.diagnostics.clone(), None)
            .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("file saved: {}", params.text_document.uri),
            )
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("file closed: {}", params.text_document.uri),
            )
            .await;
        let uri = &params.text_document.uri;

        let mut state = self.state.lock().await;
        state.open_files.remove(uri);
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        self.client
            .log_message(MessageType::INFO, format!("goto definition: {:?}", params))
            .await;

        let uri = &params.text_document_position_params.text_document.uri;
        let state = self.state.lock().await;
        let file = state
            .open_files
            .get(uri)
            .expect("definition for file that isn't open");

        let position = params.text_document_position_params.position;

        if let Some(token) = file.token_at(position) {
            if let (sail_parser::Token::Id(ident), _) = token {
                // Search the files in an arbitrary order currently.
                // TODO: Smarter order. I think VSCode favours the first one.
                // TODO: This is currently limited to one definition per file
                // even though you can actually have more (e.g. for `overload`).
                let definitions = state.all_files()
                    .filter_map(|(uri, file)| {
                        if let Some(offset) = file.definitions.get(ident) {
                            let position = file.source.position_at(*offset);
                            Some(Location::new(uri.clone(), Range::new(position, position)))
                        } else {
                            None
                        }
                    }).collect::<Vec<_>>();

                if !definitions.is_empty() {
                    return Ok(Some(GotoDefinitionResponse::Array(definitions)));
                }
            }
        }
        Ok(None)
    }

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // self.client
        //     .log_message(MessageType::INFO, format!("completion: {:?}", params))
        //     .await;

        // let uri = &params.text_document_position.text_document.uri;
        // let state = self.state.lock().await;
        // let _file = state
        //     .open_files
        //     .get(uri)
        //     .expect("completion for file that isn't open");

        // TODO: Completion

        Ok(None)
    }

    async fn hover(&self, _params: HoverParams) -> Result<Option<Hover>> {
        // self.client
        //     .log_message(MessageType::INFO, format!("hover: {:?}", params))
        //     .await;

        // let uri = &params.text_document_position_params.text_document.uri;
        // let state = self.state.lock().await;
        // let _file = state
        //     .open_files
        //     .get(uri)
        //     .expect("hover for file that isn't open");

        // TODO: Hover

        Ok(None)
    }

    async fn signature_help(&self, _params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        // self.client
        //     .log_message(MessageType::INFO, format!("signature: {:?}", params))
        //     .await;

        // let uri = &params.text_document_position_params.text_document.uri;
        // let state = self.state.lock().await;
        // let _file = state
        //     .open_files
        //     .get(uri)
        //     .expect("signature for file that isn't open");

        // TODO: Signature help.

        Ok(None)
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new_with_client);
    Server::new(stdin, stdout, socket).serve(service).await;
}
