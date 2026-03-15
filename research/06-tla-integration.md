# TLA+ Integration

## What is TLA+?

TLA+ (Temporal Logic of Actions) is a formal specification language for modeling and verifying systems. Created by Leslie Lamport.

**Key concepts:**
- **Specification** - Mathematical description of system behavior
- **Invariant** - Property that must always hold true
- **Safety** - Nothing bad ever happens
- **Liveness** - Something good eventually happens
- **Model checking** - Exhaustively verify all possible states

## Why TLA+ for Catalyst?

| Benefit | Description |
|---------|-------------|
| **Prevent bugs** | Find bugs before code exists |
| **Prove correctness** | Mathematical proof, not just testing |
| **Document design** | Specs serve as precise documentation |
| **Challenge assumptions** | Surface hidden assumptions |

## Integration Strategy

### Phase 1: TLA+ Thinking (MVP+)

Catalyst reasons in TLA+ style without generating specs:
- Identify invariants before coding
- Consider edge cases systematically
- State preconditions/postconditions

### Phase 2: Spec Generation (Future)

Generate TLA+ specs for critical modules:
- Model state transitions
- Define invariants
- User runs TLC model checker

### Phase 3: Full Integration (Future)

Automated workflow:
1. Catalyst generates spec
2. Runs TLC automatically
3. Interprets results
4. Iterates until spec is correct
5. Generates code from verified spec

## TLA+ Basics

### PlusCal

PlusCal is an algorithm language that compiles to TLA+. Easier to write.

```tla
---- MODULE Example ----
EXTENDS Integers, Sequences

(* --algorithm Example
variables
    counter = 0;
    max = 10;

begin
Loop:
    while counter < max do
        counter := counter + 1;
    end while;
end algorithm; *)
====
```

### Generated TLA+

```tla
---- MODULE Example ----
EXTENDS Integers, Sequences

VARIABLES counter, pc

Init == 
    /\ counter = 0
    /\ pc = "Loop"

Loop == 
    /\ pc = "Loop"
    /\ IF counter < 10 
       THEN /\ counter' = counter + 1
            /\ pc' = "Loop"
       ELSE /\ pc' = "Done"

Next == Loop \/ (* Termination *)

Spec == Init /\ [][Next]_<<counter, pc>>
====
```

### Invariants

```tla
TypeInvariant == 
    /\ counter \in Int
    /\ counter >= 0
    /\ counter <= 10

CounterBounded == counter <= max
```

## Catalyst Workflow

### Example: Authentication Module

**User request:** "Implement user authentication"

**Catalyst response:**

```
Before implementing, let me specify the authentication system:

## States
- Unauthenticated
- Authenticating
- Authenticated
- Failed

## Variables
- session_token: Null | Token
- attempt_count: Nat
- locked_until: Null | Time

## Invariants
1. TokenValidity: session_token ≠ Null ⇒ token_is_valid(session_token)
2. AttemptLimit: attempt_count ≤ MAX_ATTEMPTS
3. LockEnforced: locked_until ≠ Null ∧ time < locked_until ⇒ session_token = Null

## Safety Properties
- Never authenticated with invalid credentials
- Session tokens are cryptographically random
- Account lockout prevents brute force

## Liveness Properties
- Valid credentials eventually authenticate
- Lockout eventually expires

Would you like me to generate the TLA+ spec for formal verification?
```

## Rust Verification Tools

### Prusti

Prusti verifies Rust code using Viper verification framework.

```rust
// Prusti annotations
#[requires(x >= 0)]
#[ensures(result >= 0)]
fn abs(x: i32) -> i32 {
    if x < 0 { -x } else { x }
}
```

### Kani

Kani is a model checker for Rust (developed by AWS).

```rust
// Kani proof
#[kani::proof]
fn verify_abs() {
    let x: i32 = kani::any();
    kani::assume(x >= i32::MIN + 1); // Avoid overflow
    let result = abs(x);
    assert!(result >= 0);
}
```

## Implementation Plan

### Phase 1: TLA+ Thinking

```rust
pub struct TlaAnalyzer {
    // Reasoning engine
}

impl TlaAnalyzer {
    /// Analyze a change request and identify:
    /// - State variables involved
    /// - Invariants that must hold
    /// - Edge cases to consider
    pub fn analyze_change(&self, request: &str, context: &CodeContext) -> TlaAnalysis {
        TlaAnalysis {
            states: self.identify_states(request, context),
            invariants: self.identify_invariants(request, context),
            edge_cases: self.identify_edge_cases(request, context),
            safety_properties: self.identify_safety(request, context),
        }
    }
    
    /// Generate prompts that enforce TLA+ thinking
    pub fn augment_system_prompt(&self) -> String {
        r#"
        Before making any code change, consider:
        
        1. **State Variables**: What state is affected?
        2. **Invariants**: What must always be true?
        3. **Transitions**: How does state change?
        4. **Preconditions**: What must be true before?
        5. **Postconditions**: What must be true after?
        6. **Edge Cases**: What could go wrong?
        
        Document these in your response before showing code.
        "#.to_string()
    }
}

pub struct TlaAnalysis {
    pub states: Vec<StateVariable>,
    pub invariants: Vec<Invariant>,
    pub edge_cases: Vec<EdgeCase>,
    pub safety_properties: Vec<SafetyProperty>,
}
```

### Phase 2: Spec Generation

```rust
pub struct TlaGenerator;

impl TlaGenerator {
    /// Generate PlusCal specification from Rust code or description
    pub fn generate_pluscal(&self, analysis: &TlaAnalysis) -> String {
        // Generate PlusCal algorithm
    }
    
    /// Generate TLA+ specification
    pub fn generate_tla(&self, pluscal: &str) -> String {
        // Compile PlusCal to TLA+
    }
    
    /// Generate invariants to check
    pub fn generate_invariants(&self, analysis: &TlaAnalysis) -> Vec<String> {
        // Generate invariant definitions
    }
}
```

### Phase 3: TLC Integration

```rust
pub struct TlcRunner;

impl TlcRunner {
    /// Run TLC model checker
    pub async fn check(&self, spec: &TlaSpec) -> TlcResult {
        // Execute: java -jar tla2tools.jar spec.tla
        // Parse output
        // Return result
    }
}

pub enum TlcResult {
    Pass,
    Fail { 
        error: String,
        counterexample: Option<Trace>,
    },
    Error(String),
}
```

## Example Session

```
User: Implement a rate limiter

Catalyst: Let me first specify the rate limiter formally.

## TLA+ Analysis

### State Variables
- request_count: Nat (requests in current window)
- window_start: Time
- blocked: Boolean

### Invariants
1. RequestCountBounded: request_count ≤ MAX_REQUESTS
2. WindowMonotonic: window_start never decreases
3. BlockConsistent: blocked ⇔ request_count ≥ MAX_REQUESTS

### Safety Properties
- Never exceeds rate limit
- Window transitions are atomic

### Edge Cases
- Concurrent requests at boundary
- Clock skew
- Window overflow

Generating PlusCal spec...

---- MODULE RateLimiter ----
EXTENDS Integers, Sequences

CONSTANTS MAX_REQUESTS, WINDOW_MS

VARIABLES request_count, window_start, blocked

Init ==
    /\ request_count = 0
    /\ window_start = Now
    /\ blocked = FALSE

Request ==
    /\ ~blocked
    /\ request_count' = request_count + 1
    /\ IF request_count' >= MAX_REQUESTS
       THEN blocked' = TRUE
       ELSE UNCHANGED blocked
    /\ UNCHANGED window_start

WindowExpired ==
    /\ Now - window_start >= WINDOW_MS
    /\ window_start' = Now
    /\ request_count' = 0
    /\ blocked' = FALSE

Next == Request \/ WindowExpired

Spec == Init /\ [][Next]_<<request_count, window_start, blocked>>

====

Would you like me to:
1. Run TLC to verify this spec?
2. Proceed with implementation?
```

## Resources

- [TLA+ Home](https://lamport.azurewebsites.net/tla/tla.html)
- [Learn TLA+](https://learntla.com)
- [PlusCal Manual](https://lamport.azurewebsites.net/tla/pluscal.html)
- [Prusti](https://github.com/viperproject/prusti-dev)
- [Kani](https://github.com/model-checking/kani)
