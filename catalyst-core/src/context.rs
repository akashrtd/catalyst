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
}
