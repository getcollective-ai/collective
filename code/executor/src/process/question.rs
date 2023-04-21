use futures::Stream;
use tokio_openai::ChatRequest;

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

    fn question_request(&self) -> ChatRequest {
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
    ) -> anyhow::Result<impl Stream<Item = Result<String, anyhow::Error>>> {
        let request = self.question_request();

        let stream = self.executor.ctx.ai.stream_chat(request).await?;

        Ok(stream)
    }

    pub fn answer(&mut self, answer: String) {
        self.answers.push(answer);
    }
}

#[cfg(test)]
mod tests {
    use futures::TryStreamExt;

    use crate::{process::question::QAndA, Executor};

    #[tokio::test]
    async fn test_get_question() -> anyhow::Result<()> {
        let mut q_and_a = QAndA::new(Executor::new()?, "Create a calculator");
        let question = q_and_a.gen_question().await?;

        let question: String = question.try_collect().await?;
        let question = question.trim().to_lowercase();

        // question will most likely contain one of these keywords
        let keywords = &[
            "basic",
            "scientific",
            "language",
            "gui",
            "graphical user interface",
            "command line",
            "cli",
            "command-line interface",
            "web",
            "web-based",
            "web-based interface",
            "web interface",
            "web-based",
            "math",
            "calculator",
            "function",
        ];
        let contains_any = keywords.iter().any(|keyword| question.contains(keyword));

        assert!(
            contains_any,
            "question: {question} does not contain any mentioned keywords"
        );

        Ok(())
    }
}
