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

impl<V, F> GpioIn for DummyGpioIn<F>
where
    V: Into<GpioValue>,
    F: Fn() -> V,
{
    type Error = ();

    fn read_value(&mut self) -> Result<GpioValue, Self::Error> {
        Ok((self.value)().into())
    }
}

#[derive(Debug)]
struct DummyGpioOut<F> {
    dest: F,
}

// FIXME: support result

impl<F> DummyGpioOut<F> {
    pub fn new(dest: F) -> DummyGpioOut<F> {
        DummyGpioOut { dest }
    }
}

impl<F: Fn(GpioValue)> GpioOut for DummyGpioOut<F> {
    type Error = ();

    fn set_low(&mut self) -> Result<(), Self::Error> {
        (self.dest)(GpioValue::Low);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        (self.dest)(GpioValue::High);
        Ok(())
    }
}
