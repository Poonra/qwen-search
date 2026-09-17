mod agent;
mod ollama;
mod search;

use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 2 && args[1] == "--search" {
        let ws = search::Websearch::new()?;
        let results = ws.search(&args[2..].join(" "), 5).await?;
        println!("{}", search::format_results(&results));
        return Ok(());
    }

    let model = std::env::var("MODEL").unwrap_or_else(|_| "qwen2.5:7b".to_string());
    let mut agent = agent::Agent::new(&model)?;
    println!("Chatting with {model}. /clear resets the chat, /exit quits.\n");
    let stdin = io::stdin();

    loop {
        print!("you>");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }

        match line.trim() {
            "" => continue,
            "/exit" => break,
            "/clear" => {
                agent.clear();
                println!("(chat cleared)\n");
                continue;
            }
            input => {
                print!("bot> ");
                io::stdout().flush()?;
                if let Err(e) = agent.ask(input).await {
                    eprintln!("\nerror: {e:#}");
                }
                println!();
            }
        }
    }
    Ok(())
}
