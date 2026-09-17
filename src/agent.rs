use crate::net;
use crate::ollama::{Message, Ollama, ToolCall};
use crate::search::{self, Websearch};
use anyhow::Result;
use serde_json::{Value, json};
use std::io::{self, Write};
const MAX_TOOL_ROUNDS: usize = 3;

pub struct Agent {
    llm: Ollama,
    search: Websearch,
    tools: Value,
    history: Vec<Message>,
}

impl Agent {
    pub fn new(model: &str) -> Result<Self> {
        let today = chrono::Local::now().format("%Y-%m-%d");
        let system = format!(
            "You are a helpful assistant running locally. Today's date is {today}. \
               If a web_search tool is available, use it for recent or time-sensitive facts \
               (news, prices, software versions, events) and cite the URLs you used. \
               If no tool is available, answer from your own knowledge and mention that \
               it may be out of date when the question is time-sensitive.  \n\
             Take a position. When asked what is better, faster, or the right choice, \
             pick one and say which, in the first sentence. Give your two strongest \
             reasons. Mention the main trade-off in one line at most. \
             Never answer 'it depends' without immediately saying what you would pick \
             and under what condition you would switch. Do not list pros and cons of \
             every option and leave the choice to the user. \
             If the evidence is genuinely thin, still name your best guess and say \
             how confident you are.
               \n\
               Style: answer the question directly in the first sentence, then stop. \
               No preamble, no restating the question, no summary at the end. \
               Do not say 'Certainly', 'Great question', or 'I hope this helps'. \
               Use short sentences. Prefer a bullet list over a paragraph when there is \
               more than one item. Only give background or caveats if they change the answer. \
               If you are unsure, say so in one line instead of hedging throughout. If you are not sure about \
             the answer immediately search for the right answer"
        );
        Ok(Self {
            llm: Ollama::new(model),
            search: Websearch::new()?,
            tools: tool_schema(),
            history: vec![Message::system(system)],
        })
    }

    pub fn clear(&mut self) {
        self.history.truncate(1);
    }

    pub async fn ask(&mut self, input: &str) -> Result<()> {
        let online = net::is_online().await;
        if !online {
            eprintln!("[offline - no web search] thinking!");
        }

        self.history.push(Message::user(input));
        // always search first, whether the model asked for it or not\
        if online {
            eprintln!("[searching: {input}]");

            let results = match self.search.search(input, 5).await {
                Ok(r) => search::format_results(&r),
                Err(e) => format!("Search failed ({e}). Answer from your own knowledge."),
            };
            self.history.push(Message::tool("web_search", results));
        }
        for round in 0..=MAX_TOOL_ROUNDS {
            let tools = (online && round < MAX_TOOL_ROUNDS).then_some(&self.tools);
            let reply = self
                .llm
                .chat_stream(&self.history, tools, |token| {
                    print!("{token}");
                    let _ = io::stdout().flush();
                })
                .await?;

            let calls = reply.tool_calls.clone();
            self.history.push(reply);
            if calls.is_empty() {
                println!();
                return Ok(());
            }
            for call in &calls {
                let result = self.run_tool(call).await;
                self.history
                    .push(Message::tool(&call.function.name, result));
            }
        }
        Ok(())
    }

    async fn run_tool(&self, call: &ToolCall) -> String {
        match call.function.name.as_str() {
            "web_search" => {
                let Some(query) = call.function.arguments.get("query").and_then(Value::as_str)
                else {
                    return "Error: missing 'query' argument".into();
                };
                eprintln!("[searching: {query}]");
                match self.search.search(query, 5).await {
                    Ok(results) => search::format_results(&results),
                    Err(e) => format!(
                        "Search failed ({e}). The internet may be unavailable. Answer from your \
                         own knowledge and tell the user the information may be out of date."
                    ),
                }
            }
            other => format!("Error: unknown tool '{other}'"),
        }
    }
}

fn tool_schema() -> Value {
    json!([{
        "type": "function",
        "function": {
            "name": "web_search",
            "description": "Search the internet for current or recent information: news, prices, \
                            software versions, events after your training data, anything time-sensitive. \
                            Do not use it for general knowledge you already know.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "A short, specific search query" }
                },
                "required": ["query"]
            }
        }
    }])
}
