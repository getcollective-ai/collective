use once_cell::sync::Lazy;
use protocol::client;
use regex::Regex;
use tokio_openai::ChatRequest;
use tracing::info;

use crate::Executor;

pub struct QAndA {
    executor: Executor,
    instruction: String,
    questions: Vec<String>,
    answers: Vec<String>,
}

impl QAndA {
    pub fn new(executor: Executor, instruction: impl Into<String>) -> Self {
        Self {
            questions: vec![],
            answers: vec![],
            instruction: instruction.into(),
            executor,
        }
    }

    fn plan_request(&self) -> ChatRequest {
        let mut message = String::new();

        message.push_str(&format!("Instruction: {}\n\n", self.instruction));

        for (question, answer) in self.questions.iter().zip(self.answers.iter()) {
            message.push_str(&format!("Q: {}\nA: {}\n\n", question, answer));
        }

        message.push_str("---\n\nIntricate Plan:\n");

        ChatRequest::new()
            .sys_msg(
                "Plan how to complete the instruction. List one step per line and include \
                 in-depth explanation on how you think you can best complete the task.",
            )
            .user_msg(message)
    }

    pub async fn plan(&mut self) -> anyhow::Result<String> {
        let request = self.plan_request();

        info!("Getting plan from OpenAI...");
        let answer = self.executor.ctx.ai.chat(request).await?;
        info!("Plan:\n{}", answer);

        Ok(answer)
    }

    fn chat_request(&self) -> ChatRequest {
        let mut message = String::new();

        message.push_str(&format!("Instruction: {}\n\n", self.instruction));

        for (question, answer) in self.questions.iter().zip(self.answers.iter()) {
            message.push_str(&format!("Q: {}\nA: {}\n\n", question, answer));
        }

        message.push_str("Q: ");

        ChatRequest::new()
            .stop_at("\n")
            .sys_msg(
                "list relevant questions that are important for completing the task. One per \
                 line. Only include the raw question text. Do not include any other text. Also \
                 ask questions to correct any mistakes or misunderstandings. The user might have",
            )
            .user_msg(message)
    }

    pub async fn gen_question(&mut self) -> anyhow::Result<String> {
        let request = self.chat_request();

        let question = self.executor.ctx.ai.chat(request).await?;
        let question = trim_question(&question).trim().to_string();

        self.questions.push(question.clone());

        Ok(question)
    }

    pub fn answer(&mut self, answer: String) {
        self.answers.push(answer);
    }
}
async fn get_question(
    executor: Executor,
    instruction: client::Instruction,
) -> anyhow::Result<String> {
    let instruction = String::from(instruction);

    let request = ChatRequest::new()
        .sys_msg(
            "list relevant questions that are important for completing the task. One per line. \
             Only include the raw question text. Do not include any other text. Also ask \
             questions to correct any mistakes or misunderstandings. The user might have",
        )
        .user_msg(instruction)
        .assistant_msg(
            "What specific programming languages or platforms should be used for creating the \
             program?",
        )
        .user_msg("Rust")
        .stop_at("\n");

    let res = executor.ctx.ai.chat(request).await?;

    let res = trim_question(&res).trim().to_string();

    Ok(res)
}

fn trim_question(question: &str) -> &str {
    let mut question = question.trim();

    // trim number or bullet at start of question using regex

    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d+\.?").unwrap());

    if let Some(caps) = RE.captures(question) {
        question = &question[caps.get(0).unwrap().end()..];
    }

    question
}

#[cfg(test)]
mod tests {
    use protocol::client;

    use crate::{process::question::get_question, Executor};

    #[tokio::test]
    async fn test_get_question() -> anyhow::Result<()> {
        let exec = Executor::new()?;

        let instruction = client::Instruction {
            instruction: "Create a program that real time does voice translation between Chinese \
                          and English and English and Chinese"
                .to_string(),
        };

        let question = get_question(exec, instruction).await?;

        println!("{}", question);

        Ok(())
    }
}
