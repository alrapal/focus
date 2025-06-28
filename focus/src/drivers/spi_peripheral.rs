use core::{cell::RefCell, fmt::Debug};
use critical_section::Mutex;
use embedded_hal::{
    digital::OutputPin,
    spi::{Error, ErrorKind, ErrorType, Operation, SpiBus, SpiDevice},
};

#[allow(dead_code)]
#[derive(Debug)]
pub enum SpiPeripheralError<E> {
    SpiBus(E),  // Errors wrapper from the SpiBus
    Lock,       // Error when attempting to lock the bus
    ChipSelect, // Error when interacting with the chip select gpio
}

// Allow to map the custom error types to error compatible with the SpiDevice trait
impl<E> Error for SpiPeripheralError<E>
where
    E: Error + Debug,
{
    #[inline]
    fn kind(&self) -> ErrorKind {
        match self {
            SpiPeripheralError::SpiBus(e) => e.kind(), // Fwd SpiBus error by converting them into ErroKind
            SpiPeripheralError::Lock => ErrorKind::Other,
            SpiPeripheralError::ChipSelect => ErrorKind::ChipSelectFault,
        }
    }
}

#[allow(dead_code)]
pub struct SpiPeripheral<'a, S, E, P>
where
    S: SpiBus<u8, Error = E>,
    E: Error,
    P: OutputPin,
{
    // spi_bus: &'a mut S,
    mutex_bus: &'a Mutex<RefCell<Option<S>>>,
    ns_per_tick: u32,
    cs: P,
}

// ErrorType trait implementation for the SpiDeviceWrapper.
// This binds the custom error type to the wrapper, and since
// the type implements Error, it can be used as type Error.
impl<S, E, P> ErrorType for SpiPeripheral<'_, S, E, P>
where
    S: SpiBus<u8, Error = E>,
    E: Error,
    P: OutputPin,
{
    type Error = SpiPeripheralError<E>;
}

// Wrapper specific implementation
impl<'a, S, E, P> SpiPeripheral<'a, S, E, P>
where
    S: SpiBus<u8, Error = E>,
    E: Error,
    P: OutputPin,
{
    pub fn new(mutex_bus: &'a Mutex<RefCell<Option<S>>>, cs: P, ns_per_tick: u32) -> Self {
        SpiPeripheral {
            mutex_bus,
            ns_per_tick,
            cs,
        }
    }

    #[allow(dead_code)]
    #[inline]
    fn assert_cs(&mut self) -> Result<(), SpiPeripheralError<E>> {
        self.cs
            .set_low()
            .map_err(|_| SpiPeripheralError::ChipSelect)
    }

    #[allow(dead_code)]
    #[inline]
    fn deassert_cs(&mut self) -> Result<(), SpiPeripheralError<E>> {
        self.cs
            .set_high()
            .map_err(|_| SpiPeripheralError::ChipSelect)
    }
}

// SpiDevice trait implementation
impl<S, E, P> SpiDevice for SpiPeripheral<'_, S, E, P>
where
    S: SpiBus<u8, Error = E>,
    E: Error,
    P: OutputPin,
{
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        // Locks the bus
        let res = critical_section::with(|cs| -> Result<(), Self::Error> {
            let spi_ref = &mut *self.mutex_bus.borrow_ref_mut(cs);
            let spi_bus = spi_ref.as_mut().ok_or(SpiPeripheralError::Lock)?;

            self.assert_cs()?;
            for operation in operations {
                match operation {
                    Operation::Read(words) => {
                        spi_bus.read(words).map_err(SpiPeripheralError::SpiBus)?;
                    }
                    Operation::Write(words) => {
                        spi_bus.write(words).map_err(SpiPeripheralError::SpiBus)?;
                    }
                    Operation::Transfer(in_buff, out_buff) => {
                        spi_bus
                            .transfer(in_buff, out_buff)
                            .map_err(SpiPeripheralError::SpiBus)?;
                    }
                    Operation::TransferInPlace(words) => {
                        spi_bus
                            .transfer_in_place(words)
                            .map_err(SpiPeripheralError::SpiBus)?;
                    }
                    Operation::DelayNs(delay_ns) => {
                        let mut delay_in_ticks = *delay_ns / self.ns_per_tick;
                        while delay_in_ticks > 0 {
                            delay_in_ticks -= 1;
                        }
                    }
                }
            }
            // Asserts the CS (Chip Select) pin.
            // Performs all the operations.
            // Flushes the bus.
            spi_bus.flush().map_err(SpiPeripheralError::SpiBus)?;
            // Deasserts the CS pin.
            self.deassert_cs()?;
            Ok(())
        });
        // TODO

        // Unlocks the bus.
        res
    }
}
