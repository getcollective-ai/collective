use futures::{executor::block_on_stream, select, Stream, StreamExt};
use once_cell::sync::Lazy;
use protocol::client;
use regex::Regex;
use tokio::sync::mpsc;
use tokio_openai::ChatRequest;
use tracing::{error, info};

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

    pub fn add_question(&mut self, question: String) {
        self.questions.push(question);
    }

    fn plan_request(&self) -> ChatRequest {
        let mut message = String::new();

        message.push_str(&format!("Instruction: {}\n\n", self.instruction));

        for (question, answer) in self.questions.iter().zip(self.answers.iter()) {
            message.push_str(&format!("Q: {question}\nA: {answer}\n\n"));
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
        let answer = match self.executor.ctx.ai.chat(request).await {
            Ok(answer) => answer,
            Err(err) => {
                error!("Error generating plan: {}", err);
                return Ok(
                    "Error generating plan. Check logs. (does your API KEY support GPT4?)"
                        .to_string(),
                );
            }
        };

        info!("Plan:\n{}", answer);

        Ok(answer)
    }

    fn chat_request(&self) -> ChatRequest {
        let mut message = String::new();

        message.push_str(&format!("Instruction: {}\n\n", self.instruction));

        for (question, answer) in self.questions.iter().zip(self.answers.iter()) {
            message.push_str(&format!("Q: {question}\nA: {answer}\n\n"));
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

    pub async fn gen_question(
        &mut self,
    ) -> impl Stream<Item = Result<std::string::String, anyhow::Error>> {
        let request = self.chat_request();

        self.executor.ctx.ai.stream_chat(request).await.unwrap()
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
    use crate::{process::question::QAndA, Executor};

    #[tokio::test]
    async fn test_plan() -> anyhow::Result<()> {
        let mut q_and_a = QAndA {
            executor: Executor::new()?,
            instruction: "Create a simple CLI calculator in 3 steps".to_string(),
            questions: vec!["Which language should I use?".to_string()],
            answers: vec!["Rust".to_string()],
        };

        let plan = q_and_a.plan().await?;
        let plan = plan.trim().to_lowercase();

        assert!(
            plan.contains("rust"),
            "{plan} does not contain the keyword 'rust'"
        );

        println!("plan:\n{}", plan);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_question() -> anyhow::Result<()> {
        let mut q_and_a = QAndA::new(Executor::new()?, "Create a calculator");
        let question = q_and_a.gen_question().await;

        // TODO: make this test work again

        // let question = question.trim().to_lowercase();

        // question will most likely contain one of these keywords
        // let keywords = &[
        //     "basic",
        //     "scientific",
        //     "language",
        //     "gui",
        //     "graphical user interface",
        //     "command line",
        //     "cli",
        //     "command-line interface",
        //     "web",
        //     "web-based",
        //     "web-based interface",
        //     "web interface",
        //     "web-based",
        //     "math",
        //     "calculator",
        //     "function",
        // ];
        //
        // let contains_any = keywords.iter().any(|keyword| question.contains(keyword));

        // assert!(
        //     contains_any,
        //     "question: {question} does not contain any mentioned keywords"
        // );

        Ok(())
    }
}
