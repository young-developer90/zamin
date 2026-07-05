const vscode = require('vscode');
const path = require('path');
const fs = require('fs');
const cp = require('child_process');
const lc = require('vscode-languageclient');

let client = null;

function activate(ctx) {
    const extDir = ctx.extensionPath;
    const workspaceFolders = vscode.workspace.workspaceFolders;
    const wsDir = workspaceFolders ? workspaceFolders[0].uri.fsPath : path.dirname(extDir);

    const candidates = [
        'lion-lsp',
        path.join(wsDir, 'target', 'debug', 'lion-lsp.exe'),
        path.join(wsDir, 'target', 'release', 'lion-lsp.exe'),
        path.join(extDir, 'lion-lsp.exe'),
        path.join(extDir, '..', 'target', 'debug', 'lion-lsp.exe'),
    ];

    let serverModule = null;
    for (const c of candidates) {
        if (c === 'lion-lsp') {
            try {
                cp.execSync('lion-lsp --version', { stdio: 'ignore' });
                serverModule = c;
                break;
            } catch (_) {}
        } else if (fs.existsSync(c)) {
            serverModule = c;
            break;
        }
    }

    if (!serverModule) {
        vscode.window.showErrorMessage('lion-lsp not found. Build it with: cargo build --bin lion-lsp');
        return;
    }

    const serverOptions = {
        run: { command: serverModule, transport: lc.TransportKind.stdio },
        debug: { command: serverModule, transport: lc.TransportKind.stdio }
    };

    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'lion' }],
        diagnosticCollectionName: 'lion'
    };

    client = new lc.LanguageClient('lion-lsp', 'Lion Language Server', serverOptions, clientOptions);
    client.start();
}

function deactivate() {
    if (client) return client.stop();
}

module.exports = { activate, deactivate };
