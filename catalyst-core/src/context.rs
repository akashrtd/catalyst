use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Instant, SystemTime};
use tiktoken_rs::CoreBPE;

static TOKENIZER: OnceLock<CoreBPE> = OnceLock::new();

fn get_tokenizer() -> &'static CoreBPE {
    TOKENIZER.get_or_init(|| tiktoken_rs::cl100k_base().unwrap())
}

pub struct TokenCounter;

impl TokenCounter {
    pub fn count(text: &str) -> usize {
        get_tokenizer().encode_with_special_tokens(text).len()
    }

    pub fn count_messages(messages: &[catalyst_llm::Message]) -> usize {
        let mut total = 0;
        for msg in messages {
            total += Self::count(&format!("{:?}: ", msg.role));
            match &msg.content {
                catalyst_llm::Content::Text(t) => total += Self::count(t),
                catalyst_llm::Content::Blocks(blocks) => {
                    for block in blocks {
                        match block {
                            catalyst_llm::ContentBlock::Text { text } => total += Self::count(text),
                            catalyst_llm::ContentBlock::ToolUse { name, input, .. } => {
                                total += Self::count(&format!("tool_use({}): ", name));
                                total += Self::count(&input.to_string());
                            }
                            catalyst_llm::ContentBlock::ToolResult { content, .. } => {
                                total += Self::count(content);
                            }
                            _ => {}
                        }
                    }
                }
            }
            total += 4;
        }
        total
    }
}

pub struct TokenBudget {
    pub system_prompt: usize,
    pub tool_definitions: usize,
    pub working_memory: usize,
    pub tool_results: usize,
    pub archive: usize,
    pub reserve_output: usize,
    pub model_limit: usize,
}

impl TokenBudget {
    pub fn for_model(model: &str) -> Self {
        let model_limit = if model.contains("gpt-4o") {
            128_000
        } else {
            200_000
        };

        Self {
            system_prompt: 0,
            tool_definitions: 0,
            working_memory: 0,
            tool_results: 0,
            archive: 0,
            reserve_output: model_limit / 10,
            model_limit,
        }
    }

    pub fn total_used(&self) -> usize {
        self.system_prompt
            + self.tool_definitions
            + self.working_memory
            + self.tool_results
            + self.archive
    }

    pub fn available(&self) -> usize {
        self.model_limit.saturating_sub(self.reserve_output) - self.total_used()
    }

    pub fn would_overflow(&self, additional_tokens: usize) -> bool {
        self.total_used() + additional_tokens > self.model_limit - self.reserve_output
    }
}

pub struct CachedFile {
    pub content: String,
    pub mtime: SystemTime,
    pub token_count: usize,
    pub referenced_at: Instant,
}

pub struct FileCache {
    entries: HashMap<PathBuf, CachedFile>,
    max_entries: usize,
}

impl FileCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
        }
    }

    pub fn get(&mut self, path: &Path) -> Option<&CachedFile> {
        let entry = self.entries.get_mut(path)?;
        entry.referenced_at = Instant::now();
        Some(entry)
    }

    pub fn insert(&mut self, path: &Path, content: String, mtime: SystemTime) {
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(path) {
            self.evict_oldest();
        }

        let token_count = TokenCounter::count(&content);
        self.entries.insert(
            path.to_path_buf(),
            CachedFile {
                content,
                mtime,
                token_count,
                referenced_at: Instant::now(),
            },
        );
    }

    pub fn is_valid(&self, path: &Path, current_mtime: SystemTime) -> bool {
        match self.entries.get(path) {
            Some(entry) => entry.mtime == current_mtime,
            None => false,
        }
    }

    pub fn invalidate(&mut self, path: &Path) {
        self.entries.remove(path);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .entries
            .iter()
            .min_by_key(|(_, v)| v.referenced_at)
            .map(|(k, _)| k.clone())
        {
            self.entries.remove(&oldest_key);
        }
    }
}

pub struct Summary {
    pub topic: String,
    pub actions: Vec<String>,
    pub outcomes: Vec<String>,
    pub token_count: usize,
}

impl Summary {
    pub fn to_text(&self) -> String {
        let mut parts = vec![format!("Topic: {}", self.topic)];
        if !self.actions.is_empty() {
            parts.push(format!("Actions: {}", self.actions.join("; ")));
        }
        if !self.outcomes.is_empty() {
            parts.push(format!("Outcomes: {}", self.outcomes.join("; ")));
        }
        parts.join("\n")
    }
}

pub struct ContextEngine {
    max_context: usize,
    _working_window_size: usize,
    file_cache: FileCache,
    archive: Vec<Summary>,
}

impl ContextEngine {
    pub fn new(model: &str) -> Self {
        let max_context = if model.contains("gpt-4o") {
            128_000
        } else {
            200_000
        };

        Self {
            max_context,
            _working_window_size: max_context * 3 / 5,
            file_cache: FileCache::new(50),
            archive: Vec::new(),
        }
    }

    pub fn build_messages(
        &self,
        messages: &[catalyst_llm::Message],
        system_prompt: &str,
    ) -> Vec<catalyst_llm::Message> {
        let reserve_output = self.max_context / 10;
        let system_tokens = TokenCounter::count(system_prompt);
        let budget = self.max_context - reserve_output - system_tokens;

        if messages.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut tokens_used = 0;

        // Walk messages from newest to oldest, fitting within budget
        for msg in messages.iter().rev() {
            let msg_tokens = TokenCounter::count_messages(std::slice::from_ref(msg));
            if tokens_used + msg_tokens > budget {
                break;
            }
            tokens_used += msg_tokens;
            result.push(msg.clone());
        }

        // Reverse to restore chronological order
        result.reverse();

        self.truncate_tool_results(result, budget.saturating_sub(tokens_used))
    }

    pub fn would_overflow(
        &self,
        current_messages: &[catalyst_llm::Message],
        additional_tokens: usize,
    ) -> bool {
        let reserve_output = self.max_context / 10;
        let current = TokenCounter::count_messages(current_messages);
        current + additional_tokens > self.max_context - reserve_output
    }

    pub fn truncate_output(&self, output: &str, budget: usize) -> String {
        if output.len() <= budget {
            return output.to_string();
        }

        let head_chars = budget / 3;
        let tail_chars = budget / 3;

        let head: String = output.chars().take(head_chars).collect();
        let tail: String = output
            .chars()
            .rev()
            .take(tail_chars)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        let omitted = output.len() - head_chars - tail_chars;

        format!(
            "{}\n\n... [truncated: {} characters omitted] ...\n\n{}",
            head, omitted, tail
        )
    }

    fn truncate_tool_results(
        &self,
        messages: Vec<catalyst_llm::Message>,
        budget: usize,
    ) -> Vec<catalyst_llm::Message> {
        if budget > 0 {
            return messages;
        }

        messages
            .into_iter()
            .map(|msg| match &msg.content {
                catalyst_llm::Content::Blocks(blocks) => {
                    let truncated_blocks: Vec<catalyst_llm::ContentBlock> = blocks
                        .iter()
                        .map(|block| match block {
                            catalyst_llm::ContentBlock::ToolResult {
                                tool_use_id,
                                content,
                                is_error,
                            } => {
                                let max_tool_output = 5000;
                                if content.len() > max_tool_output {
                                    catalyst_llm::ContentBlock::ToolResult {
                                        tool_use_id: tool_use_id.clone(),
                                        content: self.truncate_output(content, max_tool_output),
                                        is_error: *is_error,
                                    }
                                } else {
                                    block.clone()
                                }
                            }
                            _ => block.clone(),
                        })
                        .collect();
                    catalyst_llm::Message {
                        role: msg.role,
                        content: catalyst_llm::Content::Blocks(truncated_blocks),
                    }
                }
                _ => msg,
            })
            .collect()
    }

    pub fn add_summary(&mut self, topic: String, actions: Vec<String>, outcomes: Vec<String>) {
        let text = format!(
            "Topic: {}\nActions: {}\nOutcomes: {}",
            topic,
            actions.join("; "),
            outcomes.join("; ")
        );
        let token_count = TokenCounter::count(&text);
        self.archive.push(Summary {
            topic,
            actions,
            outcomes,
            token_count,
        });
    }

    pub fn archive_summaries(&self) -> &[Summary] {
        &self.archive
    }

    pub fn archive_token_count(&self) -> usize {
        self.archive.iter().map(|s| s.token_count).sum()
    }

    pub fn file_cache(&mut self) -> &mut FileCache {
        &mut self.file_cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counter_counts_tokens() {
        let count = TokenCounter::count("Hello, world!");
        assert!(count > 0);
        assert!(count < 10);
    }

    #[test]
    fn test_token_counter_empty_string() {
        assert_eq!(TokenCounter::count(""), 0);
    }

    #[test]
    fn test_token_counter_code() {
        let code = "fn main() { println!(\"hello\"); }";
        let count = TokenCounter::count(code);
        assert!(count > 0);
    }

    #[test]
    fn test_token_counter_large_text() {
        let text = "word ".repeat(1000);
        let count = TokenCounter::count(&text);
        assert!(count > 500);
    }

    #[test]
    fn test_token_budget_default_model() {
        let budget = TokenBudget::for_model("claude-sonnet-4-20250514");
        assert_eq!(budget.model_limit, 200_000);
        assert_eq!(budget.reserve_output, 20_000);
    }

    #[test]
    fn test_token_budget_gpt4o() {
        let budget = TokenBudget::for_model("openai/gpt-4o");
        assert_eq!(budget.model_limit, 128_000);
    }

    #[test]
    fn test_token_budget_available() {
        let mut budget = TokenBudget::for_model("claude-sonnet-4-20250514");
        budget.system_prompt = 500;
        budget.working_memory = 1000;
        let available = budget.available();
        assert_eq!(available, 200_000 - 20_000 - 500 - 1000);
    }

    #[test]
    fn test_token_budget_would_overflow() {
        let mut budget = TokenBudget::for_model("claude-sonnet-4-20250514");
        budget.system_prompt = 170_000;
        assert!(budget.would_overflow(20_000));
        assert!(!budget.would_overflow(1_000));
    }

    #[test]
    fn test_file_cache_insert_and_get() {
        let mut cache = FileCache::new(10);
        let path = PathBuf::from("test.rs");

        cache.insert(&path, "fn main() {}".to_string(), SystemTime::UNIX_EPOCH);

        let entry = cache.get(&path).unwrap();
        assert_eq!(entry.content, "fn main() {}");
        assert!(entry.token_count > 0);
    }

    #[test]
    fn test_file_cache_eviction() {
        let mut cache = FileCache::new(2);

        cache.insert(Path::new("a.rs"), "a".to_string(), SystemTime::UNIX_EPOCH);
        cache.insert(Path::new("b.rs"), "b".to_string(), SystemTime::UNIX_EPOCH);
        cache.insert(Path::new("c.rs"), "c".to_string(), SystemTime::UNIX_EPOCH);

        assert_eq!(cache.len(), 2);
        assert!(cache.get(Path::new("c.rs")).is_some());
    }

    #[test]
    fn test_file_cache_validity() {
        let mut cache = FileCache::new(10);
        let path = Path::new("test.rs");
        let mtime = SystemTime::UNIX_EPOCH;

        cache.insert(path, "content".to_string(), mtime);
        assert!(cache.is_valid(path, mtime));

        let later_mtime = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(60);
        assert!(!cache.is_valid(path, later_mtime));
    }

    #[test]
    fn test_file_cache_invalidate() {
        let mut cache = FileCache::new(10);
        let path = Path::new("test.rs");

        cache.insert(path, "content".to_string(), SystemTime::UNIX_EPOCH);
        cache.invalidate(path);
        assert!(cache.get(path).is_none());
    }

    #[test]
    fn test_file_cache_clear() {
        let mut cache = FileCache::new(10);
        cache.insert(Path::new("a.rs"), "a".to_string(), SystemTime::UNIX_EPOCH);
        cache.insert(Path::new("b.rs"), "b".to_string(), SystemTime::UNIX_EPOCH);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_context_engine_new() {
        let engine = ContextEngine::new("claude-sonnet-4-20250514");
        assert_eq!(engine.max_context, 200_000);
    }

    #[test]
    fn test_context_engine_build_messages_empty() {
        let engine = ContextEngine::new("claude-sonnet-4-20250514");
        let result = engine.build_messages(&[], "system prompt");
        assert!(result.is_empty());
    }

    #[test]
    fn test_context_engine_build_messages_fits() {
        let engine = ContextEngine::new("claude-sonnet-4-20250514");
        let messages = vec![
            catalyst_llm::Message {
                role: catalyst_llm::Role::User,
                content: catalyst_llm::Content::Text("Hello".to_string()),
            },
            catalyst_llm::Message {
                role: catalyst_llm::Role::Assistant,
                content: catalyst_llm::Content::Text("Hi there!".to_string()),
            },
        ];

        let result = engine.build_messages(&messages, "system");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_context_engine_build_messages_truncates_old() {
        let mut engine = ContextEngine::new("claude-sonnet-4-20250514");
        engine.max_context = 50;

        let messages: Vec<catalyst_llm::Message> = (0..20)
            .map(|i| catalyst_llm::Message {
                role: catalyst_llm::Role::User,
                content: catalyst_llm::Content::Text(format!("Message number {}", i)),
            })
            .collect();

        let result = engine.build_messages(&messages, "sys");
        assert!(result.len() < messages.len());
        match &result.last().unwrap().content {
            catalyst_llm::Content::Text(t) => assert!(t.contains("19")),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_context_engine_truncate_output() {
        let engine = ContextEngine::new("claude-sonnet-4-20250514");
        let long_output = "x".repeat(10_000);
        let truncated = engine.truncate_output(&long_output, 1000);

        assert!(truncated.len() < long_output.len());
        assert!(truncated.contains("truncated"));
    }

    #[test]
    fn test_context_engine_truncate_output_no_truncation() {
        let engine = ContextEngine::new("claude-sonnet-4-20250514");
        let short = "hello";
        let result = engine.truncate_output(short, 100);
        assert_eq!(result, short);
    }

    #[test]
    fn test_context_engine_would_overflow() {
        let engine = ContextEngine::new("claude-sonnet-4-20250514");
        let messages = vec![catalyst_llm::Message {
            role: catalyst_llm::Role::User,
            content: catalyst_llm::Content::Text("small message".to_string()),
        }];
        assert!(!engine.would_overflow(&messages, 100));
    }

    #[test]
    fn test_summary_to_text() {
        let summary = Summary {
            topic: "Fixed auth bug".to_string(),
            actions: vec!["Read auth.rs".to_string(), "Edited line 42".to_string()],
            outcomes: vec!["Tests pass".to_string()],
            token_count: 20,
        };
        let text = summary.to_text();
        assert!(text.contains("Fixed auth bug"));
        assert!(text.contains("Read auth.rs"));
        assert!(text.contains("Tests pass"));
    }

    #[test]
    fn test_context_engine_add_summary() {
        let mut engine = ContextEngine::new("claude-sonnet-4-20250514");
        engine.add_summary(
            "Refactored tools".to_string(),
            vec!["Split tools.rs".to_string()],
            vec!["7 tools working".to_string()],
        );
        assert_eq!(engine.archive_summaries().len(), 1);
        assert!(engine.archive_token_count() > 0);
    }
}
