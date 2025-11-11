use otterlang::lsp;

#[tokio::main]
async fn main() {
    lsp::run_stdio_server().await;
}
