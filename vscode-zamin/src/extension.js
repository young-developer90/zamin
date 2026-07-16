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
    const ext = process.platform === 'win32' ? '.exe' : '';

    const candidates = [
        'zamin-lsp',
        path.join(wsDir, 'target', 'debug', `zamin-lsp${ext}`),
        path.join(wsDir, 'target', 'release', `zamin-lsp${ext}`),
        path.join(extDir, `zamin-lsp${ext}`),
        path.join(extDir, '..', 'target', 'debug', `zamin-lsp${ext}`),
        path.join(extDir, '..', 'target', 'release', `zamin-lsp${ext}`),
    ];

    let serverModule = null;
    for (const c of candidates) {
        if (c === 'zamin-lsp') {
            try {
                cp.execSync('zamin-lsp --version', { stdio: 'ignore' });
                serverModule = c;
                break;
            } catch (_) {}
        } else if (fs.existsSync(c)) {
            serverModule = c;
            break;
        }
    }

    if (!serverModule) {
        vscode.window.showErrorMessage('zamin-lsp not found. Build it with: cargo build --bin zamin-lsp');
        return;
    }

    const serverOptions = {
        run: { command: serverModule, transport: lc.TransportKind.stdio },
        debug: { command: serverModule, transport: lc.TransportKind.stdio }
    };

    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'zamin' }],
        diagnosticCollectionName: 'zamin'
    };

    client = new lc.LanguageClient('zamin-lsp', 'Zamin Language Server', serverOptions, clientOptions);
    client.start();
}

function deactivate() {
    if (client) return client.stop();
}

module.exports = { activate, deactivate };
