mod ollama;
use ollama::{Message, Ollama};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let llm = Ollama::new("qwen2.5:7b");
    let messages = vec![Message::user("greet user in exactly 2 word")];
    let reply = llm.chat(&messages, None).await?;
    println!("{}", reply.content);
    Ok(())
}
