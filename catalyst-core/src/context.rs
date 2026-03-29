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
    working_window_size: usize,
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
            working_window_size: max_context * 3 / 5,
            file_cache: FileCache::new(50),
            archive: Vec::new(),
        }
    }

    pub fn working_window_size(&self) -> usize {
        self.working_window_size
    }

    pub fn set_working_window_size(&mut self, size: usize) {
        self.working_window_size = size;
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

        if !self.archive.is_empty() {
            let archive_text: String = self
                .archive
                .iter()
                .map(|s| s.to_text())
                .collect::<Vec<_>>()
                .join("\n\n");
            let archive_tokens = TokenCounter::count(&archive_text);
            if archive_tokens < budget {
                tokens_used += archive_tokens;
                result.push(catalyst_llm::Message {
                    role: catalyst_llm::Role::User,
                    content: catalyst_llm::Content::Text(format!(
                        "[Conversation Summary]\n{}",
                        archive_text
                    )),
                });
                result.push(catalyst_llm::Message {
                    role: catalyst_llm::Role::Assistant,
                    content: catalyst_llm::Content::Text(
                        "Understood, I have the conversation context.".to_string(),
                    ),
                });
            }
        }

        for msg in messages.iter().rev() {
            let msg_tokens = TokenCounter::count_messages(std::slice::from_ref(msg));
            if tokens_used + msg_tokens > budget {
                break;
            }
            tokens_used += msg_tokens;
            result.push(msg.clone());
        }

        result.reverse();

        self.truncate_tool_results(result, budget.saturating_sub(tokens_used))
    }

    pub fn summarize_messages(
        &mut self,
        messages: &[catalyst_llm::Message],
        keep_recent: usize,
    ) -> Vec<catalyst_llm::Message> {
        if messages.len() <= keep_recent {
            return messages.to_vec();
        }

        let split_point = messages.len() - keep_recent;
        let older = &messages[..split_point];

        let mut topics = Vec::new();
        let mut actions = Vec::new();
        let mut outcomes = Vec::new();

        for msg in older {
            match &msg.content {
                catalyst_llm::Content::Text(t) => {
                    let preview: String = t.chars().take(100).collect();
                    match msg.role {
                        catalyst_llm::Role::User => {
                            if !preview.is_empty() {
                                topics.push(preview);
                            }
                        }
                        catalyst_llm::Role::Assistant => {
                            if !preview.is_empty() {
                                outcomes.push(preview);
                            }
                        }
                    }
                }
                catalyst_llm::Content::Blocks(blocks) => {
                    for block in blocks {
                        match block {
                            catalyst_llm::ContentBlock::ToolUse { name, .. } => {
                                actions.push(format!("used {}", name));
                            }
                            catalyst_llm::ContentBlock::ToolResult {
                                is_error: true,
                                content,
                                ..
                            } => {
                                outcomes.push(format!(
                                    "error: {}",
                                    content.chars().take(50).collect::<String>()
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let topic = topics
            .first()
            .cloned()
            .unwrap_or_else(|| "general discussion".to_string());
        self.archive.push(Summary {
            topic,
            actions: actions.into_iter().take(5).collect(),
            outcomes: outcomes.into_iter().take(5).collect(),
            token_count: 0,
        });

        if let Some(last) = self.archive.last_mut() {
            last.token_count = TokenCounter::count(&last.to_text());
        }

        messages[split_point..].to_vec()
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

    pub fn max_context(&self) -> usize {
        self.max_context
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

    #[test]
    fn test_context_engine_gpt4o_limit() {
        let engine = ContextEngine::new("openai/gpt-4o");
        assert_eq!(engine.max_context(), 128_000);
    }

    #[test]
    fn test_token_counter_multibyte() {
        let text = "こんにちは世界";
        let count = TokenCounter::count(text);
        assert!(count > 0);
    }

    #[test]
    fn test_token_counter_json() {
        let json = r#"{"key": "value", "number": 42, "array": [1, 2, 3]}"#;
        let count = TokenCounter::count(json);
        assert!(count > 5);
    }

    #[test]
    fn test_token_counter_single_char() {
        assert_eq!(TokenCounter::count("a"), 1);
    }

    #[test]
    fn test_token_counter_messages_with_text() {
        let messages = vec![catalyst_llm::Message {
            role: catalyst_llm::Role::User,
            content: catalyst_llm::Content::Text("Hello, how are you?".to_string()),
        }];
        let count = TokenCounter::count_messages(&messages);
        assert!(count > 0);
        assert!(count < 20);
    }

    #[test]
    fn test_token_counter_messages_with_blocks() {
        let messages = vec![catalyst_llm::Message {
            role: catalyst_llm::Role::Assistant,
            content: catalyst_llm::Content::Blocks(vec![
                catalyst_llm::ContentBlock::Text {
                    text: "Reading file...".to_string(),
                },
                catalyst_llm::ContentBlock::ToolUse {
                    id: "tool_1".to_string(),
                    name: "read".to_string(),
                    input: serde_json::json!({"path": "/test.rs"}),
                },
                catalyst_llm::ContentBlock::ToolResult {
                    tool_use_id: "tool_1".to_string(),
                    content: "file contents here".to_string(),
                    is_error: false,
                },
            ]),
        }];
        let count = TokenCounter::count_messages(&messages);
        assert!(count > 0);
    }

    #[test]
    fn test_token_counter_multiple_messages() {
        let messages = vec![
            catalyst_llm::Message {
                role: catalyst_llm::Role::User,
                content: catalyst_llm::Content::Text("Hello".to_string()),
            },
            catalyst_llm::Message {
                role: catalyst_llm::Role::Assistant,
                content: catalyst_llm::Content::Text("Hi there!".to_string()),
            },
            catalyst_llm::Message {
                role: catalyst_llm::Role::User,
                content: catalyst_llm::Content::Text("How are you?".to_string()),
            },
        ];
        let count = TokenCounter::count_messages(&messages);
        assert!(count > 5);
    }

    #[test]
    fn test_token_budget_total_used() {
        let mut budget = TokenBudget::for_model("claude-sonnet-4-20250514");
        budget.system_prompt = 100;
        budget.tool_definitions = 200;
        budget.working_memory = 300;
        budget.tool_results = 400;
        budget.archive = 500;
        assert_eq!(budget.total_used(), 1500);
    }

    #[test]
    fn test_token_budget_available_after_use() {
        let mut budget = TokenBudget::for_model("claude-sonnet-4-20250514");
        budget.system_prompt = 10000;
        budget.tool_definitions = 5000;
        let available = budget.available();
        assert_eq!(available, 200_000 - 20_000 - 10_000 - 5_000);
    }

    #[test]
    fn test_file_cache_update_existing() {
        let mut cache = FileCache::new(10);
        let path = Path::new("test.rs");

        cache.insert(path, "v1".to_string(), SystemTime::UNIX_EPOCH);
        assert_eq!(cache.get(path).unwrap().content, "v1");

        cache.insert(path, "v2".to_string(), SystemTime::UNIX_EPOCH);
        assert_eq!(cache.get(path).unwrap().content, "v2");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_file_cache_lru_ordering() {
        let mut cache = FileCache::new(3);

        cache.insert(Path::new("a.rs"), "a".to_string(), SystemTime::UNIX_EPOCH);
        std::thread::sleep(std::time::Duration::from_millis(1));
        cache.insert(Path::new("b.rs"), "b".to_string(), SystemTime::UNIX_EPOCH);
        std::thread::sleep(std::time::Duration::from_millis(1));
        cache.insert(Path::new("c.rs"), "c".to_string(), SystemTime::UNIX_EPOCH);

        assert!(cache.get(Path::new("a.rs")).is_some());

        std::thread::sleep(std::time::Duration::from_millis(1));
        cache.insert(Path::new("d.rs"), "d".to_string(), SystemTime::UNIX_EPOCH);

        assert_eq!(cache.len(), 3);
        assert!(cache.get(Path::new("a.rs")).is_some());
        assert!(cache.get(Path::new("d.rs")).is_some());
    }

    #[test]
    fn test_file_cache_token_count_tracking() {
        let mut cache = FileCache::new(10);
        let content = "fn main() { println!(\"hello\"); }";
        cache.insert(
            Path::new("test.rs"),
            content.to_string(),
            SystemTime::UNIX_EPOCH,
        );

        let entry = cache.get(Path::new("test.rs")).unwrap();
        assert!(entry.token_count > 0);
        assert_eq!(entry.token_count, TokenCounter::count(content));
    }

    #[test]
    fn test_context_engine_build_messages_preserves_order() {
        let engine = ContextEngine::new("claude-sonnet-4-20250514");
        let messages: Vec<catalyst_llm::Message> = (0..5)
            .map(|i| catalyst_llm::Message {
                role: catalyst_llm::Role::User,
                content: catalyst_llm::Content::Text(format!("Message {}", i)),
            })
            .collect();

        let result = engine.build_messages(&messages, "system");
        assert_eq!(result.len(), 5);

        match &result[0].content {
            catalyst_llm::Content::Text(t) => assert!(t.contains("Message 0")),
            _ => panic!("Expected text"),
        }
        match &result[4].content {
            catalyst_llm::Content::Text(t) => assert!(t.contains("Message 4")),
            _ => panic!("Expected text"),
        }
    }

    #[test]
    fn test_context_engine_build_messages_single_message() {
        let engine = ContextEngine::new("claude-sonnet-4-20250514");
        let messages = vec![catalyst_llm::Message {
            role: catalyst_llm::Role::User,
            content: catalyst_llm::Content::Text("Hello".to_string()),
        }];

        let result = engine.build_messages(&messages, "system");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_context_engine_build_messages_keeps_recent() {
        let mut engine = ContextEngine::new("claude-sonnet-4-20250514");
        engine.max_context = 30;

        let messages: Vec<catalyst_llm::Message> = (0..10)
            .map(|i| catalyst_llm::Message {
                role: catalyst_llm::Role::User,
                content: catalyst_llm::Content::Text(format!("Msg {}", i)),
            })
            .collect();

        let result = engine.build_messages(&messages, "s");
        assert!(result.len() < 10);

        if let Some(last) = result.last() {
            match &last.content {
                catalyst_llm::Content::Text(t) => assert!(t.contains("9")),
                _ => panic!("Expected text"),
            }
        }
    }

    #[test]
    fn test_context_engine_truncate_output_head_tail() {
        let engine = ContextEngine::new("claude-sonnet-4-20250514");
        let output = "abcdefghij".repeat(100);
        let truncated = engine.truncate_output(&output, 300);

        assert!(truncated.starts_with("abcdefghij"));
        assert!(truncated.ends_with("abcdefghij"));
        assert!(truncated.contains("truncated"));
        assert!(truncated.len() < output.len());
    }

    #[test]
    fn test_context_engine_truncate_output_exact_fit() {
        let engine = ContextEngine::new("claude-sonnet-4-20250514");
        let output = "x".repeat(50);
        let result = engine.truncate_output(&output, 100);
        assert_eq!(result, output);
    }

    #[test]
    fn test_context_engine_would_overflow_large() {
        let mut engine = ContextEngine::new("claude-sonnet-4-20250514");
        engine.max_context = 1000;

        let large_messages: Vec<catalyst_llm::Message> = (0..50)
            .map(|_| catalyst_llm::Message {
                role: catalyst_llm::Role::User,
                content: catalyst_llm::Content::Text("x ".repeat(500)),
            })
            .collect();
        assert!(engine.would_overflow(&large_messages, 0));
    }

    #[test]
    fn test_context_engine_would_overflow_empty() {
        let engine = ContextEngine::new("claude-sonnet-4-20250514");
        assert!(!engine.would_overflow(&[], 100));
    }

    #[test]
    fn test_multiple_summaries() {
        let mut engine = ContextEngine::new("claude-sonnet-4-20250514");
        engine.add_summary("Topic 1".to_string(), vec!["Action 1".to_string()], vec![]);
        engine.add_summary("Topic 2".to_string(), vec!["Action 2".to_string()], vec![]);
        engine.add_summary("Topic 3".to_string(), vec![], vec!["Outcome 3".to_string()]);

        assert_eq!(engine.archive_summaries().len(), 3);
        assert!(engine.archive_token_count() > 0);
        assert_eq!(engine.archive_summaries()[0].topic, "Topic 1");
        assert_eq!(engine.archive_summaries()[2].outcomes.len(), 1);
    }

    #[test]
    fn test_summary_empty_actions() {
        let summary = Summary {
            topic: "Test".to_string(),
            actions: vec![],
            outcomes: vec![],
            token_count: 0,
        };
        let text = summary.to_text();
        assert!(text.contains("Test"));
        assert!(!text.contains("Actions"));
        assert!(!text.contains("Outcomes"));
    }

    #[test]
    fn test_context_engine_file_cache_access() {
        let mut engine = ContextEngine::new("claude-sonnet-4-20250514");
        engine.file_cache().insert(
            Path::new("test.rs"),
            "content".to_string(),
            SystemTime::UNIX_EPOCH,
        );

        assert_eq!(engine.file_cache().len(), 1);
        assert!(engine.file_cache().get(Path::new("test.rs")).is_some());
        assert!(engine.file_cache().get(Path::new("missing.rs")).is_none());
    }

    #[test]
    fn test_context_engine_max_context() {
        let claude = ContextEngine::new("claude-sonnet-4-20250514");
        assert_eq!(claude.max_context(), 200_000);

        let gpt = ContextEngine::new("gpt-4o");
        assert_eq!(gpt.max_context(), 128_000);
    }

    #[test]
    fn test_build_messages_with_large_system_prompt() {
        let mut engine = ContextEngine::new("claude-sonnet-4-20250514");
        engine.max_context = 100;

        let messages = vec![catalyst_llm::Message {
            role: catalyst_llm::Role::User,
            content: catalyst_llm::Content::Text("Hello".to_string()),
        }];
        let large_prompt = "x".repeat(50);

        let result = engine.build_messages(&messages, &large_prompt);
        assert!(result.len() <= 1);
    }

    #[test]
    fn test_token_counter_empty_messages() {
        let count = TokenCounter::count_messages(&[]);
        assert_eq!(count, 0);
    }
}
