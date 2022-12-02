use std::{cell::RefCell, rc::Rc, pin::Pin, future::Future};



pub struct FutureSelf<'a, 'output, T: 'output> {
    inner: Rc<RefCell<Option<Box<dyn Fn() -> Pin<Box<dyn Future<Output = T> + 'output>> + 'a>>>>,
}

impl<'a, 'output: 'a, T: 'output> FutureSelf<'a, 'output, T> {
    pub fn get_self(&self) -> Pin<Box<dyn Future<Output = T> + 'output>> {
        self.inner.borrow().as_ref().unwrap()()
    }
}

pub fn self_referential_future<'a, 'output: 'a, T: 'output>(
    fut: impl Fn(FutureSelf<'a, 'output, T>) -> Box<dyn Future<Output = T> + 'static> + 'a)
    -> Pin<Box<dyn Future<Output = T> + 'output>>
{
    let fut_ref = Rc::new(RefCell::new(None));

    {
        let inner_fut_ref = fut_ref.clone();

        *fut_ref.clone().borrow_mut() = Some(Box::new(
            move || {
                let fut_ref = inner_fut_ref.clone();
                let future_self = FutureSelf { inner: fut_ref };
                
                Box::into_pin(Box::pin(&fut)(future_self))
            }
        ) as Box<dyn Fn() -> Pin<Box<dyn Future<Output = T> + 'output>> + 'a>);
    }

    let future = fut_ref.borrow().as_ref().unwrap()();

    Box::pin(async move {
        future.await
    })
}
