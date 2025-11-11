# OtterLang VSCode Extension

Language support for OtterLang in Visual Studio Code.

## Features

- **Syntax Highlighting** - Comprehensive colorized code editing
- **Language Server Protocol** - Full LSP support:
  - **Rich Diagnostics** - Enhanced error messages with:
    - Precise source location highlighting
    - Contextual suggestions for common mistakes
    - Helpful explanations and guidance
    - Type error reporting with detailed information
  - Go to definition (F12)
  - Go to type definition
  - Go to implementation
  - Find all references
  - Symbol renaming
  - Workspace symbol search
  - Hover information with types
  - Code completion with imports
  - Semantic highlighting
  - Inlay hints for types
  - Code actions and assists
- **Commands** - Available via Command Palette (`Cmd+Shift+P`):
  - `OtterLang: Restart Language Server`
  - `OtterLang: Start Language Server`
  - `OtterLang: Stop Language Server`
  - `OtterLang: Toggle LSP Logs`
  - `OtterLang: Show Output`

## Installation

1. **Build the LSP server:**
   ```bash
   cargo build --release --bin otterlang-lsp
   ```

2. **Package the extension:**
   ```bash
   cd vscode-extension
   npx @vscode/vsce package --allow-missing-repository
   ```

3. **Install in VSCode:**
   ```bash
   code --install-extension otterlang-0.1.0.vsix
   ```
   
   Or via VSCode UI: `Cmd+Shift+X` → `...` → "Install from VSIX..." → select `otterlang-0.1.0.vsix`

### Configuration

**LSP Server Path:**
If the LSP server isn't in your PATH, configure it in VSCode settings:

1. Press `Cmd+,` (Settings)
2. Search for `otterlang.lsp.serverPath`
3. Set to the full path: `/path/to/otterlang/target/release/otterlang-lsp`

Or add to `settings.json`:
```json
{
  "otterlang.lsp.serverPath": "/path/to/otterlang/target/release/otterlang-lsp",
  "otterlang.lsp.trace": "off"
}
```

**LSP Trace Level:**
- `off` - No logging (default)
- `messages` - Log LSP messages
- `verbose` - Full verbose logging

Toggle logs via Command Palette: `OtterLang: Toggle LSP Logs`

## Development

```bash
npm install
npm run compile
```

Press `F5` in VSCode to launch Extension Development Host.
