use std::sync::Arc;
use std::future::Future;

use tokio::sync::{OnceCell, RwLock};

pub struct GlobalMockable<T>
where
    T: ?Sized + Send + Sync,
{
    instance: RwLock<OnceCell<Arc<T>>>,
}

impl<T> GlobalMockable<T>
where
    T: ?Sized + Send + Sync,
{
    pub const fn const_new() -> Self {
        GlobalMockable {
            instance: RwLock::const_new(OnceCell::const_new()),
        }
    }

    pub async fn get_or_init<F, Fut>(&self, f: F) -> Arc<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Arc<T>> + Send,
    {
        self.instance
            .read()
            .await
            .get_or_init::<F, Fut>(f)
            .await
            .clone()
    }

    pub async fn set(&self, value: Arc<T>) {
        let mut write = self.instance.write().await;
        *write = OnceCell::const_new_with(value);
    }

    pub async fn clear(&self) {
        let mut write = self.instance.write().await;
        *write = OnceCell::const_new();
    }
}

#[macro_export]
macro_rules! define_global_mockable {
    ($struct_name:ident, $trait_ty:ty, $default_impl:path) => {
        pub struct $struct_name;

        impl $struct_name {
            fn static_instance() -> &'static $crate::GlobalMockable<$trait_ty> {
                static STATIC_INSTANCE: $crate::GlobalMockable<$trait_ty> =
                    $crate::GlobalMockable::const_new();

                &STATIC_INSTANCE
            }

            pub async fn get() -> ::std::sync::Arc<$trait_ty> {
                Self::static_instance().get_or_init(Self::default_impl).await
            }

            async fn default_impl() -> ::std::sync::Arc<$trait_ty> {
                $default_impl().await
            }

            pub async fn set(value: ::std::sync::Arc<$trait_ty>) {
                Self::static_instance().set(value).await;
            }

            pub async fn clear() {
                Self::static_instance().clear().await;
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct SimpleStruct {
        value: usize,
    }

    #[tokio::test]
    async fn supports_mocking_concrete_types() {
        let global: GlobalMockable<SimpleStruct> = GlobalMockable::const_new();
        let default_calls = Arc::new(AtomicUsize::new(0));

        let first = global
            .get_or_init({
                let default_calls = Arc::clone(&default_calls);
                || async move {
                    default_calls.fetch_add(1, Ordering::SeqCst);
                    Arc::new(SimpleStruct { value: 1 })
                }
            })
            .await;

        assert_eq!(first.value, 1);

        global.set(Arc::new(SimpleStruct { value: 99 })).await;

        let second = global
            .get_or_init(|| async { Arc::new(SimpleStruct { value: 2 }) })
            .await;

        assert_eq!(second.value, 99);
        assert_eq!(default_calls.load(Ordering::SeqCst), 1);

        global.clear().await;

        let third = global
            .get_or_init({
                let default_calls = Arc::clone(&default_calls);
                || async move {
                    default_calls.fetch_add(1, Ordering::SeqCst);
                    Arc::new(SimpleStruct { value: 7 })
                }
            })
            .await;

        assert_eq!(third.value, 7);
        assert_eq!(default_calls.load(Ordering::SeqCst), 2);
    }

    pub trait Greeter: Send + Sync {
        fn greet(&self) -> &'static str;
    }

    struct DefaultGreeter;

    impl Greeter for DefaultGreeter {
        fn greet(&self) -> &'static str {
            "hello"
        }
    }

    struct MockGreeter;

    impl Greeter for MockGreeter {
        fn greet(&self) -> &'static str {
            "mock hello"
        }
    }

    async fn default_greeter() -> Arc<dyn Greeter> {
        Arc::new(DefaultGreeter)
    }

    define_global_mockable!(TestGreeter, dyn Greeter, default_greeter);

    #[tokio::test]
    async fn swaps_trait_object_implementations() {
        TestGreeter::clear().await;

        let real = TestGreeter::get().await;
        assert_eq!(real.greet(), "hello");

        TestGreeter::set(Arc::new(MockGreeter)).await;

        let mocked = TestGreeter::get().await;
        assert_eq!(mocked.greet(), "mock hello");

        TestGreeter::clear().await;

        let reset = TestGreeter::get().await;
        assert_eq!(reset.greet(), "hello");
    }
}
