use super::{GpioIn, GpioOut, GpioValue};

#[derive(Debug)]
struct DummyGpioIn<F> {
    value: F,
}

impl<F> DummyGpioIn<F> {
    pub fn new(value: F) -> DummyGpioIn<F> {
        DummyGpioIn { value }
    }
}

impl<V, F, E> GpioIn for DummyGpioIn<F>
where
    V: Into<GpioValue>,
    F: Fn() -> Result<V, E>,
{
    type Error = E;

    fn read_value(&mut self) -> Result<GpioValue, Self::Error> {
        (self.value)().map(|v| v.into())
    }
}

#[derive(Debug)]
struct DummyGpioOut<F> {
    dest: F,
}

impl<F> DummyGpioOut<F> {
    pub fn new(dest: F) -> DummyGpioOut<F> {
        DummyGpioOut { dest }
    }
}

impl<F, E> GpioOut for DummyGpioOut<F>
where
    F: Fn(GpioValue) -> Result<(), E>,
{
    type Error = E;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        (self.dest)(GpioValue::Low)
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        (self.dest)(GpioValue::High)
    }
}
