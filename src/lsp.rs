mod ast;
mod lexer;
mod parser;

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::*;

fn main() {
    let (connection, io_threads) = Connection::stdio();
    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".to_string()]),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        ..Default::default()
    };
    let init_result = serde_json::json!({
        "capabilities": server_capabilities,
        "serverInfo": { "name": "zamin-lsp", "version": "0.1.0" }
    });
    connection.initialize(init_result).unwrap();

    main_loop(&connection);

    io_threads.join().unwrap();
}

fn main_loop(connection: &Connection) {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req).unwrap() {
                    return;
                }
                handle_request(&connection, req);
            }
            Message::Notification(not) => {
                handle_notification(&connection, not);
            }
            Message::Response(_) => {}
        }
    }
}

fn handle_request(connection: &Connection, req: Request) {
    match req.method.as_str() {
        "textDocument/completion" => {
            let params: CompletionParams = serde_json::from_value(req.params).unwrap();
            let items = get_completions(&params);
            let result = serde_json::to_value(items).unwrap();
            let resp = Response::new_ok(req.id, result);
            connection.sender.send(Message::Response(resp)).unwrap();
        }
        "textDocument/hover" => {
            let params: HoverParams = serde_json::from_value(req.params).unwrap();
            let result = get_hover(&params);
            let resp = Response::new_ok(req.id, serde_json::to_value(result).unwrap());
            connection.sender.send(Message::Response(resp)).unwrap();
        }
        _ => {}
    }
}

fn handle_notification(connection: &Connection, not: Notification) {
    match not.method.as_str() {
        "textDocument/didOpen" => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(not.params).unwrap();
            check_diagnostics(connection, &params.text_document.uri, &params.text_document.text);
        }
        "textDocument/didChange" => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(not.params).unwrap();
            if let Some(change) = params.content_changes.into_iter().last() {
                check_diagnostics(connection, &params.text_document.uri, &change.text);
            }
        }
        _ => {}
    }
}

fn check_diagnostics(connection: &Connection, uri: &Uri, text: &str) {
    let mut diagnostics = Vec::new();
    let mut p = parser::Parser::new(text);
    match p.parse() {
        Ok(_) => {}
        Err(e) => {
            let msg = format!("{}", e);
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 1 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: msg,
                ..Default::default()
            });
        }
    }
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };
    let not = Notification::new(
        "textDocument/publishDiagnostics".to_string(),
        params,
    );
    connection.sender.send(Message::Notification(not)).unwrap();
}

fn get_completions(_params: &CompletionParams) -> Vec<CompletionItem> {
    let mut items: Vec<CompletionItem> = Vec::new();
    let keywords = [
        "let", "const", "func", "if", "else", "elif", "while", "for", "in",
        "return", "true", "false", "nil", "match", "throw", "try", "catch",
        "break", "continue", "and", "or", "not", "struct", "import", "from",
        "as", "export",
    ];
    for kw in &keywords {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("keyword".to_string()),
            ..Default::default()
        });
    }
    let builtins = [
        "print", "input", "len", "str", "int", "float", "bool", "type", "range",
        "math.sqrt", "math.pow", "math.abs", "math.sin", "math.cos", "math.tan",
        "time.now", "time.unix", "time.sleep",
        "rand.int", "rand.float", "rand.choice",
        "os.name", "os.cwd", "os.args", "os.getenv",
        "fs.read", "fs.write", "fs.exists", "fs.mkdir",
    ];
    for b in &builtins {
        items.push(CompletionItem {
            label: b.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("built-in".to_string()),
            ..Default::default()
        });
    }
    items
}

fn get_hover(params: &HoverParams) -> Option<Hover> {
    let pos = &params.text_document_position_params.position;
    Some(Hover {
        contents: HoverContents::Scalar(MarkedString::String(format!(
            "Zamin language\nLine {}, column {}",
            pos.line + 1,
            pos.character
        ))),
        range: None,
    })
}
