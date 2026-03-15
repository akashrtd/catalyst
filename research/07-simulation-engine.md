# Simulation Engine

## Overview

Catalyst's simulation engine verifies proposed code changes before applying them. Uses a hybrid approach - Catalyst decides between lightweight sandbox and full containerized environment based on change complexity.

## Simulation Types

| Type | Description | Environment |
|------|-------------|-------------|
| **Syntax Check** | Parse code for errors | Lightweight |
| **Type Check** | Verify type correctness | Lightweight |
| **Unit Test** | Run affected tests | Sandbox |
| **Integration Test** | Run integration tests | Sandbox |
| **Behavior Simulation** | Execute code in isolated env | Containerized |
| **Load Test** | Test under load | Containerized |
| **Failure Injection** | Test error handling | Containerized |

## Complexity Detection

Catalyst auto-detects change complexity to choose simulation strategy:

```rust
pub enum Complexity {
    Low,      // Syntax, type checks only
    Medium,   // Sandbox execution
    High,     // Containerized environment
}

pub struct ComplexityAnalyzer;

impl ComplexityAnalyzer {
    pub fn analyze(&self, change: &CodeChange) -> Complexity {
        let score = self.calculate_score(change);
        
        if score < 10 { Complexity::Low }
        else if score < 50 { Complexity::Medium }
        else { Complexity::High }
    }
    
    fn calculate_score(&self, change: &CodeChange) -> u32 {
        let mut score = 0u32;
        
        // Factors
        score += change.lines_changed() * 1;
        score += change.files_changed() * 5;
        
        if change.touches_critical_path() { score += 20; }
        if change.modifies_api() { score += 15; }
        if change.affects_database() { score += 25; }
        if change.has_external_dependencies() { score += 15; }
        if change.modifies_concurrent_code() { score += 20; }
        if change.changes_security_code() { score += 30; }
        
        score
    }
}
```

## Simulation Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                   SIMULATION ENGINE                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Code Change ──► Complexity Analyzer ──► Environment Select │
│         │                                         │         │
│         │         ┌───────────────────────────────┘         │
│         │         │                                           │
│         │         ▼                                           │
│         │    ┌─────────────┐    ┌─────────────────┐          │
│         │    │  LIGHTWEIGHT │    │   CONTAINERIZED │          │
│         │    │  - Syntax    │    │   - Full env    │          │
│         │    │  - Types     │    │   - Network     │          │
│         │    │  - Lint      │    │   - Database    │          │
│         │    └──────┬──────┘    └────────┬────────┘          │
│         │           │                    │                   │
│         │           └────────┬───────────┘                   │
│         │                    ▼                               │
│         │           ┌─────────────────┐                      │
│         │           │  Result Aggregator                     │
│         │           └────────┬────────┘                      │
│         │                    │                               │
│         └────────────────────┘                               │
│                              │                               │
│                              ▼                               │
│                    ┌─────────────────┐                      │
│                    │  Simulation Report                      │
│                    │  - Passed/Failed                        │
│                    │  - Issues Found                         │
│                    │  - Recommendations                     │
│                    └─────────────────┘                      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Lightweight Checks

### Syntax & Type Check

```rust
pub struct LightweightSimulator {
    rustc: RustcAnalyzer,
    cargo: CargoAnalyzer,
}

impl LightweightSimulator {
    pub fn check(&self, change: &CodeChange) -> Result<CheckResult, Error> {
        let mut results = Vec::new();
        
        // Syntax check
        let syntax = self.rustc.parse(&change.new_content)?;
        if !syntax.errors.is_empty() {
            results.push(CheckIssue::SyntaxErrors(syntax.errors));
        }
        
        // Type check (cargo check)
        let types = self.cargo.check(&change.project_root)?;
        if !types.errors.is_empty() {
            results.push(CheckIssue::TypeErrors(types.errors));
        }
        
        // Lint (clippy)
        let lints = self.cargo.clippy(&change.project_root)?;
        if !lints.warnings.is_empty() {
            results.push(CheckIssue::LintWarnings(lints.warnings));
        }
        
        Ok(CheckResult { issues: results })
    }
}
```

## Sandbox Execution

### WASM-based Isolation

```rust
pub struct SandboxSimulator {
    wasmtime: Engine,
}

impl SandboxSimulator {
    /// Execute code in WASM sandbox
    pub async fn execute(&self, code: &str, inputs: Vec<Value>) -> Result<ExecutionResult, Error> {
        // Compile to WASM (for Rust, use wasm32-unknown-unknown)
        let wasm = self.compile_to_wasm(code)?;
        
        // Execute in sandboxed runtime
        let module = Module::new(&self.wasmtime, &wasm)?;
        let instance = Instance::new(&mut store, &module, &imports)?;
        
        // Run with limits
        let result = instance.call("simulate", inputs)?;
        
        Ok(ExecutionResult {
            output: result,
            memory_used: store.memory_used(),
            time_taken: store.time_elapsed(),
        })
    }
}
```

### Test Execution

```rust
pub struct TestSimulator;

impl TestSimulator {
    pub async fn run_tests(&self, change: &CodeChange) -> Result<TestResult, Error> {
        // Find affected tests
        let affected_tests = self.find_affected_tests(change)?;
        
        // Run in isolated process
        let output = Command::new("cargo")
            .args(["test", "--no-run"])
            .current_dir(&change.project_root)
            .output()?;
        
        // Execute tests
        let results = self.execute_tests(affected_tests).await?;
        
        Ok(TestResult {
            passed: results.iter().filter(|r| r.passed).count(),
            failed: results.iter().filter(|r| !r.passed).count(),
            details: results,
        })
    }
    
    fn find_affected_tests(&self, change: &CodeChange) -> Result<Vec<TestPath>, Error> {
        // Use cargo-mutants or similar to find affected tests
        // Analyze dependencies
    }
}
```

## Containerized Environment

### Docker Integration

```rust
pub struct ContainerSimulator {
    docker: Docker,
}

impl ContainerSimulator {
    pub async fn simulate(&self, change: &CodeChange) -> Result<SimulationResult, Error> {
        // Create container from project image
        let container = self.docker.create_container(ContainerConfig {
            image: "catalyst-sim:latest",
            workdir: "/workspace",
            mounts: vec![
                Mount::bind(&change.project_root, "/workspace"),
            ],
            resources: ResourceLimits {
                cpu: 2.0,
                memory: 4 * 1024 * 1024 * 1024, // 4GB
                timeout: Duration::from_secs(300),
            },
        }).await?;
        
        // Apply changes in container
        self.apply_changes(&container, change).await?;
        
        // Run simulation scenarios
        let scenarios = self.determine_scenarios(change);
        let mut results = Vec::new();
        
        for scenario in scenarios {
            let result = self.run_scenario(&container, &scenario).await?;
            results.push(result);
        }
        
        // Cleanup
        self.docker.remove_container(&container).await?;
        
        Ok(SimulationResult { scenarios: results })
    }
    
    async fn run_scenario(&self, container: &Container, scenario: &Scenario) -> Result<ScenarioResult, Error> {
        match scenario {
            Scenario::Build => self.run_build(container).await,
            Scenario::Tests => self.run_tests(container).await,
            Scenario::LoadTest { concurrent_users, duration } => {
                self.run_load_test(container, *concurrent_users, *duration).await
            }
            Scenario::FailureInjection { failures } => {
                self.run_chaos_test(container, failures).await
            }
        }
    }
}
```

### Scenario Types

```rust
pub enum Scenario {
    Build,
    Tests,
    LoadTest {
        concurrent_users: u32,
        duration: Duration,
    },
    FailureInjection {
        failures: Vec<FailureType>,
    },
    NetworkLatency {
        latency: Duration,
        jitter: Duration,
    },
    DatabaseStress {
        connections: u32,
    },
}

pub enum FailureType {
    NetworkDrop,
    DatabaseTimeout,
    DiskFull,
    MemoryExhaustion,
    ProcessKill,
}
```

## Result Aggregation

```rust
pub struct SimulationReport {
    pub overall_status: Status,
    pub checks: Vec<CheckResult>,
    pub tests: Option<TestResult>,
    pub scenarios: Vec<ScenarioResult>,
    pub issues: Vec<Issue>,
    pub recommendations: Vec<Recommendation>,
}

pub enum Status {
    Passed,
    Warnings,
    Failed,
    Error,
}

pub struct Issue {
    pub severity: Severity,
    pub category: Category,
    pub message: String,
    pub location: Option<Location>,
    pub suggestion: Option<String>,
}

pub enum Severity {
    Critical,  // Must fix before applying
    High,      // Should fix
    Medium,    // Consider fixing
    Low,       // Minor issue
    Info,      // Informational
}

pub enum Category {
    Syntax,
    Type,
    Logic,
    Performance,
    Security,
    Compatibility,
    Style,
}
```

## Catalyst Integration

### Before Applying Changes

```rust
impl CatalystAgent {
    async fn apply_change(&mut self, change: CodeChange) -> Result<(), Error> {
        // 1. Analyze complexity
        let complexity = self.complexity_analyzer.analyze(&change);
        
        // 2. Run appropriate simulation
        let report = match complexity {
            Complexity::Low => {
                self.lightweight_sim.check(&change)?
            }
            Complexity::Medium => {
                let mut report = self.lightweight_sim.check(&change)?;
                report.merge(self.sandbox_sim.run_tests(&change).await?);
                report
            }
            Complexity::High => {
                let mut report = self.lightweight_sim.check(&change)?;
                report.merge(self.sandbox_sim.run_tests(&change).await?);
                report.merge(self.container_sim.simulate(&change).await?);
                report
            }
        };
        
        // 3. Present results to user
        if report.overall_status == Status::Failed {
            self.present_issues(&report.issues);
            return Err(Error::SimulationFailed(report));
        }
        
        // 4. Apply change
        self.apply_to_filesystem(&change)?;
        
        Ok(())
    }
}
```

### Example Output

```
Catalyst: Running simulation for proposed changes...

┌─────────────────────────────────────────────────────────────┐
│ SIMULATION REPORT                                           │
├─────────────────────────────────────────────────────────────┤
│ Complexity: HIGH (Score: 67)                                │
│ Environment: Containerized                                  │
├─────────────────────────────────────────────────────────────┤
│ CHECKS                                                      │
│   ✓ Syntax check passed                                     │
│   ✓ Type check passed                                       │
│   ⚠ Lint: 2 warnings                                        │
│                                                             │
│ TESTS                                                       │
│   ✓ 45 passed                                               │
│   ✗ 2 failed                                                │
│     - test_user_authentication                              │
│     - test_session_expiry                                   │
│                                                             │
│ SCENARIOS                                                   │
│   ✓ Build succeeded                                         │
│   ✓ Load test (100 users, 60s): p99 < 200ms                 │
│   ✗ Failure injection: Database timeout caused crash        │
│                                                             │
│ ISSUES                                                      │
│   [CRITICAL] Missing timeout handling in db_query()         │
│   [HIGH] Session token not validated on refresh             │
│   [MEDIUM] Unnecessary clone in hot path                    │
│                                                             │
│ RECOMMENDATIONS                                             │
│   1. Add timeout wrapper around database calls              │
│   2. Validate token before refresh operation                │
│   3. Consider using Cow<str> instead of clone               │
└─────────────────────────────────────────────────────────────┘

Status: FAILED

I cannot apply these changes in their current state. The simulation
found critical issues that need to be addressed:

1. Missing timeout handling - the process crashes when database is slow
2. Session validation gap - security vulnerability

Would you like me to fix these issues first, or would you prefer to
proceed anyway? (Note: Proceeding is not recommended)
```

## Future Enhancements

| Feature | Description |
|---------|-------------|
| **Mutation Testing** | Inject bugs to test test quality |
| **Fuzzing** | Generate random inputs |
| **Property-based Testing** | QuickCheck/proptest integration |
| **Performance Regression** | Compare against baseline |
| **Security Scanning** | Static analysis for vulnerabilities |
