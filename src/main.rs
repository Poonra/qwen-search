mod ollama;
mod search;
use ollama::{Message, Ollama};
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    
    let args: Vec<String> =std::env::args().collect();
    if args.len() > 2 && args[1] == "--search" {
        let ws =search::Websearch::new()?;
        let results = ws.search(&args[2..].join(" "),5).await?;
        println!("{}", search::format_results(&results));
        return Ok(());
    }
    
    let llm = Ollama::new("qwen2.5:7b");
    let mut history = vec![Message::system("You are a helpful assistant")];
    let stdin = io::stdin();

    loop{
        print!("you>");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "/exit"{
            break;
        }

        history.push(Message::user(input));
        let reply = llm.chat(&history, None).await?;
        println!("bot> {}\n", reply.content);
        history.push(reply);
    }
    Ok(())
}
