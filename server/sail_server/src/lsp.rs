use std::io::{BufRead, Write};
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Request {
    /// JSON-RPC version. Always "2.0".
    pub jsonrpc: String,

    /// The request ID.
    pub id: u64,

    /// The method, e.g. "textDocument/completion"
    pub method: String,

    /// The parameters for the method. This is a JSON object.
    pub params: serde_json::Value,
}

#[derive(Serialize)]
pub struct Response {
    /// JSON-RPC version. Must be "2.0".
    pub jsonrpc: String,

    /// The request ID.
    pub id: u64,

    /// The result of the request. This is a JSON object.
    pub result: serde_json::Value,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    /// JSON-RPC version. Must be "2.0".
    pub jsonrpc: String,

    /// The request ID. Can be `null` for errors.
    pub id: Option<u64>,

    /// The error object.
    pub error: Error,
}

#[derive(Serialize)]
pub struct Error {
    /// The error code.
    pub code: i64,

    /// A short description of the error.
    pub message: String,
}

// Standard JSON-RPC error codes.
pub const ERROR_PARSE_ERROR: i64 = -32700;
pub const ERROR_INVALID_REQUEST: i64 = -32600;
pub const ERROR_METHOD_NOT_FOUND: i64 = -32601;
pub const ERROR_INVALID_PARAMS: i64 = -32602;
pub const ERROR_INTERNAL_ERROR: i64 = -32603;

/// Receive a message from the client.
pub fn receive_request(mut r: impl BufRead) -> Result<Request> {
    // Read headers.
    let mut content_length = None;
    let mut content_type = None;

    for line in r.by_ref().lines() {
        let line = line?;
        if line.is_empty() {
            break;
        }
        // Split key: value.
        let (header, value) = line.split_once(':').ok_or_else(|| anyhow!("Invalid request"))?;
        match header.to_ascii_lowercase().as_str() {
            "content-length" => {
                if content_length.is_some() {
                    bail!("Duplicate Content-Length header");
                }
                content_length = Some(value.trim().parse()?);
            }
            "content-type" => {
                if content_type.is_some() {
                    bail!("Duplicate Content-Type header");
                }
                content_type = Some(value.trim().to_string());
            }
            _ => bail!("Unrecognised LSP header: {}", header),
        }
    }

    if content_type.as_deref().is_some_and(|ct| ct != "application/vscode-jsonrpc; charset=utf-8") {
        bail!("Invalid Content-Type header: {:?}", content_type);
    }

    if let Some(content_length) = content_length {
        // Read the content.
        let mut content = vec![0; content_length];
        r.read_exact(&mut content)?;

        // Parse the content.
        let request: Request = serde_json::from_slice(&content)?;
        Ok(request)
    } else {
        bail!("Missing Content-Length header");
    }
}

/// Send a response to the client. `response` should either be a Response
/// or ErrorResponse.
pub fn send_response(mut w: impl Write, response: &impl Serialize) -> Result<()> {
    let json = serde_json::to_string(response)?;
    let bytes = json.as_bytes();

    // Write headers.
    writeln!(w, "Content-Length: {}\r", bytes.len())?;
    writeln!(w, "Content-Type: application/vscode-jsonrpc; charset=utf-8\r")?;
    writeln!(w, "\r")?;
    // Write content.
    w.write_all(bytes)?;

    Ok(())
}

/// Convenience for send_response.
pub fn send_result_response(w: impl Write, id: u64, result: &impl Serialize) -> Result<()> {
    send_response(w, &Response {
        jsonrpc: "2.0".to_string(),
        id,
        result: serde_json::to_value(result)?,
    })
}

/// Convenience for send_response.
pub fn send_error_response(w: impl Write, id: Option<u64>, code: i64, message: String) -> Result<()> {
    send_response(w, &ErrorResponse {
        jsonrpc: "2.0".to_string(),
        id,
        error: Error {
            code,
            message,
        },
    })
}
