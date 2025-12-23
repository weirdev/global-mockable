```markdown
# Global Mockable

A utility for creating global mockable objects.

## Usage

The crate exposes a small macro to define a global, mockable object for an object. 
This is especially useful for dyn Trait objects. 

```rust
use std::sync::Arc;

/// Define a trait and two implementations (real + mock).
pub trait Greeter: Send + Sync {
    fn greet(&self) -> &'static str;
}

struct DefaultGreeter;
impl Greeter for DefaultGreeter {
    fn greet(&self) -> &'static str { "hello" }
}

struct MockGreeter;
impl Greeter for MockGreeter {
    fn greet(&self) -> &'static str { "mock hello" }
}

/// Async function that returns the default implementation.
async fn default_greeter() -> Arc<dyn Greeter> {
    Arc::new(DefaultGreeter)
}

// Create the global mockable type `TestGreeter` for `dyn Greeter`.
global_mockable::define_global_mockable!(TestGreeter, dyn Greeter, default_greeter);

#[tokio::main]
async fn main() {
    // Get the current (default) implementation.
    let real = TestGreeter::get().await;
    assert_eq!(real.greet(), "hello");

    // Swap in a mock implementation for testing.
    TestGreeter::set(Arc::new(MockGreeter)).await;

    let mocked = TestGreeter::get().await;
    assert_eq!(mocked.greet(), "mock hello");

    // Clear the override to return to the default implementation.
    TestGreeter::clear().await;

    let reset = TestGreeter::get().await;
    assert_eq!(reset.greet(), "hello");
}
```
